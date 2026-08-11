// SPDX-License-Identifier: GPL-3.0-or-later

use std::fmt::Write as _;

use librespot_protocol::playlist4_external::{ListAttributes, SelectedListContent as RootList};
use serde::Deserialize;

use crate::models;

pub const UNKNOWN: &str = "Unknown";
const IMAGE_CDN: &str = "https://i.scdn.co/image/";
const BY_SIZE: [&str; 4] = ["xlarge", "large", "default", "small"];

pub fn image_url(file_id: &[u8]) -> Option<String> {
    if file_id.is_empty() {
        return None;
    }

    let hex = file_id.iter().fold(String::new(), |mut hex, byte| {
        let _ = write!(hex, "{byte:02x}");
        hex
    });
    Some(format!("{IMAGE_CDN}{hex}"))
}

fn playlist_cover(attributes: &ListAttributes) -> Option<String> {
    for target in BY_SIZE {
        if let Some(size) = attributes
            .picture_size
            .iter()
            .find(|size| size.target_name() == target)
            .filter(|size| !size.url().is_empty())
        {
            return Some(size.url().to_owned());
        }
    }

    attributes
        .picture_size
        .first()
        .map(|size| size.url())
        .filter(|url| !url.is_empty())
        .map(str::to_owned)
        .or_else(|| image_url(attributes.picture()))
}

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

pub fn playlist_from(id: &str, content: &RootList, username: &str) -> models::Playlist {
    let owner = match content.owner_username() {
        "" => UNKNOWN,
        owner => owner,
    };
    let name = match content.attributes.name() {
        "" => UNKNOWN,
        name => name,
    };

    models::Playlist {
        id: id.to_owned(),
        name: name.to_owned(),
        owner: owner.to_owned(),
        owned: owner == username,
        collaborative: content.attributes.collaborative(),
        public: false,
        cover: playlist_cover(&content.attributes),
        track_count: content.length().max(0) as u32,
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
                owned: false,
                collaborative: meta.is_some_and(|meta| meta.attributes.collaborative()),
                public: item.attributes.public(),
                cover: meta.and_then(|meta| playlist_cover(&meta.attributes)),
                track_count: meta.map(|meta| meta.length()).unwrap_or_default().max(0) as u32,
            })
        })
        .collect()
}
