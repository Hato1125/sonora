// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context as _, Result};
use async_trait::async_trait;
use tokio::task::JoinSet;
use ytmusic::YtMusic;

use crate::youtube::wire;
use crate::{
    Album, AlbumDetail, Artist, ArtistProfile, MediaKind, MusicApi, Playlist, PlaylistDetail,
    Track, UserProfile,
};

const PORTRAIT_LIMIT: usize = 24;

pub struct YouTubeClient {
    api: Arc<YtMusic>,
}

impl YouTubeClient {
    pub fn new(api: Arc<YtMusic>) -> Self {
        Self { api }
    }
}

#[async_trait]
impl MusicApi for YouTubeClient {
    fn share_url(&self, kind: MediaKind, id: &str) -> Option<String> {
        let url = match kind {
            MediaKind::Track => format!("https://music.youtube.com/watch?v={id}"),
            MediaKind::Album => format!("https://music.youtube.com/browse/{id}"),
            MediaKind::Artist => format!("https://music.youtube.com/channel/{id}"),
            MediaKind::Playlist => format!("https://music.youtube.com/playlist?list={id}"),
        };
        Some(url)
    }

    async fn profile(&self) -> Result<UserProfile> {
        Ok(wire::profile(self.api.profile().await?))
    }

    async fn artist(&self, artist_id: &str) -> Result<Artist> {
        Ok(wire::artist(self.api.artist(artist_id).await?))
    }

    async fn artist_profile(&self, artist_id: &str) -> Result<ArtistProfile> {
        Ok(wire::artist_profile(&self.api.artist(artist_id).await?))
    }

    async fn artist_images(&self, ids: Vec<String>) -> Result<HashMap<String, String>> {
        let mut tasks = JoinSet::new();
        for id in ids.into_iter().take(PORTRAIT_LIMIT) {
            let api = self.api.clone();
            tasks.spawn(async move {
                let artist = api.artist(&id).await.ok()?;
                let image = wire::cover_large(&artist.thumbnails)?;
                Some((id, image))
            });
        }
        let mut images = HashMap::new();
        while let Some(result) = tasks.join_next().await {
            if let Ok(Some((id, image))) = result {
                images.insert(id, image);
            }
        }
        Ok(images)
    }

    async fn saved_tracks(&self, limit: u32) -> Result<Vec<Track>> {
        let mut tracks: Vec<Track> = self
            .api
            .liked_songs_resolved()
            .await?
            .into_iter()
            .enumerate()
            .map(|(index, track)| wire::track(track, index as u32))
            .collect();
        tracks.truncate(limit as usize);
        Ok(tracks)
    }

    async fn set_track_saved(&self, track_id: &str, saved: bool) -> Result<()> {
        self.api.rate_track(track_id, saved).await
    }

    async fn track(&self, track_id: &str) -> Result<Track> {
        let response = self
            .api
            .player_response(track_id, ytmusic::Client::Music)
            .await?;
        let details = response
            .get("videoDetails")
            .context("player response has no video details")?;
        let title = details
            .get("title")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string();
        let author = details
            .get("author")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string();
        let seconds: u64 = details
            .get("lengthSeconds")
            .and_then(serde_json::Value::as_str)
            .and_then(|length| length.parse().ok())
            .unwrap_or(0);
        let channel = details
            .get("channelId")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string);
        let source = ytmusic::Track {
            video_id: Some(track_id.to_string()),
            title,
            artists: vec![ytmusic::ArtistRef {
                name: author,
                id: channel,
            }],
            album: None,
            duration: Some(std::time::Duration::from_secs(seconds)),
            thumbnails: Vec::new(),
            explicit: false,
            available: true,
            kind: ytmusic::TrackKind::Song,
            set_video_id: None,
            liked: None,
            views: None,
        };
        let mut track = wire::track(source, 0);
        track.cover = details
            .get("thumbnail")
            .map(|thumbnail| {
                let thumbs = collect_thumbnails(thumbnail);
                wire::cover(&thumbs)
            })
            .unwrap_or_default();
        Ok(track)
    }

    async fn track_playcount(&self, _track_id: &str) -> Result<Option<u64>> {
        Ok(None)
    }

    async fn playlists(&self, limit: u32) -> Result<Vec<Playlist>> {
        let mut playlists: Vec<Playlist> = self
            .api
            .library_playlists()
            .await?
            .into_iter()
            .map(|playlist| wire::playlist(playlist, false, false))
            .collect();
        playlists.truncate(limit as usize);
        Ok(playlists)
    }

    async fn create_playlist(&self, name: &str) -> Result<String> {
        self.api.create_playlist(name).await
    }

    async fn rename_playlist(&self, playlist_id: &str, name: &str) -> Result<()> {
        self.api.rename_playlist(playlist_id, name).await
    }

    async fn delete_playlist(&self, playlist_id: &str) -> Result<()> {
        self.api.delete_playlist(playlist_id).await
    }

    async fn remove_playlist_from_library(&self, playlist_id: &str) -> Result<()> {
        self.api.rate_playlist(playlist_id, false).await
    }

    async fn add_playlist_to_library(&self, playlist_id: &str) -> Result<()> {
        self.api.rate_playlist(playlist_id, true).await
    }

    async fn set_playlist_public(&self, playlist_id: &str, public: bool) -> Result<()> {
        self.api.set_playlist_privacy(playlist_id, public).await
    }

    async fn add_track_to_playlist(&self, playlist_id: &str, track_id: &str) -> Result<()> {
        self.api.add_playlist_track(playlist_id, track_id).await
    }

    async fn remove_track_from_playlist(&self, playlist_id: &str, track_id: &str) -> Result<()> {
        let detail = self.api.playlist(playlist_id).await?;
        let set_video_id = detail
            .tracks
            .iter()
            .find(|track| track.video_id.as_deref() == Some(track_id))
            .and_then(|track| track.set_video_id.clone())
            .context("track is not in the playlist")?;
        self.api
            .remove_playlist_track(playlist_id, track_id, &set_video_id)
            .await
    }

    async fn saved_albums(&self, limit: u32) -> Result<Vec<Album>> {
        let mut albums: Vec<Album> = self
            .api
            .library_albums()
            .await?
            .into_iter()
            .map(wire::album)
            .collect();
        albums.truncate(limit as usize);
        Ok(albums)
    }

    async fn set_album_saved(&self, album_id: &str, saved: bool) -> Result<()> {
        let detail = self.api.album(album_id).await?;
        let playlist_id = detail
            .album
            .playlist_id
            .context("album has no audio playlist")?;
        self.api.rate_playlist(&playlist_id, saved).await
    }

    async fn album(&self, album_id: &str) -> Result<AlbumDetail> {
        Ok(wire::album_detail(self.api.album(album_id).await?))
    }

    async fn album_tracks(&self, album_id: &str) -> Result<Vec<Track>> {
        Ok(self.album(album_id).await?.tracks)
    }

    async fn playlist(&self, playlist_id: &str) -> Result<PlaylistDetail> {
        Ok(wire::playlist_detail(self.api.playlist(playlist_id).await?))
    }

    async fn playlist_tracks(&self, playlist_id: &str) -> Result<Vec<Track>> {
        Ok(self.playlist(playlist_id).await?.tracks)
    }

    async fn track_radio(&self, track_id: &str) -> Result<Vec<Track>> {
        Ok(self
            .api
            .track_radio(track_id)
            .await?
            .into_iter()
            .enumerate()
            .map(|(index, track)| wire::track(track, index as u32))
            .collect())
    }

    async fn search(&self, query: &str) -> Result<Vec<Track>> {
        Ok(self
            .api
            .search_songs(query)
            .await?
            .into_iter()
            .enumerate()
            .map(|(index, track)| wire::track(track, index as u32))
            .collect())
    }
}

fn collect_thumbnails(node: &serde_json::Value) -> Vec<ytmusic::Thumbnail> {
    node.get("thumbnails")
        .and_then(serde_json::Value::as_array)
        .map(|thumbs| {
            thumbs
                .iter()
                .filter_map(|thumb| {
                    Some(ytmusic::Thumbnail {
                        url: thumb.get("url")?.as_str()?.to_string(),
                        width: thumb.get("width").and_then(serde_json::Value::as_u64)? as u32,
                        height: thumb.get("height").and_then(serde_json::Value::as_u64)? as u32,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}
