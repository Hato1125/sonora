// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::{HashMap, HashSet};

use crate::{MediaKind, MusicApi};
use anyhow::{Context as _, Result};
use async_trait::async_trait;
use librespot_core::Session;
use librespot_protocol::playlist4_external::SelectedListContent as RootList;
use protobuf::Message as _;

use crate::spotify::{
    albums, artists, collection, collection2, pathfinder, playlists, profiles, radio, search, wire,
};
use crate::{
    Album, AlbumDetail, Artist, ArtistProfile, Playlist, PlaylistDetail, SavedArtist, Track,
    UserProfile,
};

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
impl MusicApi for LibrespotClient {
    fn share_url(&self, kind: MediaKind, id: &str) -> Option<String> {
        let kind = match kind {
            MediaKind::Track => "track",
            MediaKind::Album => "album",
            MediaKind::Artist => "artist",
            MediaKind::Playlist => "playlist",
        };
        Some(format!("https://open.spotify.com/{kind}/{id}"))
    }

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

    async fn artist(&self, artist_id: &str) -> Result<Artist> {
        artists::artist(&self.session, artist_id).await
    }

    async fn artist_profile(&self, artist_id: &str) -> Result<ArtistProfile> {
        artists::profile(&self.session, artist_id).await
    }

    async fn artist_images(&self, ids: Vec<String>) -> Result<HashMap<String, String>> {
        artists::images(&self.session, &ids).await
    }

    async fn saved_tracks(&self, limit: u32) -> Result<Vec<Track>> {
        collection::saved_tracks(&self.session, limit).await
    }

    async fn set_track_saved(&self, track_id: &str, saved: bool) -> Result<()> {
        collection2::set_track_saved(&self.session, track_id, saved).await
    }

    async fn track(&self, track_id: &str) -> Result<Track> {
        collection::track(&self.session, track_id).await
    }

    async fn track_playcount(&self, track_id: &str) -> Result<Option<u64>> {
        pathfinder::track(&self.session, track_id).await
    }

    async fn saved_albums(&self, limit: u32) -> Result<Vec<Album>> {
        albums::saved_albums(&self.session, limit).await
    }

    async fn set_album_saved(&self, album_id: &str, saved: bool) -> Result<()> {
        collection2::set_album_saved(&self.session, album_id, saved).await
    }

    async fn saved_artists(&self, limit: u32) -> Result<Vec<SavedArtist>> {
        artists::saved_artists(&self.session, limit).await
    }

    async fn set_artist_saved(&self, artist_id: &str, saved: bool) -> Result<()> {
        collection2::set_artist_saved(&self.session, artist_id, saved).await
    }

    async fn album(&self, album_id: &str) -> Result<AlbumDetail> {
        albums::album(&self.session, album_id).await
    }

    async fn album_tracks(&self, album_id: &str) -> Result<Vec<Track>> {
        albums::album_tracks(&self.session, album_id).await
    }

    async fn playlist(&self, playlist_id: &str) -> Result<PlaylistDetail> {
        let mut detail = playlists::playlist(&self.session, playlist_id).await?;
        let owner = detail.playlist.owner.clone();
        if owner != wire::UNKNOWN {
            let names =
                profiles::display_names(&self.session, HashSet::from([owner.clone()])).await;
            if let Some(name) = names.get(&owner) {
                detail.playlist.owner = name.clone();
            }
        }
        Ok(detail)
    }

    async fn playlist_covers(&self, playlist_id: &str, wanted: usize) -> Result<Vec<String>> {
        playlists::covers(&self.session, playlist_id, wanted).await
    }

    async fn playlist_tracks(&self, playlist_id: &str) -> Result<Vec<Track>> {
        playlists::playlist_tracks(&self.session, playlist_id).await
    }

    async fn track_radio(&self, track_id: &str) -> Result<Vec<Track>> {
        radio::track_radio(&self.session, track_id).await
    }

    async fn search(&self, query: &str) -> Result<Vec<Track>> {
        search::search(&self.session, query).await
    }

    async fn create_playlist(&self, name: &str) -> Result<String> {
        playlists::create(&self.session, name).await
    }

    async fn rename_playlist(&self, playlist_id: &str, name: &str) -> Result<()> {
        playlists::rename(&self.session, playlist_id, name).await
    }

    async fn delete_playlist(&self, playlist_id: &str) -> Result<()> {
        playlists::delete(&self.session, playlist_id).await
    }

    async fn remove_playlist_from_library(&self, playlist_id: &str) -> Result<()> {
        playlists::remove_from_library(&self.session, playlist_id).await
    }

    async fn add_playlist_to_library(&self, playlist_id: &str) -> Result<()> {
        playlists::add_to_library(&self.session, playlist_id).await
    }

    async fn set_playlist_public(&self, playlist_id: &str, public: bool) -> Result<()> {
        playlists::set_public(&self.session, playlist_id, public).await
    }

    async fn add_track_to_playlist(&self, playlist_id: &str, track_id: &str) -> Result<()> {
        playlists::add_track(&self.session, playlist_id, track_id).await
    }

    async fn remove_track_from_playlist(&self, playlist_id: &str, track_id: &str) -> Result<()> {
        playlists::remove_track(&self.session, playlist_id, track_id).await
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
            playlist.owned = playlist.owner == self.session.username();
            if let Some(name) = names.get(&playlist.owner) {
                playlist.owner = name.clone();
            }
        }

        Ok(playlists)
    }
}
