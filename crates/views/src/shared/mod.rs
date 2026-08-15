pub(crate) mod about;
pub(crate) mod accounts;
pub(crate) mod adaptive;
pub(crate) mod album_grid;
pub(crate) mod browsers;
pub(crate) mod cells;
pub(crate) mod hero;
pub(crate) mod menu;
pub(crate) mod page;
pub(crate) mod picks;
pub(crate) mod playlist_editor;
pub(crate) mod shelves;
pub(crate) mod tracks;
pub(crate) mod trouble;

use gpui::prelude::*;
use gpui::{App, Div, Pixels, div, px, svg};
use i18n::t;
use ui::{ActiveTheme as _, Text};

const NOTE: Pixels = px(14.);

pub(crate) fn firefox_note(cx: &App) -> Div {
    let theme = *cx.theme();
    div()
        .flex()
        .items_center()
        .gap_1()
        .text_size(theme.text(Text::Small))
        .text_color(theme.muted_foreground)
        .child(
            svg()
                .path("icons/firefoxbrowser.svg")
                .size(NOTE)
                .flex_none()
                .text_color(theme.muted_foreground),
        )
        .child(t!("login-browser-firefox"))
}

pub(crate) fn provider_logo(slug: &str) -> &'static str {
    match slug {
        "spotify" => "icons/spotify.svg",
        "youtube" => "icons/youtubemusic.svg",
        _ => "icons/music.svg",
    }
}
