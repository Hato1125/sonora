use std::collections::HashMap;
use std::sync::RwLock;

use anyhow::{Result, anyhow};
use async_trait::async_trait;

use crate::{
    Album, AlbumDetail, Artist, ArtistProfile, MediaKind, MusicApi, Playlist, PlaylistDetail,
    SavedArtist, Track, UserProfile,
};

use super::scan::Scanned;
use super::wire;

const NOT_SUPPORTED: &str = "local files do not support playlists";

pub struct LocalClient {
    scanned: RwLock<Scanned>,
}

impl LocalClient {
    pub fn new(scanned: Scanned) -> Self {
        Self {
            scanned: RwLock::new(scanned),
        }
    }
}

#[async_trait]
impl MusicApi for LocalClient {
    fn share_url(&self, kind: MediaKind, id: &str) -> Option<String> {
        let path = match kind {
            MediaKind::Track => wire::path_from_track_id(id)?,
            MediaKind::Album => wire::path_from_album_id(id)?,
            MediaKind::Artist | MediaKind::Playlist => return None,
        };
        Some(format!("file://{}", path.display()))
    }

    async fn profile(&self) -> Result<UserProfile> {
        Ok(UserProfile {
            id: "local".to_owned(),
            display_name: "Local Files".to_owned(),
        })
    }

    async fn artist(&self, artist_id: &str) -> Result<Artist> {
        let name = wire::artist_name_from_id(artist_id)
            .ok_or_else(|| anyhow!("{artist_id} is not a local artist id"))?;
        let scanned = self.scanned.read().unwrap();
        Ok(Artist {
            name: name.to_owned(),
            cover_large: None,
            biography: None,
            monthly_listeners: None,
            top_tracks: scanned
                .tracks
                .iter()
                .filter(|track| track.artists == name)
                .cloned()
                .collect(),
            albums: scanned
                .albums
                .iter()
                .filter(|album| album.artists == name)
                .cloned()
                .collect(),
        })
    }

    async fn artist_profile(&self, artist_id: &str) -> Result<ArtistProfile> {
        let name = wire::artist_name_from_id(artist_id)
            .ok_or_else(|| anyhow!("{artist_id} is not a local artist id"))?;
        Ok(ArtistProfile {
            name: name.to_owned(),
            cover_large: None,
            biography: None,
        })
    }

    async fn artist_images(&self, _ids: Vec<String>) -> Result<HashMap<String, String>> {
        Ok(HashMap::new())
    }

    async fn saved_tracks(&self, limit: u32) -> Result<Vec<Track>> {
        let scanned = self.scanned.read().unwrap();
        Ok(scanned
            .tracks
            .iter()
            .take(limit as usize)
            .cloned()
            .collect())
    }

    async fn set_track_saved(&self, _track_id: &str, _saved: bool) -> Result<()> {
        Ok(())
    }

    async fn track(&self, track_id: &str) -> Result<Track> {
        let scanned = self.scanned.read().unwrap();
        scanned
            .tracks
            .iter()
            .find(|track| track.id.as_deref() == Some(track_id))
            .cloned()
            .ok_or_else(|| anyhow!("cannot find local track {track_id}"))
    }

    async fn track_playcount(&self, _track_id: &str) -> Result<Option<u64>> {
        Ok(None)
    }

    async fn playlists(&self, _limit: u32) -> Result<Vec<Playlist>> {
        Ok(Vec::new())
    }

    async fn create_playlist(&self, _name: &str) -> Result<String> {
        Err(anyhow!(NOT_SUPPORTED))
    }

    async fn rename_playlist(&self, _playlist_id: &str, _name: &str) -> Result<()> {
        Err(anyhow!(NOT_SUPPORTED))
    }

    async fn delete_playlist(&self, _playlist_id: &str) -> Result<()> {
        Err(anyhow!(NOT_SUPPORTED))
    }

    async fn remove_playlist_from_library(&self, _playlist_id: &str) -> Result<()> {
        Err(anyhow!(NOT_SUPPORTED))
    }

    async fn add_playlist_to_library(&self, _playlist_id: &str) -> Result<()> {
        Err(anyhow!(NOT_SUPPORTED))
    }

    async fn set_playlist_public(&self, _playlist_id: &str, _public: bool) -> Result<()> {
        Err(anyhow!(NOT_SUPPORTED))
    }

    async fn add_track_to_playlist(&self, _playlist_id: &str, _track_id: &str) -> Result<()> {
        Err(anyhow!(NOT_SUPPORTED))
    }

    async fn remove_track_from_playlist(&self, _playlist_id: &str, _track_id: &str) -> Result<()> {
        Err(anyhow!(NOT_SUPPORTED))
    }

    async fn saved_albums(&self, limit: u32) -> Result<Vec<Album>> {
        let scanned = self.scanned.read().unwrap();
        Ok(scanned
            .albums
            .iter()
            .take(limit as usize)
            .cloned()
            .collect())
    }

    async fn set_album_saved(&self, _album_id: &str, _saved: bool) -> Result<()> {
        Ok(())
    }

    async fn saved_artists(&self, _limit: u32) -> Result<Vec<SavedArtist>> {
        Ok(Vec::new())
    }

    async fn set_artist_saved(&self, _artist_id: &str, _saved: bool) -> Result<()> {
        Ok(())
    }

    async fn album(&self, album_id: &str) -> Result<AlbumDetail> {
        let scanned = self.scanned.read().unwrap();
        let album = scanned
            .albums
            .iter()
            .find(|album| album.id == album_id)
            .cloned()
            .ok_or_else(|| anyhow!("cannot find local album {album_id}"))?;
        let tracks = scanned
            .tracks
            .iter()
            .filter(|track| track.album_id.as_deref() == Some(album_id))
            .cloned()
            .collect();
        Ok(AlbumDetail { album, tracks })
    }

    async fn album_tracks(&self, album_id: &str) -> Result<Vec<Track>> {
        let scanned = self.scanned.read().unwrap();
        Ok(scanned
            .tracks
            .iter()
            .filter(|track| track.album_id.as_deref() == Some(album_id))
            .cloned()
            .collect())
    }

    async fn playlist(&self, _playlist_id: &str) -> Result<PlaylistDetail> {
        Err(anyhow!(NOT_SUPPORTED))
    }

    async fn playlist_tracks(&self, _playlist_id: &str) -> Result<Vec<Track>> {
        Err(anyhow!(NOT_SUPPORTED))
    }

    async fn playlist_covers(&self, _playlist_id: &str, _wanted: usize) -> Result<Vec<String>> {
        Ok(Vec::new())
    }

    async fn track_radio(&self, _track_id: &str) -> Result<Vec<Track>> {
        Ok(Vec::new())
    }

    async fn search(&self, query: &str) -> Result<Vec<Track>> {
        let query = query.to_lowercase();
        let scanned = self.scanned.read().unwrap();
        Ok(scanned
            .tracks
            .iter()
            .filter(|track| {
                track.name.to_lowercase().contains(&query)
                    || track.artists.to_lowercase().contains(&query)
                    || track.album.to_lowercase().contains(&query)
            })
            .cloned()
            .collect())
    }
}
