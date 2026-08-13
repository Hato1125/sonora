use gpui::prelude::*;
use gpui::{
    AnyElement, App, Context, Div, ElementId, Entity, FontWeight, Pixels, Render, ScrollHandle,
    SharedString, WeakEntity, Window, div, point, px,
};
use i18n::t;
use music::{Album, GenreItem, GenreSection, Playlist};
use router::{Destination, navigate};
use state::{AppSettings, GenreDetails, Origin, Playback, PlaybackState, Sonora};
use ui::{
    ActiveTheme as _, Button, Card, Mode, Pin, PinKind, Pinnable as _, Popovers, Scrollbar,
    Scroller, Skeleton, Text, heading, vacant,
};

use crate::chrome::{Chrome, Toolbar, Tooled, tools};
use crate::shared::album_grid::CardGrid;
use crate::shared::cells;

const TILE: Pixels = px(220.);
const PLATE: Pixels = px(260.);
const LANES: usize = 5;
const ROWS: usize = 3;
const STEADY: Pixels = px(0.5);
const SECTION: &str = "genre";

pub(crate) struct GenreView {
    detail: Entity<GenreDetails>,
    playback: Entity<Playback>,
    settings: Entity<AppSettings>,
    scrollbar: Entity<Scrollbar>,
    toolbar: Entity<Toolbar>,
    popovers: Popovers,
    mode: Mode,
    width: Pixels,
    rails: Vec<ScrollHandle>,
    me: WeakEntity<Self>,
}

impl GenreView {
    pub(crate) fn new(
        detail: Entity<GenreDetails>,
        playback: Entity<Playback>,
        cx: &mut Context<Self>,
    ) -> Self {
        let settings = Sonora::global(cx).settings.clone();
        let mode = settings.read(cx).view_or(SECTION, Mode::Cards);

        cx.observe(&detail, |this, _, cx| {
            this.rails.clear();
            cx.notify();
        })
        .detach();
        cx.observe(&playback, |_, _, cx| cx.notify()).detach();
        let chrome = Chrome::entity(cx);
        cx.observe(&chrome, |_, _, cx| cx.notify()).detach();

        let me = cx.entity();
        let toolbar = cx.new(|cx| {
            let mut toolbar = Toolbar::new(cx);
            toolbar.wire(&me, cx);
            toolbar
        });

        Self {
            detail,
            playback,
            settings,
            scrollbar: cx.new(|_| Scrollbar::new(ScrollHandle::new())),
            toolbar,
            popovers: Popovers::default(),
            mode,
            width: Pixels::ZERO,
            rails: Vec::new(),
            me: me.downgrade(),
        }
    }

    fn set_mode(&mut self, mode: Mode, cx: &mut Context<Self>) {
        self.mode = mode;
        self.settings
            .update(cx, |settings, cx| settings.set_view(SECTION, mode, cx));
        cx.notify();
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

        Card::new(("genre-playlist", id), SharedString::from(playlist.name))
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
            .map(|card| dressed(card, tile, cx))
            .play(playing, move |_, _, cx| {
                playback.update(cx, |playback, cx| playback.toggle_origin(&origin, cx));
            })
            .press(move |_, _, cx| navigate(Destination::Album(opened.clone()), cx))
            .pin(pin)
            .into_any_element()
    }

    fn card(&self, id: usize, item: GenreItem, tile: Option<Pixels>, cx: &App) -> AnyElement {
        match item {
            GenreItem::Playlist(playlist) => self.playlist_card(id, playlist, tile, cx),
            GenreItem::Album(album) => self.album_card(id, album, tile, cx),
            GenreItem::Genre(genre) => plate(("genre-plate", id), genre, tile, cx),
        }
    }

    fn sections(&mut self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let sections = self.detail.read(cx).sections().to_vec();
        while self.rails.len() < sections.len() {
            self.rails.push(ScrollHandle::new());
        }

        sections
            .into_iter()
            .enumerate()
            .map(|(place, section)| match self.mode {
                Mode::Cards => self.rail(place, section, cx),
                Mode::List => self.lane(place, section, cx),
            })
            .collect()
    }

    fn lane(&self, place: usize, section: GenreSection, cx: &App) -> AnyElement {
        let lanes = lanes(self.width);
        let cards = section
            .items
            .into_iter()
            .take(lanes * ROWS)
            .enumerate()
            .map(|(index, item)| self.card(place * 100 + index, item, None, cx))
            .collect();

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(heading(SharedString::from(section.title), cx))
            .child(spread(cards, lanes))
            .into_any_element()
    }

    fn rail(&self, place: usize, section: GenreSection, cx: &App) -> AnyElement {
        let layout = CardGrid::layout(self.width);
        let handle = self.rails[place].clone();
        let crowded = section.items.len() > layout.columns;
        let cards: Vec<AnyElement> = section
            .items
            .into_iter()
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
                    .child(heading(SharedString::from(section.title), cx))
                    .when(crowded, |this| this.child(self.arrows(place, &handle, cx))),
            )
            .child(
                div()
                    .id(("genre-rail", place))
                    .flex()
                    .w_full()
                    .gap_4()
                    .overflow_x_scroll()
                    .track_scroll(&handle)
                    .children(cards),
            )
            .into_any_element()
    }

    fn arrows(&self, place: usize, handle: &ScrollHandle, cx: &App) -> AnyElement {
        let at = handle.offset().x;
        let reach = handle.max_offset().x;

        div()
            .flex()
            .flex_none()
            .items_center()
            .gap_1()
            .child(
                self.arrow(("genre-previous", place), false, handle, cx)
                    .disabled(at >= -STEADY),
            )
            .child(
                self.arrow(("genre-next", place), true, handle, cx)
                    .disabled(reach > Pixels::ZERO && at <= STEADY - reach),
            )
            .into_any_element()
    }

    fn arrow(
        &self,
        id: impl Into<ElementId>,
        forward: bool,
        handle: &ScrollHandle,
        _cx: &App,
    ) -> Button {
        let handle = handle.clone();
        let me = self.me.clone();

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
}

impl Tooled for GenreView {
    fn toolbar(&self) -> Entity<Toolbar> {
        self.toolbar.clone()
    }

    fn tools(&self, _cx: &App) -> Vec<AnyElement> {
        let viewed = self.me.clone();

        vec![tools::views(&self.popovers, self.mode, move |mode, cx| {
            viewed.update(cx, |view, cx| view.set_mode(mode, cx)).ok();
        })]
    }
}

impl Render for GenreView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.theme();
        let pad = theme.metrics.inset;
        let room = cells::content_width(window, pad * 2., cx);
        if (room - self.width).abs() >= STEADY {
            self.width = room;
        }

        let detail = self.detail.read(cx);
        let loading = detail.is_loading();
        let title = detail.name().unwrap_or_default().to_owned();
        let error = detail.error().map(str::to_owned);
        let empty = !loading && detail.sections().is_empty();
        let sections = self.sections(cx);

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
