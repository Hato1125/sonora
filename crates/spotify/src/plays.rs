use std::collections::HashMap;

use anyhow::{Context as _, Result, anyhow, bail};
use bytes::Bytes;
use http::{Method, Request, header};
use librespot_core::{Session, spclient::CLIENT_TOKEN};
use serde::Deserialize;

const ENDPOINT: &str = "https://api-partner.spotify.com/pathfinder/v2/query";
const TRACK_HASH: &str = "612585ae06ba435ad26369870deaae23b5c8800a256cd8a57e08eddc25a37294";
const ALBUM_HASH: &str = "b9bfabef66ed756e5e13f68a942deb60bd4125ec1f1be8cc42769dc0259b4b10";
const PAGE_LIMIT: usize = 50;
const TRACK_PREFIX: &str = "spotify:track:";

#[derive(Deserialize)]
struct TrackResponse {
    data: Option<TrackData>,
    #[serde(default)]
    errors: Vec<GraphqlError>,
}

#[derive(Deserialize)]
struct TrackData {
    #[serde(rename = "trackUnion")]
    track: Option<Track>,
}

#[derive(Deserialize)]
struct Track {
    playcount: Option<String>,
}

#[derive(Deserialize)]
struct AlbumResponse {
    data: Option<AlbumData>,
    #[serde(default)]
    errors: Vec<GraphqlError>,
}

#[derive(Deserialize)]
struct AlbumData {
    #[serde(rename = "albumUnion")]
    album: Option<Album>,
}

#[derive(Deserialize)]
struct Album {
    #[serde(rename = "tracksV2")]
    tracks: AlbumTracks,
}

#[derive(Deserialize)]
struct AlbumTracks {
    items: Vec<AlbumItem>,
    #[serde(rename = "totalCount")]
    total_count: usize,
}

#[derive(Deserialize)]
struct AlbumItem {
    track: AlbumTrack,
}

#[derive(Deserialize)]
struct AlbumTrack {
    uri: String,
    playcount: Option<String>,
}

#[derive(Deserialize)]
struct GraphqlError {
    message: String,
}

struct AlbumPage {
    plays: HashMap<String, u64>,
    items: usize,
    total: usize,
}

pub async fn track(session: &Session, track_id: &str) -> Result<Option<u64>> {
    let variables = serde_json::json!({ "uri": format!("spotify:track:{track_id}") });
    let body = body("getTrack", variables, TRACK_HASH, "track")?;
    let body = request(session, body, "track").await?;
    decoded_track(&body)
}

pub async fn album(session: &Session, album_id: &str) -> Result<HashMap<String, u64>> {
    let mut plays = HashMap::new();
    let mut offset = 0;

    loop {
        let variables = serde_json::json!({
            "uri": format!("spotify:album:{album_id}"),
            "locale": "",
            "offset": offset,
            "limit": PAGE_LIMIT,
        });
        let body = body("getAlbum", variables, ALBUM_HASH, "album")?;
        let body = request(session, body, "album").await?;
        let page = decoded_album(&body)?;
        plays.extend(page.plays);

        let Some(next) = next_offset(offset, page.items, page.total) else {
            return Ok(plays);
        };
        offset = next;
    }
}

fn body(
    operation: &str,
    variables: serde_json::Value,
    hash: &str,
    subject: &str,
) -> Result<Vec<u8>> {
    let extensions = serde_json::json!({
        "persistedQuery": {
            "version": 1,
            "sha256Hash": hash,
        }
    });
    serde_json::to_vec(&serde_json::json!({
        "operationName": operation,
        "variables": variables,
        "extensions": extensions,
    }))
    .with_context(|| format!("cannot encode {subject} play count request"))
}

async fn request(session: &Session, body: Vec<u8>, subject: &str) -> Result<Bytes> {
    let token = session
        .login5()
        .auth_token()
        .await
        .context("cannot obtain Spotify access token")?;
    let client_token = session
        .spclient()
        .client_token()
        .await
        .context("cannot obtain Spotify client token")?;
    let request = Request::builder()
        .method(Method::POST)
        .uri(ENDPOINT)
        .header(header::ACCEPT, "application/json")
        .header(header::CONTENT_TYPE, "application/json")
        .header(
            header::AUTHORIZATION,
            format!("{} {}", token.token_type, token.access_token),
        )
        .header(CLIENT_TOKEN, client_token)
        .body(Bytes::from(body))
        .with_context(|| format!("cannot build {subject} play count request"))?;
    session
        .http_client()
        .request_body(request)
        .await
        .with_context(|| format!("cannot request {subject} play count"))
}

fn decoded_track(bytes: &[u8]) -> Result<Option<u64>> {
    let response: TrackResponse =
        serde_json::from_slice(bytes).context("cannot decode track play count response")?;
    rejected(response.errors, "track")?;
    let Some(track) = response.data.and_then(|data| data.track) else {
        return Err(anyhow!("track play count response has no track"));
    };
    track
        .playcount
        .map(|count| count.parse().context("invalid track play count"))
        .transpose()
}

fn decoded_album(bytes: &[u8]) -> Result<AlbumPage> {
    let response: AlbumResponse =
        serde_json::from_slice(bytes).context("cannot decode album play count response")?;
    rejected(response.errors, "album")?;
    let Some(album) = response.data.and_then(|data| data.album) else {
        return Err(anyhow!("album play count response has no album"));
    };
    let items = album.tracks.items.len();
    let total = album.tracks.total_count;
    let plays = map_items(album.tracks.items)?;
    Ok(AlbumPage {
        plays,
        items,
        total,
    })
}

fn map_items(items: Vec<AlbumItem>) -> Result<HashMap<String, u64>> {
    items
        .into_iter()
        .filter_map(|item| {
            let id = item.track.uri.strip_prefix(TRACK_PREFIX)?.to_owned();
            Some(item.track.playcount.map(|count| (id, count)))
        })
        .flatten()
        .map(|(id, count)| {
            count
                .parse()
                .with_context(|| format!("invalid album play count for track {id}"))
                .map(|count| (id, count))
        })
        .collect()
}

fn next_offset(offset: usize, items: usize, total: usize) -> Option<usize> {
    let loaded = offset.saturating_add(items);
    (items > 0 && loaded < total).then_some(loaded)
}

fn rejected(errors: Vec<GraphqlError>, subject: &str) -> Result<()> {
    if errors.is_empty() {
        return Ok(());
    }
    let messages = errors
        .into_iter()
        .map(|error| error.message)
        .collect::<Vec<_>>()
        .join("; ");
    bail!("Spotify rejected {subject} play count query: {messages}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_track_playcount() {
        let body = br#"{"data":{"trackUnion":{"playcount":"1234567"}}}"#;
        assert_eq!(decoded_track(body).unwrap(), Some(1_234_567));
    }

    #[test]
    fn decodes_album_page() {
        let body = br#"{"data":{"albumUnion":{"tracksV2":{"items":[{"track":{"uri":"spotify:track:abc123","playcount":"462537503"}},{"track":{"uri":"spotify:track:def456","playcount":null}}],"totalCount":3}}}}"#;
        let page = decoded_album(body).unwrap();
        assert_eq!(page.items, 2);
        assert_eq!(page.total, 3);
        assert_eq!(page.plays.get("abc123"), Some(&462_537_503));
        assert!(!page.plays.contains_key("def456"));
    }

    #[test]
    fn maps_track_uri_to_playcount() {
        let items = vec![
            AlbumItem {
                track: AlbumTrack {
                    uri: "spotify:track:base62id".to_owned(),
                    playcount: Some("1234".to_owned()),
                },
            },
            AlbumItem {
                track: AlbumTrack {
                    uri: "spotify:episode:ignored".to_owned(),
                    playcount: Some("99".to_owned()),
                },
            },
        ];
        assert_eq!(
            map_items(items).unwrap(),
            HashMap::from([("base62id".to_owned(), 1_234)])
        );
    }

    #[test]
    fn advances_page_offset() {
        assert_eq!(next_offset(0, 50, 101), Some(50));
        assert_eq!(next_offset(50, 50, 101), Some(100));
        assert_eq!(next_offset(100, 1, 101), None);
        assert_eq!(next_offset(50, 0, 101), None);
    }

    #[test]
    fn reports_graphql_error() {
        let body = br#"{"data":null,"errors":[{"message":"bad hash"}]}"#;
        assert!(
            decoded_track(body)
                .unwrap_err()
                .to_string()
                .contains("bad hash")
        );
    }
}
