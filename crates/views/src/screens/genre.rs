use gpui::prelude::*;
use gpui::{
    AnyElement, App, Context, Entity, FontWeight, Hsla, Pixels, Render, ScrollHandle, SharedString,
    Window, div,
};
use i18n::t;
use music::{Album, GenreItem, Playlist};
use router::{Destination, navigate};
use state::{GenreDetails, Origin, Playback, PlaybackState};
use ui::{
    ActiveTheme as _, Card, Pin, PinKind, Pinnable as _, Scrollbar, Scroller, Skeleton, Text, Tile,
    eyebrow, heading, paint, vacant,
};

use crate::chrome::Chrome;
use crate::shared::album_grid::CardGrid;
use crate::shared::cells;
use crate::shared::tints::Tints;

const TILE: Pixels = gpui::px(220.);

pub(crate) struct GenreView {
    detail: Entity<GenreDetails>,
    playback: Entity<Playback>,
    tints: Entity<Tints>,
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
        let tints = cx.new(|_| Tints::default());
        cx.observe(&tints, |_, _, cx| cx.notify()).detach();

        Self {
            detail,
            playback,
            tints,
            scrollbar: cx.new(|_| Scrollbar::new(ScrollHandle::new())),
        }
    }

    fn playlist_card(&self, id: usize, playlist: Playlist, width: Pixels, cx: &App) -> AnyElement {
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
            .tile(width)
            .cover(playlist.cover)
            .weight(FontWeight::SEMIBOLD)
            .flat()
            .underline()
            .meta(SharedString::from(playlist.owner))
            .play(playing, move |_, _, cx| {
                playback.update(cx, |playback, cx| playback.toggle_origin(&origin, cx));
            })
            .press(move |_, _, cx| navigate(Destination::Playlist(opened.clone()), cx))
            .pin(pin)
            .into_any_element()
    }

    fn album_card(&self, id: usize, album: Album, width: Pixels, cx: &App) -> AnyElement {
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
            .tile(width)
            .cover(album.cover)
            .weight(FontWeight::SEMIBOLD)
            .flat()
            .underline()
            .bare_meta(meta)
            .play(playing, move |_, _, cx| {
                playback.update(cx, |playback, cx| playback.toggle_origin(&origin, cx));
            })
            .press(move |_, _, cx| navigate(Destination::Album(opened.clone()), cx))
            .pin(pin)
            .into_any_element()
    }

    fn sections(&self, width: Pixels, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let layout = CardGrid::layout(width);
        let tiles = CardGrid::tiles(width);
        let sections = self.detail.read(cx).sections().to_vec();

        sections
            .into_iter()
            .enumerate()
            .map(|(place, section)| {
                let genres = section
                    .items
                    .iter()
                    .all(|item| matches!(item, GenreItem::Genre(_)));
                let shape = match genres {
                    true => tiles,
                    false => layout,
                };
                let cards = section
                    .items
                    .into_iter()
                    .take(shape.columns)
                    .enumerate()
                    .map(|(index, item)| {
                        let id = place * 100 + index;
                        match item {
                            GenreItem::Playlist(playlist) => {
                                self.playlist_card(id, playlist, shape.card, cx)
                            }
                            GenreItem::Album(album) => self.album_card(id, album, shape.card, cx),
                            GenreItem::Genre(genre) => self.tile(genre, shape.card, cx),
                        }
                    })
                    .collect::<Vec<_>>();

                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(heading(SharedString::from(section.title), cx))
                    .child(CardGrid::of(shape).children(cards))
                    .into_any_element()
            })
            .collect()
    }

    pub(crate) fn tile(
        &self,
        genre: music::Genre,
        width: Pixels,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let wash = wash(&self.tints, &genre, cx);
        let opened = SharedString::from(genre.id);

        Tile::new(
            SharedString::from(format!("genre-tile-{opened}")),
            genre.name,
            width,
        )
        .wash(wash)
        .cover(genre.cover)
        .press(move |_, _, cx| navigate(Destination::Genre(opened.clone()), cx))
        .into_any_element()
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
            Scroller::new("genre", &self.scrollbar).child(
                div()
                    .flex()
                    .flex_col()
                    .gap_8()
                    .p(pad)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(eyebrow(t!("genre-eyebrow"), cx))
                            .child(
                                div()
                                    .text_size(theme.text(Text::Display))
                                    .font_weight(FontWeight::BOLD)
                                    .child(SharedString::from(title)),
                            ),
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

pub(crate) fn wash<V: 'static>(
    tints: &Entity<Tints>,
    genre: &music::Genre,
    cx: &mut Context<V>,
) -> Option<Hsla> {
    match genre.cover.as_deref() {
        Some(cover) => tints.update(cx, |tints, cx| tints.of(cover, cx)),
        None => genre.color.map(paint),
    }
}
