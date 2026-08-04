use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserProfile {
    pub id: String,
    pub display_name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Track {
    pub id: Option<String>,
    pub name: String,
    pub artists: String,
    pub album: String,
    pub cover: Option<String>,
    pub duration: Duration,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Playlist {
    pub id: String,
    pub name: String,
    pub owner: String,
    pub cover: Option<String>,
    pub track_count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Album {
    pub id: String,
    pub name: String,
    pub artists: String,
    pub cover: Option<String>,
    pub year: i32,
    pub track_count: u32,
}
