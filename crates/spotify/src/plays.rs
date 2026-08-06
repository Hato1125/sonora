use anyhow::{Context as _, Result, anyhow, bail};
use bytes::Bytes;
use http::{Method, Request, header};
use librespot_core::{Session, spclient::CLIENT_TOKEN};
use serde::Deserialize;

const ENDPOINT: &str = "https://api-partner.spotify.com/pathfinder/v2/query";
const HASH: &str = "612585ae06ba435ad26369870deaae23b5c8800a256cd8a57e08eddc25a37294";

#[derive(Deserialize)]
struct Response {
    data: Option<Data>,
    #[serde(default)]
    errors: Vec<GraphqlError>,
}

#[derive(Deserialize)]
struct Data {
    #[serde(rename = "trackUnion")]
    track: Option<Track>,
}

#[derive(Deserialize)]
struct Track {
    playcount: Option<String>,
}

#[derive(Deserialize)]
struct GraphqlError {
    message: String,
}

pub async fn track(session: &Session, track_id: &str) -> Result<Option<u64>> {
    let variables = serde_json::json!({ "uri": format!("spotify:track:{track_id}") });
    let extensions = serde_json::json!({
        "persistedQuery": {
            "version": 1,
            "sha256Hash": HASH,
        }
    });
    let body = serde_json::to_vec(&serde_json::json!({
        "operationName": "getTrack",
        "variables": variables,
        "extensions": extensions,
    }))
    .context("cannot encode track play count request")?;

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
        .context("cannot build track play count request")?;
    let body = session
        .http_client()
        .request_body(request)
        .await
        .context("cannot request track play count")?;
    decoded(&body)
}

fn decoded(bytes: &[u8]) -> Result<Option<u64>> {
    let response: Response =
        serde_json::from_slice(bytes).context("cannot decode track play count response")?;
    if !response.errors.is_empty() {
        let messages = response
            .errors
            .into_iter()
            .map(|error| error.message)
            .collect::<Vec<_>>()
            .join("; ");
        bail!("Spotify rejected track play count query: {messages}");
    }
    let Some(track) = response.data.and_then(|data| data.track) else {
        return Err(anyhow!("track play count response has no track"));
    };
    track
        .playcount
        .map(|count| count.parse().context("invalid track play count"))
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_playcount() {
        let body = br#"{"data":{"trackUnion":{"playcount":"1234567"}}}"#;
        assert_eq!(decoded(body).unwrap(), Some(1_234_567));
    }

    #[test]
    fn reports_graphql_error() {
        let body = br#"{"data":null,"errors":[{"message":"bad hash"}]}"#;
        assert!(decoded(body).unwrap_err().to_string().contains("bad hash"));
    }
}
