use anyhow::{Context as _, Result};
use async_trait::async_trait;
use librespot_core::Session;
use librespot_protocol::playlist4_external::SelectedListContent as RootList;
use protobuf::Message as _;

use crate::models::{Album, Playlist, Track, UserProfile};
use crate::{albums, collection, profiles, wire};

#[async_trait]
pub trait SpotifyApi: Send + Sync + 'static {
    async fn profile(&self) -> Result<UserProfile>;
    async fn saved_tracks(&self, limit: u32) -> Result<Vec<Track>>;
    async fn playlists(&self, limit: u32) -> Result<Vec<Playlist>>;
    async fn saved_albums(&self, limit: u32) -> Result<Vec<Album>>;
    async fn album_tracks(&self, album_id: &str) -> Result<Vec<Track>>;
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

    async fn saved_albums(&self, limit: u32) -> Result<Vec<Album>> {
        albums::saved_albums(&self.session, limit).await
    }

    async fn album_tracks(&self, album_id: &str) -> Result<Vec<Track>> {
        albums::album_tracks(&self.session, album_id).await
    }

    async fn playlists(&self, limit: u32) -> Result<Vec<Playlist>> {
        let body = self
            .session
            .spclient()
            .get_rootlist(0, Some(limit as usize))
            .await?;

        let rootlist =
            RootList::parse_from_bytes(&body).context("cannot decode the rootlist protobuf")?;
        let mut playlists = wire::playlists_from(&rootlist);

        let owners = playlists
            .iter()
            .map(|playlist| playlist.owner.clone())
            .filter(|owner| owner != wire::UNKNOWN)
            .collect();
        let names = profiles::display_names(&self.session, owners).await;

        for playlist in &mut playlists {
            if let Some(name) = names.get(&playlist.owner) {
                playlist.owner = name.clone();
            }
        }

        Ok(playlists)
    }
}
