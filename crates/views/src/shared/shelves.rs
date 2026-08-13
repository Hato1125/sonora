use gpui::prelude::*;
use gpui::{
    AnyElement, App, Context, Div, ElementId, Entity, FontWeight, Pixels, ScrollHandle,
    ScrollWheelEvent, SharedString, WeakEntity, Window, div, point, px,
};
use std::cell::Cell;
use std::rc::Rc;

use music::{Album, GenreItem, GenreSection, Playlist};
use router::{Destination, navigate};
use state::{Origin, Playback, PlaybackState};
use ui::{
    ActiveTheme as _, Button, Card, Deck, Glide, Mode, Pin, PinKind, Pinnable as _, Skeleton, Text,
    Viewport, heading, snapped,
};

use crate::shared::album_grid::CardGrid;
use crate::shared::cells;

const PLATE: Pixels = px(260.);
const LANES: usize = 5;
const ROWS: usize = 3;
const STEADY: Pixels = px(0.5);
const PENDING: usize = 3;
const RAIL_GAP: Pixels = px(16.);
const STACK_GAP: Pixels = px(32.);
const LANE_GAP: Pixels = px(8.);
const HEADING_GAP: Pixels = px(12.);
const LEADING: f32 = 1.4;
const HEADING: Pixels = px(140.);

type Rail = (ScrollHandle, Glide);

pub(crate) struct Shelves {
    id: &'static str,
    playback: Entity<Playback>,
    rails: Vec<Rail>,
    above: Rc<Cell<Pixels>>,
}

impl Shelves {
    pub(crate) fn new(id: &'static str, playback: Entity<Playback>) -> Self {
        Self {
            id,
            playback,
            rails: Vec::new(),
            above: Rc::new(Cell::new(Pixels::ZERO)),
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
        viewport: Viewport,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        while self.rails.len() < sections.len() {
            self.rails.push((ScrollHandle::new(), Glide::default()));
        }
        for (scroll, glide) in &self.rails {
            glide.sync(scroll);
        }

        let heights: Vec<Pixels> = sections
            .iter()
            .map(|section| self.height(section, mode, width, window, cx))
            .collect();
        let sections = sections.to_vec();
        let me = cx.entity().downgrade();
        let above = self.above.clone();

        Deck::new(self.tag("stack", 0))
            .viewport(viewport)
            .rows(heights)
            .gap(STACK_GAP)
            .on_measure(move |top, _, _| above.set(top))
            .draw(move |place, _, cx| {
                let Some(view) = me.upgrade() else {
                    return div().into_any_element();
                };
                let Some(section) = sections.get(place) else {
                    return div().into_any_element();
                };
                let shelves = view.read(cx);

                match mode {
                    Mode::Cards => shelves.rail(place, section, width, &view.downgrade(), cx),
                    Mode::List => shelves.lane(place, section, width, cx),
                }
            })
            .into_any_element()
    }

    pub(crate) fn above(&self) -> Pixels {
        self.above.get()
    }

    fn height(
        &self,
        section: &GenreSection,
        mode: Mode,
        width: Pixels,
        window: &Window,
        cx: &App,
    ) -> Pixels {
        let theme = *cx.theme();
        let head = snapped(theme.text(Text::Large) * LEADING, window) + HEADING_GAP;
        let body = match mode {
            Mode::Cards => Card::tile_height(CardGrid::layout(width).card, window, cx),
            Mode::List => {
                let lanes = lanes(width);
                let rows = section.items.len().min(lanes * ROWS).div_ceil(lanes);
                let row = snapped(theme.metrics.list_row, window);
                row * rows as f32 + LANE_GAP * rows.saturating_sub(1) as f32
            }
        };

        head + body
    }

    fn lane(&self, place: usize, section: &GenreSection, width: Pixels, cx: &App) -> AnyElement {
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
        me: &WeakEntity<Self>,
        cx: &App,
    ) -> AnyElement {
        let layout = CardGrid::layout(width);
        let (handle, glide) = self.rails[place].clone();
        let crowded = section.items.len() > layout.columns;
        let seen = match handle.bounds().size.width {
            reach if reach > Pixels::ZERO => reach,
            _ => width,
        };
        let viewport = Viewport {
            top: -handle.offset().x.min(Pixels::ZERO),
            height: seen,
        };
        let items = section.items.clone();
        let drawn = me.clone();
        let card = layout.card;

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
                    .when(crowded, |this| {
                        this.child(self.arrows(place, &handle, &glide, me))
                    }),
            )
            .child(
                div()
                    .id((self.id, place))
                    .w_full()
                    .overflow_x_scroll()
                    .restrict_scroll_to_axis()
                    .track_scroll(&handle)
                    .on_scroll_wheel({
                        let scroll = handle.clone();
                        let glide = glide.clone();
                        move |event: &ScrollWheelEvent, window, _| {
                            if event.delta.precise() {
                                return;
                            }
                            glide.nudge(&scroll, window);
                        }
                    })
                    .child(
                        Deck::new(self.tag("rail", place))
                            .across()
                            .viewport(viewport)
                            .rows(items.iter().map(|_| card))
                            .gap(RAIL_GAP)
                            .draw(move |index, _, cx| {
                                let Some(view) = drawn.upgrade() else {
                                    return div().into_any_element();
                                };
                                let Some(item) = items.get(index) else {
                                    return div().into_any_element();
                                };

                                view.read(cx)
                                    .card(place * 100 + index, item, Some(card), cx)
                            }),
                    ),
            )
            .into_any_element()
    }

    fn arrows(
        &self,
        place: usize,
        handle: &ScrollHandle,
        glide: &Glide,
        me: &WeakEntity<Self>,
    ) -> AnyElement {
        let at = glide.goal(handle).x;
        let reach = handle.max_offset().x;

        div()
            .flex()
            .flex_none()
            .items_center()
            .gap_1()
            .child(
                self.arrow(self.tag("previous", place), false, handle, glide, me)
                    .disabled(at >= -STEADY),
            )
            .child(
                self.arrow(self.tag("next", place), true, handle, glide, me)
                    .disabled(reach > Pixels::ZERO && at <= STEADY - reach),
            )
            .into_any_element()
    }

    fn arrow(
        &self,
        id: impl Into<ElementId>,
        forward: bool,
        handle: &ScrollHandle,
        glide: &Glide,
        me: &WeakEntity<Self>,
    ) -> Button {
        let handle = handle.clone();
        let glide = glide.clone();
        let me = me.clone();

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
            .on_click(move |_, window, cx| {
                slide(&handle, &glide, forward, window);
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

pub(crate) fn grid(
    id: &'static str,
    genres: Vec<music::Genre>,
    width: Pixels,
    viewport: Viewport,
    window: &Window,
    cx: &App,
) -> AnyElement {
    let lanes = lanes(width);
    let row = snapped(cx.theme().metrics.list_row, window);
    let rows = genres.len().div_ceil(lanes);

    Deck::new(id)
        .viewport(viewport)
        .rows((0..rows).map(|_| row))
        .gap(LANE_GAP)
        .draw(move |place, _, cx| {
            let first = place * lanes;
            let cells = (first..(first + lanes).min(genres.len()))
                .map(|index| plate((id, index), genres[index].clone(), None, cx));

            div()
                .flex()
                .w_full()
                .gap_2()
                .children(cells.map(|cell| div().flex().flex_1().min_w_0().child(cell)))
                .into_any_element()
        })
        .into_any_element()
}

fn lanes(width: Pixels) -> usize {
    ((width / PLATE).floor().max(1.) as usize).min(LANES)
}

fn spread(cards: Vec<AnyElement>, lanes: usize) -> Div {
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

fn slide(handle: &ScrollHandle, glide: &Glide, forward: bool, window: &mut Window) {
    let page = handle.bounds().size.width;
    let at = glide.goal(handle);
    let next = match forward {
        true => at.x - page,
        false => at.x + page,
    };

    glide.aim(handle, point(next, at.y), window);
}
