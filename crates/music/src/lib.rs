mod audio;
pub mod local;
pub mod lrclib;
pub mod lyrics;
mod models;
pub mod spotify;
pub mod youtube;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;

pub use models::{
    Album, AlbumDetail, Artist, ArtistProfile, ArtistRef, Credit, Lyrics, LyricsHit, LyricsLine,
    LyricsQuery, LyricsWord, Playlist, PlaylistDetail, ReleaseType, SavedArtist, Track,
    UserProfile,
};

pub const LOCAL_TRACK_PREFIX: &str = "local:";
pub const LOCAL_ALBUM_PREFIX: &str = "local-album:";
pub const LOCAL_ARTIST_PREFIX: &str = "local-artist:";

pub fn is_local_id(id: &str) -> bool {
    id.starts_with(LOCAL_TRACK_PREFIX)
        || id.starts_with(LOCAL_ALBUM_PREFIX)
        || id.starts_with(LOCAL_ARTIST_PREFIX)
}

pub fn distinct_covers(tracks: &[Track], wanted: usize) -> Vec<String> {
    let mut covers: Vec<String> = Vec::with_capacity(wanted);
    for cover in tracks.iter().filter_map(|track| track.cover.as_deref()) {
        if covers.len() == wanted {
            break;
        }
        if !covers.iter().any(|kept| kept == cover) {
            covers.push(cover.to_owned());
        }
    }

    covers
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaKind {
    Track,
    Album,
    Artist,
    Playlist,
}

#[async_trait]
pub trait MusicApi: Send + Sync {
    fn share_url(&self, kind: MediaKind, id: &str) -> Option<String>;
    async fn profile(&self) -> Result<UserProfile>;
    async fn artist(&self, artist_id: &str) -> Result<Artist>;
    async fn artist_profile(&self, artist_id: &str) -> Result<ArtistProfile>;
    async fn artist_images(&self, ids: Vec<String>) -> Result<HashMap<String, String>>;
    async fn saved_tracks(&self, limit: u32) -> Result<Vec<Track>>;
    async fn set_track_saved(&self, track_id: &str, saved: bool) -> Result<()>;
    async fn track(&self, track_id: &str) -> Result<Track>;
    async fn track_playcount(&self, track_id: &str) -> Result<Option<u64>>;
    async fn playlists(&self, limit: u32) -> Result<Vec<Playlist>>;
    async fn create_playlist(&self, name: &str) -> Result<String>;
    async fn rename_playlist(&self, playlist_id: &str, name: &str) -> Result<()>;
    async fn delete_playlist(&self, playlist_id: &str) -> Result<()>;
    async fn remove_playlist_from_library(&self, playlist_id: &str) -> Result<()>;
    async fn add_playlist_to_library(&self, playlist_id: &str) -> Result<()>;
    async fn set_playlist_public(&self, playlist_id: &str, public: bool) -> Result<()>;
    async fn add_track_to_playlist(&self, playlist_id: &str, track_id: &str) -> Result<()>;
    async fn remove_track_from_playlist(&self, playlist_id: &str, track_id: &str) -> Result<()>;
    async fn saved_albums(&self, limit: u32) -> Result<Vec<Album>>;
    async fn set_album_saved(&self, album_id: &str, saved: bool) -> Result<()>;
    async fn saved_artists(&self, limit: u32) -> Result<Vec<SavedArtist>>;
    async fn set_artist_saved(&self, artist_id: &str, saved: bool) -> Result<()>;
    async fn album(&self, album_id: &str) -> Result<AlbumDetail>;
    async fn album_tracks(&self, album_id: &str) -> Result<Vec<Track>>;
    async fn playlist(&self, playlist_id: &str) -> Result<PlaylistDetail>;
    async fn playlist_tracks(&self, playlist_id: &str) -> Result<Vec<Track>>;
    async fn playlist_covers(&self, playlist_id: &str, wanted: usize) -> Result<Vec<String>>;
    async fn track_radio(&self, track_id: &str) -> Result<Vec<Track>>;
    async fn search(&self, query: &str) -> Result<Vec<Track>>;
}

#[async_trait]
pub trait LyricsProvider: Send + Sync {
    fn name(&self) -> &'static str;
    async fn search(&self, query: &LyricsQuery) -> Result<Vec<LyricsHit>>;
}

#[derive(Clone, Copy, Debug)]
pub struct PlaybackConfig {
    pub normalisation: bool,
    pub gapless: bool,
    pub position_interval: Duration,
    pub gain: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlaybackEvent {
    Loading(Duration),
    Playing(Duration),
    Paused(Duration),
    Position(Duration),
    Length(Duration),
    Ended,
    Unavailable,
    Refused,
}

pub trait Player: Send + Sync {
    fn load(&self, track_id: &str, seamless: bool) -> Result<()>;
    fn preload(&self, track_id: &str) -> Result<()>;
    fn play(&self);
    fn pause(&self);
    fn seek(&self, position: Duration);
    fn set_gain(&self, gain: f32);
}

#[async_trait]
pub trait PlaybackEvents: Send {
    async fn next(&mut self) -> Option<PlaybackEvent>;
}

pub trait PlaybackFactory: Send + Sync {
    fn start(&self, config: PlaybackConfig) -> (Box<dyn Player>, Box<dyn PlaybackEvents>);
}

pub struct ProviderSession {
    pub profile: UserProfile,
    pub api: Arc<dyn MusicApi>,
    pub playback: Arc<dyn PlaybackFactory>,
    pub authenticated: bool,
    pub playcounts: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SignIn {
    Default,
    Anonymous,
    Browser(String),
    Secret,
    Path(PathBuf),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SignInPrompt {
    Code { code: String, url: String },
    Secret,
}

pub type PromptSink = Arc<dyn Fn(SignInPrompt) + Send + Sync>;
pub type InputSource = tokio::sync::mpsc::UnboundedReceiver<String>;

#[async_trait]
pub trait MusicProvider: Send + Sync {
    fn name(&self) -> &'static str;
    fn slug(&self) -> &'static str;
    fn sign_in_options(&self) -> Vec<SignIn>;
    fn stored(&self) -> bool;
    fn location(&self) -> Option<String> {
        None
    }
    async fn restore(&self) -> Result<Option<ProviderSession>>;
    async fn sign_in(
        &self,
        method: SignIn,
        prompt: PromptSink,
        input: InputSource,
    ) -> Result<ProviderSession>;
    fn sign_out(&self);
}
