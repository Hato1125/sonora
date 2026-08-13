use gpui::prelude::*;
use gpui::{
    AnyElement, App, Context, Div, ElementId, Entity, FontWeight, Hsla, Pixels, Render,
    ScrollHandle, SharedString, Window, div,
};
use i18n::t;
use music::{Album, GenreItem, Playlist};
use router::{Destination, navigate};
use state::{GenreDetails, Origin, Playback, PlaybackState};
use ui::{
    ActiveTheme as _, Card, Pin, PinKind, Pinnable as _, Scrollbar, Scroller, Skeleton, Text,
    heading, vacant,
};

use crate::chrome::Chrome;
use crate::shared::cells;

const TILE: Pixels = gpui::px(220.);
const PLATE: Pixels = gpui::px(260.);
const LANES: usize = 5;
const ROWS: usize = 3;

pub(crate) struct GenreView {
    detail: Entity<GenreDetails>,
    playback: Entity<Playback>,
    scrollbar: Entity<Scrollbar>,
}

impl GenreView {
    pub(crate) fn new(
        detail: Entity<GenreDetails>,
        playback: Entity<Playback>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&detail, |_, _, cx| cx.notify()).detach();
        cx.observe(&playback, |_, _, cx| cx.notify()).detach();
        let chrome = Chrome::entity(cx);
        cx.observe(&chrome, |_, _, cx| cx.notify()).detach();

        Self {
            detail,
            playback,
            scrollbar: cx.new(|_| Scrollbar::new(ScrollHandle::new())),
        }
    }

    fn playlist_card(&self, id: usize, playlist: Playlist, cx: &App) -> AnyElement {
        let origin = Origin::Playlist(playlist.id.clone());
        let playing = matches!(
            self.playback.read(cx).playing_from(&origin),
            Some(PlaybackState::Playing)
        );
        let pin = Pin::new(
            PinKind::Playlist,
            playlist.id.clone(),
            playlist.name.clone(),
        )
        .cover(playlist.cover.clone());
        let opened = SharedString::from(playlist.id);
        let playback = self.playback.clone();

        Card::new(("genre-playlist", id), SharedString::from(playlist.name))
            .cover(playlist.cover)
            .weight(FontWeight::SEMIBOLD)
            .underline()
            .meta(SharedString::from(playlist.owner))
            .bg(plated(cx))
            .play(playing, move |_, _, cx| {
                playback.update(cx, |playback, cx| playback.toggle_origin(&origin, cx));
            })
            .press(move |_, _, cx| navigate(Destination::Playlist(opened.clone()), cx))
            .pin(pin)
            .into_any_element()
    }

    fn album_card(&self, id: usize, album: Album, cx: &App) -> AnyElement {
        let theme = *cx.theme();
        let origin = Origin::Album(album.id.clone());
        let playing = matches!(
            self.playback.read(cx).playing_from(&origin),
            Some(PlaybackState::Playing)
        );
        let pin = Pin::new(PinKind::Album, album.id.clone(), album.name.clone())
            .cover(album.cover.clone());
        let opened = SharedString::from(album.id);
        let playback = self.playback.clone();
        let meta = cells::artist_links(
            SharedString::from(format!("genre-album-artist-{id}")),
            album.artist_refs,
            album.artists,
            theme.muted_foreground,
        )
        .text_size(theme.text(Text::Small))
        .truncate();

        Card::new(("genre-album", id), SharedString::from(album.name))
            .cover(album.cover)
            .weight(FontWeight::SEMIBOLD)
            .underline()
            .bare_meta(meta)
            .bg(plated(cx))
            .play(playing, move |_, _, cx| {
                playback.update(cx, |playback, cx| playback.toggle_origin(&origin, cx));
            })
            .press(move |_, _, cx| navigate(Destination::Album(opened.clone()), cx))
            .pin(pin)
            .into_any_element()
    }

    fn sections(&self, width: Pixels, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let lanes = lanes(width);
        let sections = self.detail.read(cx).sections().to_vec();

        sections
            .into_iter()
            .enumerate()
            .map(|(place, section)| {
                let cards = section
                    .items
                    .into_iter()
                    .take(lanes * ROWS)
                    .enumerate()
                    .map(|(index, item)| {
                        let id = place * 100 + index;
                        match item {
                            GenreItem::Playlist(playlist) => self.playlist_card(id, playlist, cx),
                            GenreItem::Album(album) => self.album_card(id, album, cx),
                            GenreItem::Genre(genre) => plate(("genre-plate", id), genre, cx),
                        }
                    })
                    .collect();

                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(heading(SharedString::from(section.title), cx))
                    .child(spread(cards, lanes))
                    .into_any_element()
            })
            .collect()
    }
}

impl Render for GenreView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.theme();
        let pad = theme.metrics.inset;
        let width = cells::content_width(window, pad * 2., cx);
        let detail = self.detail.read(cx);
        let loading = detail.is_loading();
        let title = detail.name().unwrap_or_default().to_owned();
        let error = detail.error().map(str::to_owned);
        let empty = !loading && detail.sections().is_empty();
        let sections = self.sections(width, cx);

        div().flex().flex_col().size_full().child(
            Scroller::new("genre", &self.scrollbar).p(pad).child(
                div()
                    .flex()
                    .flex_col()
                    .gap_8()
                    .child(
                        div()
                            .text_size(theme.text(Text::Display))
                            .font_weight(FontWeight::BOLD)
                            .child(SharedString::from(title)),
                    )
                    .when(loading, |this| this.child(Skeleton::new().w_full().h(TILE)))
                    .children(error.map(|error| {
                        div()
                            .text_color(theme.danger)
                            .child(SharedString::from(error))
                    }))
                    .when(empty, |this| this.child(vacant(t!("genre-empty"), cx)))
                    .children(sections),
            ),
        )
    }
}

pub(crate) fn plate(id: impl Into<ElementId>, genre: music::Genre, cx: &App) -> AnyElement {
    let opened = SharedString::from(genre.id);

    Card::new(id, SharedString::from(genre.name))
        .cover(genre.cover)
        .fallback("icons/music.svg")
        .weight(FontWeight::SEMIBOLD)
        .bg(plated(cx))
        .press(move |_, _, cx| navigate(Destination::Genre(opened.clone()), cx))
        .into_any_element()
}

pub(crate) fn lanes(width: Pixels) -> usize {
    ((width / PLATE).floor().max(1.) as usize).min(LANES)
}

pub(crate) fn spread(cards: Vec<AnyElement>, lanes: usize) -> Div {
    let mut columns: Vec<Vec<AnyElement>> = (0..lanes).map(|_| Vec::new()).collect();
    for (place, card) in cards.into_iter().enumerate() {
        columns[place % lanes].push(card);
    }

    div()
        .flex()
        .w_full()
        .gap_2()
        .children(columns.into_iter().map(|column| {
            div()
                .flex()
                .flex_1()
                .min_w_0()
                .flex_col()
                .gap_2()
                .children(column)
        }))
}

fn plated(cx: &App) -> Hsla {
    cx.theme().secondary
}
