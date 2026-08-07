// SPDX-License-Identifier: GPL-3.0-or-later

use gpui::prelude::*;
use gpui::{App, Entity, FontWeight, SharedString, Window, div};
use i18n::t;
use router::{Destination, navigate};
use spotify::{Album, ReleaseType};
use state::Playback;
use ui::{ActiveTheme as _, Card, Text};

pub(crate) fn release_label(kind: ReleaseType) -> SharedString {
    match kind {
        ReleaseType::Album => t!("release-album"),
        ReleaseType::Single => t!("release-single"),
        ReleaseType::Compilation => t!("release-compilation"),
        ReleaseType::Ep => t!("release-ep"),
        ReleaseType::Audiobook => t!("release-audiobook"),
        ReleaseType::Podcast => t!("release-podcast"),
    }
}

#[derive(IntoElement)]
pub(crate) struct ReleaseCard {
    index: usize,
    album: Album,
    playback: Entity<Playback>,
}

impl ReleaseCard {
    pub(crate) fn new(index: usize, album: Album, playback: Entity<Playback>) -> Self {
        Self {
            index,
            album,
            playback,
        }
    }
}

impl RenderOnce for ReleaseCard {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let Self {
            index,
            album,
            playback,
        } = self;

        let theme = *cx.theme();
        let cover = album.cover_large.clone().or_else(|| album.cover.clone());
        let release = release_label(album.release_type);
        let metadata = match album.year > 0 {
            true => t!("release-meta", year = album.year, kind = &release),
            false => release,
        };
        let played = album.id.clone();
        let opened = SharedString::from(album.id);

        Card::new(("artist-release", index), SharedString::from(album.name))
            .tile(theme.metrics.cover)
            .cover(cover)
            .weight(FontWeight::SEMIBOLD)
            .flat()
            .underline()
            .line_height(theme.text(Text::Body))
            .bare_meta(
                div()
                    .text_size(theme.text(Text::Small))
                    .line_height(theme.text(Text::Small))
                    .text_color(theme.muted_foreground)
                    .child(metadata),
            )
            .play(move |_, _, cx| {
                playback.update(cx, |playback, cx| playback.play_album(&played, cx));
            })
            .press(move |_, _, cx| navigate(Destination::Album(opened.clone()), cx))
    }
}
