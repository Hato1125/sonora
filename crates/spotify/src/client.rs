use anyhow::{Context as _, Result};
use async_trait::async_trait;
use librespot_core::Session;
use librespot_protocol::playlist4_external::SelectedListContent as RootList;
use protobuf::Message as _;

use crate::models::{Playlist, Track, UserProfile};
use crate::{collection, wire};

#[async_trait]
pub trait SpotifyApi: Send + Sync + 'static {
    async fn profile(&self) -> Result<UserProfile>;
    async fn saved_tracks(&self, limit: u32) -> Result<Vec<Track>>;
    async fn playlists(&self, limit: u32) -> Result<Vec<Playlist>>;
}

pub struct LibrespotClient {
    session: Session,
}

impl LibrespotClient {
    pub fn new(session: Session) -> Self {
        Self { session }
    }

    pub fn session(&self) -> &Session {
        &self.session
    }
}

#[async_trait]
impl SpotifyApi for LibrespotClient {
    async fn profile(&self) -> Result<UserProfile> {
        let username = self.session.username();
        let body = self
            .session
            .spclient()
            .get_user_profile(&username, None, None)
            .await?;

        let profile: wire::Named = serde_json::from_slice(&body).unwrap_or_default();
        Ok(UserProfile {
            display_name: profile.label().unwrap_or(&username).to_owned(),
            id: username,
        })
    }

    async fn saved_tracks(&self, limit: u32) -> Result<Vec<Track>> {
        collection::saved_tracks(&self.session, limit).await
    }

    async fn playlists(&self, limit: u32) -> Result<Vec<Playlist>> {
        let body = self
            .session
            .spclient()
            .get_rootlist(0, Some(limit as usize))
            .await?;

        let rootlist =
            RootList::parse_from_bytes(&body).context("cannot decode the rootlist protobuf")?;
        Ok(wire::playlists_from(&rootlist))
    }
}
