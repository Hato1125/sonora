use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserProfile {
    pub id: String,
    pub display_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtistRef {
    pub name: String,
    pub id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Credit {
    pub name: String,
    pub role: String,
    pub id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Track {
    pub id: Option<String>,
    pub name: String,
    pub playable: bool,
    pub artists: String,
    pub artist_refs: Vec<ArtistRef>,
    pub album: String,
    pub album_id: Option<String>,
    pub cover: Option<String>,
    pub duration: Duration,
    pub added_at: Option<i64>,
    pub playcount: Option<u64>,
    pub popularity: u32,
    pub explicit: bool,
    pub track_number: u32,
    pub disc_number: u32,
    pub tags: Vec<String>,
    pub languages: Vec<String>,
    pub credits: Vec<Credit>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Playlist {
    pub id: String,
    pub name: String,
    pub owner: String,
    pub owned: bool,
    pub collaborative: bool,
    pub public: bool,
    pub cover: Option<String>,
    pub track_count: u32,
    pub modified_at: Option<i64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ReleaseType {
    Album,
    Single,
    Compilation,
    Ep,
    Audiobook,
    Podcast,
}

impl ReleaseType {
    pub fn label(self) -> &'static str {
        match self {
            Self::Album => "Album",
            Self::Single => "Single",
            Self::Compilation => "Compilation",
            Self::Ep => "EP",
            Self::Audiobook => "Audiobook",
            Self::Podcast => "Podcast",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Album {
    pub id: String,
    pub name: String,
    pub artists: String,
    pub artist_refs: Vec<ArtistRef>,
    pub cover: Option<String>,
    pub cover_large: Option<String>,
    pub release_type: ReleaseType,
    pub year: i32,
    pub track_count: u32,
    pub release_date: String,
    pub label: String,
    pub copyrights: Vec<String>,
    pub added_at: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlbumDetail {
    pub album: Album,
    pub tracks: Vec<Track>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Genre {
    pub id: String,
    pub name: String,
    pub cover: Option<String>,
    pub color: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GenreItem {
    Playlist(Playlist),
    Album(Album),
    Genre(Genre),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GenreSection {
    pub title: String,
    pub items: Vec<GenreItem>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GenreDetail {
    pub name: String,
    pub sections: Vec<GenreSection>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlaylistDetail {
    pub playlist: Playlist,
    pub tracks: Vec<Track>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtistProfile {
    pub name: String,
    pub cover_large: Option<String>,
    pub biography: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SavedArtist {
    pub id: String,
    pub name: String,
    pub cover: Option<String>,
    pub added_at: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Artist {
    pub name: String,
    pub cover_large: Option<String>,
    pub biography: Option<String>,
    pub monthly_listeners: Option<u64>,
    pub top_tracks: Vec<Track>,
    pub albums: Vec<Album>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Lyrics {
    Plain(String),
    Synced { lines: Vec<LyricsLine> },
}

impl Lyrics {
    pub fn synced(&self) -> bool {
        matches!(self, Self::Synced { .. })
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Self::Plain(text) => text.trim().is_empty(),
            Self::Synced { lines } => lines.is_empty(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LyricsLine {
    pub start: Duration,
    pub end: Option<Duration>,
    pub text: String,
    pub words: Option<Vec<LyricsWord>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LyricsWord {
    pub start: Duration,
    pub end: Duration,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LyricsQuery {
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub duration: Duration,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LyricsHit {
    pub source: &'static str,
    pub lyrics: Lyrics,
    pub title: String,
    pub artist: String,
    pub duration: Option<Duration>,
}
