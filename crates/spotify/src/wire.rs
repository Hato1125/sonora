use librespot_protocol::playlist4_external::SelectedListContent as RootList;
use serde::Deserialize;

use crate::models;

pub const UNKNOWN: &str = "Unknown";

#[derive(Debug, Default, Deserialize)]
pub struct Named {
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
