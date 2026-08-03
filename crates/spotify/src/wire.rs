use std::time::Duration;

use librespot_protocol::playlist4_external::SelectedListContent as RootList;
use serde::Deserialize;

use crate::models;

const UNKNOWN: &str = "Unknown";

#[derive(Debug, Deserialize)]
pub struct Page<T> {
    pub items: Option<Vec<Option<T>>>,
}

impl<T> Page<T> {
    pub fn present(self) -> impl Iterator<Item = T> {
        self.items.unwrap_or_default().into_iter().flatten()
    }
}

#[derive(Debug, Deserialize)]
pub struct SavedTrack {
    pub track: Option<Track>,
}

#[derive(Debug, Deserialize)]
pub struct Track {
    pub id: Option<String>,
    pub name: Option<String>,
    pub artists: Option<Vec<Named>>,
    pub album: Option<Named>,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
pub struct Named {
    pub id: Option<String>,
    pub display_name: Option<String>,
    pub name: Option<String>,
}

impl Named {
    pub fn label(&self) -> Option<&str> {
        self.display_name
            .as_deref()
            .or(self.name.as_deref())
            .filter(|label| !label.is_empty())
    }
}

impl From<Track> for models::Track {
    fn from(track: Track) -> Self {
        let artists = track
            .artists
            .unwrap_or_default()
            .iter()
            .filter_map(Named::label)
            .collect::<Vec<_>>()
            .join(", ");

        Self {
            id: track.id,
            name: track.name.unwrap_or_else(|| UNKNOWN.to_owned()),
            artists: if artists.is_empty() {
                UNKNOWN.to_owned()
            } else {
                artists
            },
            album: track
                .album
                .as_ref()
                .and_then(Named::label)
                .unwrap_or_default()
                .to_owned(),
            duration: Duration::from_millis(track.duration_ms.unwrap_or_default()),
        }
    }
}

pub fn playlists_from(rootlist: &RootList) -> Vec<models::Playlist> {
    let contents = &rootlist.contents;
    let meta = &contents.meta_items;

    contents
        .items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| {
            let id = item.uri().strip_prefix("spotify:playlist:")?;
            let meta = meta.get(index);

            let name = meta
                .map(|meta| meta.attributes.name())
                .filter(|name| !name.is_empty())
                .unwrap_or(UNKNOWN);
            let owner = meta
                .map(|meta| meta.owner_username())
                .filter(|owner| !owner.is_empty())
                .unwrap_or(UNKNOWN);

            Some(models::Playlist {
                id: id.to_owned(),
                name: name.to_owned(),
                owner: owner.to_owned(),
                track_count: meta.map(|meta| meta.length()).unwrap_or_default().max(0) as u32,
            })
        })
        .collect()
}
