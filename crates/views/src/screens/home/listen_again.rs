use std::rc::Rc;

use gpui::prelude::*;
use gpui::{App, ClickEvent, Entity, MouseDownEvent, Pixels, SharedString, Window, div};
use music::Track;
use state::{Playback, PlaybackState};
use ui::{ActiveTheme as _, Button, Card, Pinnable, Text, heading};

use crate::shared::album_grid::{CardGrid, CardLayout};
use crate::shared::cells;
use crate::shared::pins::Pinned as _;

type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;
type ContextHandler = Rc<dyn Fn(usize, &MouseDownEvent, &mut Window, &mut App)>;

#[derive(Clone, Copy)]
pub(super) struct Shape {
    pub(super) columns: usize,
    pub(super) pages: usize,
}

impl Shape {
    pub(super) fn new(width: Pixels, count: usize) -> Self {
        let columns = CardGrid::layout(width).columns;

        Self {
            columns,
            pages: count.div_ceil(columns).max(1),
        }
    }
}

#[derive(IntoElement)]
pub(super) struct ListenAgain {
    tracks: Rc<Vec<Track>>,
    playback: Entity<Playback>,
    active: Option<String>,
    width: Pixels,
    page: usize,
    on_previous: Option<ClickHandler>,
    on_next: Option<ClickHandler>,
    on_context_menu: Option<ContextHandler>,
}

impl ListenAgain {
    pub(super) fn new(
        tracks: Rc<Vec<Track>>,
        playback: Entity<Playback>,
        active: Option<String>,
        width: Pixels,
        page: usize,
    ) -> Self {
        Self {
            tracks,
            playback,
            active,
            width,
            page,
            on_previous: None,
            on_next: None,
            on_context_menu: None,
        }
    }

    pub(super) fn on_previous(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_previous = Some(Rc::new(handler));
        self
    }

    pub(super) fn on_next(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_next = Some(Rc::new(handler));
        self
    }

    pub(super) fn on_context_menu(
        mut self,
        handler: impl Fn(usize, &MouseDownEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_context_menu = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for ListenAgain {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let layout = CardGrid::layout(self.width);
        let shape = Shape::new(self.width, self.tracks.len());
        let page = self.page.min(shape.pages.saturating_sub(1));
        let start = page * shape.columns;
        let tracks = self.tracks;
        let cards = tracks
            .iter()
            .enumerate()
            .skip(start)
            .take(shape.columns)
            .map(|(place, track)| {
                card(
                    track,
                    place,
                    layout,
                    tracks.clone(),
                    self.playback.clone(),
                    self.active.as_deref(),
                    self.on_context_menu.clone(),
                    cx,
                )
                .into_any_element()
            });

        div()
            .w_full()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .items_end()
                    .justify_between()
                    .gap_4()
                    .child(heading(i18n::lookup("home-listen-again", None), cx))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                Button::new("listen-again-previous")
                                    .small()
                                    .outline()
                                    .icon("icons/chevron-left.svg")
                                    .tooltip("common-previous")
                                    .disabled(page == 0)
                                    .when_some(self.on_previous, |button, handler| {
                                        button.on_click(move |event, window, cx| {
                                            handler(event, window, cx)
                                        })
                                    }),
                            )
                            .child(
                                Button::new("listen-again-next")
                                    .small()
                                    .outline()
                                    .icon("icons/chevron-right.svg")
                                    .tooltip("common-next")
                                    .disabled(page + 1 >= shape.pages)
                                    .when_some(self.on_next, |button, handler| {
                                        button.on_click(move |event, window, cx| {
                                            handler(event, window, cx)
                                        })
                                    }),
                            ),
                    ),
            )
            .child(CardGrid::new(self.width).children(cards))
    }
}

#[allow(clippy::too_many_arguments)]
fn card(
    track: &Track,
    place: usize,
    layout: CardLayout,
    tracks: Rc<Vec<Track>>,
    playback: Entity<Playback>,
    active: Option<&str>,
    on_context_menu: Option<ContextHandler>,
    cx: &App,
) -> Card {
    let theme = *cx.theme();
    let current = track.id.as_deref() == active;
    let playing = current && playback.read(cx).state() == &PlaybackState::Playing;
    let pin = track.pin();
    let artists = cells::artist_links(
        SharedString::new_static("listen-again-artist"),
        track.artist_refs.clone(),
        track.artists.clone(),
        theme.muted_foreground,
    )
    .text_size(theme.text(Text::Small))
    .truncate();
    let pressed_tracks = tracks.clone();
    let pressed_playback = playback.clone();
    let transport_tracks = tracks.clone();
    let transport_playback = playback.clone();

    Card::new(
        ("listen-again-card", place),
        SharedString::from(track.name.clone()),
    )
    .cover(track.cover.clone())
    .tile(layout.card)
    .flat()
    .weight(gpui::FontWeight::SEMIBOLD)
    .hint()
    .bare_meta(artists)
    .when(track.explicit, Card::explicit)
    .when_some(on_context_menu, |card, handler| {
        card.menu(move |event, window, cx| handler(place, event, window, cx))
    })
    .play(playing, move |_, _, cx| match current {
        true => transport_playback.update(cx, |playback, cx| playback.toggle_play(cx)),
        false => transport_playback.update(cx, |playback, cx| {
            playback.play_radio(&transport_tracks[place], cx);
        }),
    })
    .press(move |_, _, cx| {
        pressed_playback.update(cx, |playback, cx| {
            playback.play_radio(&pressed_tracks[place], cx);
        });
    })
    .when_some(pin, Pinnable::pin)
}
