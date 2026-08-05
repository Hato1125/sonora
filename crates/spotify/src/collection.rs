use std::collections::HashMap;
use std::time::Duration;

use anyhow::{Context as _, Result};
use librespot_core::Session;
use librespot_protocol::extended_metadata::{BatchedEntityRequest, EntityRequest, ExtensionQuery};
use librespot_protocol::extension_kind::ExtensionKind;
use librespot_protocol::metadata::image::Size as ImageSize;
use librespot_protocol::metadata::{Album as AlbumMessage, Track as TrackMessage};
use protobuf::{EnumOrUnknown, Message as _};

use crate::models::Track;
use crate::wire;

const LIKED_SONGS: &str = "spotify:collection:tracks";
const TRACK_PREFIX: &str = "spotify:track:";
const UNKNOWN: &str = "Unknown";

pub async fn saved_tracks(session: &Session, limit: u32) -> Result<Vec<Track>> {
    let uris = liked_uris(session, limit as usize).await?;
    if uris.is_empty() {
        return Ok(Vec::new());
    }

    let mut known = metadata(session, &uris).await?;
    Ok(uris.iter().filter_map(|uri| known.remove(uri)).collect())
}

async fn liked_uris(session: &Session, limit: usize) -> Result<Vec<String>> {
    let context = session
        .spclient()
        .get_context(LIKED_SONGS)
        .await
        .context("cannot resolve the liked songs context")?;

    Ok(context
        .pages
        .iter()
        .flat_map(|page| page.tracks.iter())
        .filter_map(|track| track.uri.as_deref())
        .filter(|uri| uri.starts_with(TRACK_PREFIX))
        .take(limit)
        .map(str::to_owned)
        .collect())
}

pub(crate) async fn metadata(session: &Session, uris: &[String]) -> Result<HashMap<String, Track>> {
    let request = BatchedEntityRequest {
        entity_request: uris
            .iter()
            .map(|uri| EntityRequest {
                entity_uri: uri.clone(),
                query: vec![ExtensionQuery {
                    extension_kind: EnumOrUnknown::new(ExtensionKind::TRACK_V4),
                    ..Default::default()
                }],
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    };

    let response = session
        .spclient()
        .get_extended_metadata(request)
        .await
        .context("cannot read track metadata")?;

    let mut tracks = HashMap::new();
    for array in response.extended_metadata {
        for entity in array.extension_data {
            let Ok(message) = TrackMessage::parse_from_bytes(&entity.extension_data.value) else {
                continue;
            };
            let track = track_from(&entity.entity_uri, &message);
            tracks.insert(entity.entity_uri, track);
        }
    }
    Ok(tracks)
}

fn track_from(uri: &str, track: &TrackMessage) -> Track {
    let artists = track
        .artist
        .iter()
        .filter_map(|artist| non_empty(artist.name.as_deref()))
        .collect::<Vec<_>>()
        .join(", ");

    Track {
        id: uri.strip_prefix(TRACK_PREFIX).map(str::to_owned),
        playable: !track.file.is_empty() || !track.alternative.is_empty(),
        name: non_empty(track.name.as_deref())
            .unwrap_or(UNKNOWN)
            .to_owned(),
        artists: if artists.is_empty() {
            UNKNOWN.to_owned()
        } else {
            artists
        },
        album: track
            .album
            .as_ref()
            .and_then(|album| non_empty(album.name.as_deref()))
            .unwrap_or_default()
            .to_owned(),
        cover: track.album.as_ref().and_then(cover_url),
        duration: Duration::from_millis(track.duration.unwrap_or_default().max(0) as u64),
    }
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.filter(|value| !value.is_empty())
}

fn cover_url(album: &AlbumMessage) -> Option<String> {
    let smallest = album
        .cover_group
        .as_ref()?
        .image
        .iter()
        .filter(|image| image.has_file_id())
        .min_by_key(|image| match image.size() {
            ImageSize::SMALL => 0,
            ImageSize::DEFAULT => 1,
            ImageSize::LARGE => 2,
            ImageSize::XLARGE => 3,
        })?;

    wire::image_url(smallest.file_id())
}
