pub(crate) mod adaptive;
pub(crate) mod album_grid;
pub(crate) mod browsers;
pub(crate) mod cells;
pub(crate) mod hero;
pub(crate) mod menu;
pub(crate) mod page;
pub(crate) mod playlist_editor;
pub(crate) mod tracks;

pub(crate) fn provider_logo(slug: &str) -> &'static str {
    match slug {
        "spotify" => "icons/spotify.svg",
        "youtube" => "icons/youtubemusic.svg",
        _ => "icons/music.svg",
    }
}
