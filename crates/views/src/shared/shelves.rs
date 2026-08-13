use gpui::prelude::*;
use gpui::{
    AnyElement, App, Context, Div, ElementId, Entity, FontWeight, Pixels, ScrollHandle,
    SharedString, div, point, px,
};
use music::{Album, GenreItem, GenreSection, Playlist};
use router::{Destination, navigate};
use state::{Origin, Playback, PlaybackState};
use ui::{
    ActiveTheme as _, Button, Card, Mode, Pin, PinKind, Pinnable as _, Skeleton, Text, heading,
};

use crate::shared::album_grid::CardGrid;
use crate::shared::cells;

const PLATE: Pixels = px(260.);
const LANES: usize = 5;
const ROWS: usize = 3;
const STEADY: Pixels = px(0.5);
const PENDING: usize = 3;
const HEADING: Pixels = px(140.);

pub(crate) struct Shelves {
    id: &'static str,
    playback: Entity<Playback>,
    rails: Vec<ScrollHandle>,
}

impl Shelves {
    pub(crate) fn new(id: &'static str, playback: Entity<Playback>) -> Self {
        Self {
            id,
            playback,
            rails: Vec::new(),
        }
    }

    fn tag(&self, kind: &str, place: usize) -> SharedString {
        SharedString::from(format!("{}-{kind}-{place}", self.id))
    }

    pub(crate) fn pending(&self, width: Pixels, cx: &App) -> Vec<AnyElement> {
        let theme = *cx.theme();
        let layout = CardGrid::layout(width);

        (0..PENDING)
            .map(|shelf| {
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        Skeleton::new()
                            .w(HEADING)
                            .h(theme.text(Text::Large))
                            .rounded(theme.radius),
                    )
                    .child(div().flex().w_full().gap_4().overflow_hidden().children(
                        (0..layout.columns).map(|place| {
                            Card::new(self.tag("pending", shelf * 100 + place), "")
                                .loading()
                                .tile(layout.card)
                        }),
                    ))
                    .into_any_element()
            })
            .collect()
    }

    pub(crate) fn reset(&mut self) {
        self.rails.clear();
    }

    pub(crate) fn render(
        &mut self,
        sections: &[GenreSection],
        mode: Mode,
        width: Pixels,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        while self.rails.len() < sections.len() {
            self.rails.push(ScrollHandle::new());
        }

        sections
            .iter()
            .enumerate()
            .map(|(place, section)| match mode {
                Mode::Cards => self.rail(place, section, width, cx),
                Mode::List => self.lane(place, section, width, cx),
            })
            .collect()
    }

    fn lane(
        &self,
        place: usize,
        section: &GenreSection,
        width: Pixels,
        cx: &Context<Self>,
    ) -> AnyElement {
        let lanes = lanes(width);
        let cards = section
            .items
            .iter()
            .take(lanes * ROWS)
            .enumerate()
            .map(|(index, item)| self.card(place * 100 + index, item, None, cx))
            .collect();

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(heading(SharedString::from(section.title.clone()), cx))
            .child(spread(cards, lanes))
            .into_any_element()
    }

    fn rail(
        &self,
        place: usize,
        section: &GenreSection,
        width: Pixels,
        cx: &Context<Self>,
    ) -> AnyElement {
        let layout = CardGrid::layout(width);
        let handle = self.rails[place].clone();
        let crowded = section.items.len() > layout.columns;
        let cards: Vec<AnyElement> = section
            .items
            .iter()
            .enumerate()
            .map(|(index, item)| self.card(place * 100 + index, item, Some(layout.card), cx))
            .collect();

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .items_end()
                    .justify_between()
                    .gap_4()
                    .child(heading(SharedString::from(section.title.clone()), cx))
                    .when(crowded, |this| this.child(self.arrows(place, &handle, cx))),
            )
            .child(
                div()
                    .id((self.id, place))
                    .flex()
                    .w_full()
                    .gap_4()
                    .overflow_x_scroll()
                    .restrict_scroll_to_axis()
                    .track_scroll(&handle)
                    .children(cards),
            )
            .into_any_element()
    }

    fn arrows(&self, place: usize, handle: &ScrollHandle, cx: &Context<Self>) -> AnyElement {
        let at = handle.offset().x;
        let reach = handle.max_offset().x;

        div()
            .flex()
            .flex_none()
            .items_center()
            .gap_1()
            .child(
                self.arrow(self.tag("previous", place), false, handle, cx)
                    .disabled(at >= -STEADY),
            )
            .child(
                self.arrow(self.tag("next", place), true, handle, cx)
                    .disabled(reach > Pixels::ZERO && at <= STEADY - reach),
            )
            .into_any_element()
    }

    fn arrow(
        &self,
        id: impl Into<ElementId>,
        forward: bool,
        handle: &ScrollHandle,
        cx: &Context<Self>,
    ) -> Button {
        let handle = handle.clone();
        let me = cx.entity().downgrade();

        Button::new(id)
            .small()
            .outline()
            .icon(match forward {
                true => "icons/chevron-right.svg",
                false => "icons/chevron-left.svg",
            })
            .tooltip(match forward {
                true => "common-next",
                false => "common-previous",
            })
            .on_click(move |_, _, cx| {
                slide(&handle, forward);
                me.update(cx, |_, cx| cx.notify()).ok();
            })
    }

    fn card(&self, id: usize, item: &GenreItem, tile: Option<Pixels>, cx: &App) -> AnyElement {
        match item {
            GenreItem::Playlist(playlist) => self.playlist_card(id, playlist.clone(), tile, cx),
            GenreItem::Album(album) => self.album_card(id, album.clone(), tile, cx),
            GenreItem::Genre(genre) => plate(self.tag("genre", id), genre.clone(), tile, cx),
        }
    }

    fn playlist_card(
        &self,
        id: usize,
        playlist: Playlist,
        tile: Option<Pixels>,
        cx: &App,
    ) -> AnyElement {
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

        Card::new(self.tag("playlist", id), SharedString::from(playlist.name))
            .cover(playlist.cover)
            .weight(FontWeight::SEMIBOLD)
            .underline()
            .meta(SharedString::from(playlist.owner))
            .map(|card| dressed(card, tile, cx))
            .play(playing, move |_, _, cx| {
                playback.update(cx, |playback, cx| playback.toggle_origin(&origin, cx));
            })
            .press(move |_, _, cx| navigate(Destination::Playlist(opened.clone()), cx))
            .pin(pin)
            .into_any_element()
    }

    fn album_card(&self, id: usize, album: Album, tile: Option<Pixels>, cx: &App) -> AnyElement {
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
            SharedString::from(format!("{}-album-artist-{id}", self.id)),
            album.artist_refs,
            album.artists,
            theme.muted_foreground,
        )
        .text_size(theme.text(Text::Small))
        .truncate();

        Card::new(self.tag("album", id), SharedString::from(album.name))
            .cover(album.cover)
            .weight(FontWeight::SEMIBOLD)
            .underline()
            .bare_meta(meta)
            .map(|card| dressed(card, tile, cx))
            .play(playing, move |_, _, cx| {
                playback.update(cx, |playback, cx| playback.toggle_origin(&origin, cx));
            })
            .press(move |_, _, cx| navigate(Destination::Album(opened.clone()), cx))
            .pin(pin)
            .into_any_element()
    }
}

pub(crate) fn plate(
    id: impl Into<ElementId>,
    genre: music::Genre,
    tile: Option<Pixels>,
    cx: &App,
) -> AnyElement {
    let opened = SharedString::from(genre.id);

    Card::new(id, SharedString::from(genre.name))
        .cover(genre.cover)
        .fallback("icons/music.svg")
        .weight(FontWeight::SEMIBOLD)
        .map(|card| dressed(card, tile, cx))
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

fn dressed(card: Card, tile: Option<Pixels>, cx: &App) -> Card {
    match tile {
        Some(width) => card.tile(width).flat(),
        None => card.bg(cx.theme().secondary),
    }
}

fn slide(handle: &ScrollHandle, forward: bool) {
    let page = handle.bounds().size.width;
    let reach = handle.max_offset().x.max(Pixels::ZERO);
    let at = handle.offset().x;
    let next = match forward {
        true => (at - page).max(-reach),
        false => (at + page).min(Pixels::ZERO),
    };

    handle.set_offset(point(next, handle.offset().y));
}
