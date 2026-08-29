mod listen_again;

use crate::chrome::Chrome;
use crate::shared::menu::ItemMenu;
use gpui::prelude::*;
use gpui::{Context, Entity, Pixels, Point, Render, ScrollHandle, Window, div, px};
use music::Track;
use state::{Home, Playback, Sonora};
use ui::{ActiveTheme as _, Mode, Popup, Scrollbar, Scroller};

use crate::shared::cells;
use crate::shared::picks::{Picks, Shape};
use crate::shared::shelves::Shelves;
use crate::shared::tracks::{PlaybackStatus, playback_status};
use listen_again::{ListenAgain, Shape as ListenAgainShape};

const STEADY: Pixels = px(0.5);

pub(crate) struct HomeView {
    home: Entity<Home>,
    playback: Entity<Playback>,
    playback_status: PlaybackStatus,
    shelves: Entity<Shelves>,
    width: Pixels,
    listen_again_columns: usize,
    listen_again_page: usize,
    quick_picks_columns: usize,
    quick_picks_page: usize,
    scrollbar: Entity<Scrollbar>,
    track_menu: ItemMenu,
    context_menu: Option<(Track, Point<Pixels>)>,
}

impl HomeView {
    pub(crate) fn new(
        home: Entity<Home>,
        playback: Entity<Playback>,
        cx: &mut Context<Self>,
    ) -> Self {
        let me = cx.entity_id();
        let playlist_scrollbar = cx.new(|_| Scrollbar::inset().watching(me));
        let track_menu = ItemMenu::new(playlist_scrollbar);

        cx.observe(&home, |this, _, cx| {
            this.listen_again_page = 0;
            this.quick_picks_page = 0;
            this.track_menu.reset(cx);
            this.context_menu = None;
            cx.notify();
        })
        .detach();
        let chrome = Chrome::entity(cx);
        cx.observe(&chrome, |_, _, cx| cx.notify()).detach();

        let library = Sonora::global(cx).library.clone();
        cx.observe(&library, |_, _, cx| cx.notify()).detach();

        let shelves = cx.new(|_| Shelves::new("home-shelf", me, playback.clone()));
        cx.observe(&shelves, |_, _, cx| cx.notify()).detach();

        let current_playback = playback_status(&playback, cx);
        cx.observe(&playback, |this, playback, cx| {
            let current = playback_status(&playback, cx);
            if this.playback_status != current {
                this.playback_status = current;
                cx.notify();
            }
        })
        .detach();

        Self {
            home,
            playback,
            playback_status: current_playback,
            shelves,
            width: Pixels::ZERO,
            listen_again_columns: 0,
            listen_again_page: 0,
            quick_picks_columns: 0,
            quick_picks_page: 0,
            scrollbar: cx.new(|_| Scrollbar::new(ScrollHandle::new()).watching(me)),
            track_menu,
            context_menu: None,
        }
    }
}

impl Render for HomeView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.theme();
        let available = cells::content_width(window, theme.metrics.inset * 2., cx);
        if (available - self.width).abs() >= STEADY {
            self.width = available;
        }
        let listen_again = self.home.read(cx).listen_again();
        let listen_shape = ListenAgainShape::new(available, listen_again.len());
        if self.listen_again_columns != listen_shape.columns {
            self.listen_again_columns = listen_shape.columns;
            self.listen_again_page = 0;
        }
        let listen_pages = listen_shape.pages;
        self.listen_again_page = self.listen_again_page.min(listen_pages.saturating_sub(1));
        let listen_page = self.listen_again_page;

        let quick_picks = self.home.read(cx).quick_picks();
        let quick_picks_paged = self.home.read(cx).quick_picks_paged();
        let quick_shape = Shape::new(available, quick_picks.len());
        if self.quick_picks_columns != quick_shape.columns {
            self.quick_picks_columns = quick_shape.columns;
            self.quick_picks_page = 0;
        }
        let quick_pages = quick_shape.pages;
        self.quick_picks_page = self.quick_picks_page.min(quick_pages.saturating_sub(1));
        let quick_page = self.quick_picks_page;
        let context_menu = self.context_menu.clone().map(|(track, position)| {
            Popup::new(position, self.track_menu.for_track(&track, cx)).on_close(cx.listener(
                |this, _, _, cx| {
                    this.context_menu = None;
                    cx.notify();
                },
            ))
        });

        let listen = (!listen_again.is_empty()).then(|| {
            let tracks = listen_again.clone();
            let home = cx.entity().downgrade();
            ListenAgain::new(
                listen_again,
                self.playback.clone(),
                self.playback_status.0.clone(),
                available,
                listen_page,
            )
            .on_previous(cx.listener(|this, _, _, cx| {
                this.listen_again_page = this.listen_again_page.saturating_sub(1);
                this.context_menu = None;
                cx.notify();
            }))
            .on_next(cx.listener(move |this, _, _, cx| {
                this.listen_again_page =
                    (this.listen_again_page + 1).min(listen_pages.saturating_sub(1));
                this.context_menu = None;
                cx.notify();
            }))
            .on_context_menu(move |place, event, _, cx| {
                let Some(track) = tracks.get(place).cloned() else {
                    return;
                };
                let Some(home) = home.upgrade() else {
                    return;
                };
                home.update(cx, |this, cx| {
                    this.track_menu.reset(cx);
                    this.context_menu = Some((track, event.position));
                    cx.notify();
                });
            })
        });

        let tracks = quick_picks.clone();
        let home = cx.entity().downgrade();
        let quick = Picks::new(
            "quick-pick",
            quick_picks,
            self.playback.clone(),
            self.playback_status.0.clone(),
            available,
            quick_page,
        )
        .title("home-quick-picks")
        .eyebrow("home-quick-picks-eyebrow")
        .vacancy("home-quick-picks-empty")
        .loading(self.home.read(cx).is_loading(cx))
        .paged(quick_picks_paged)
        .on_previous(cx.listener(|this, _, _, cx| {
            this.quick_picks_page = this.quick_picks_page.saturating_sub(1);
            this.context_menu = None;
            cx.notify();
        }))
        .on_next(cx.listener(move |this, _, _, cx| {
            this.quick_picks_page = (this.quick_picks_page + 1).min(quick_pages.saturating_sub(1));
            this.context_menu = None;
            cx.notify();
        }))
        .on_context_menu(move |place, event, _, cx| {
            let Some(track) = tracks.get(place).cloned() else {
                return;
            };
            let Some(home) = home.upgrade() else {
                return;
            };
            home.update(cx, |this, cx| {
                this.track_menu.reset(cx);
                this.context_menu = Some((track, event.position));
                cx.notify();
            });
        });

        let sections = self.home.read(cx).sections();
        let feeding = self.home.read(cx).is_feeding();
        let width = self.width;
        let scroll = self.scrollbar.read(cx).scroll().clone();
        let viewport = self.shelves.read(cx).viewport(&scroll, window);
        let shelves = self
            .shelves
            .update(cx, |shelves, cx| match sections.is_empty() && feeding {
                true => shelves.pending(width, cx),
                false => vec![shelves.render(sections, Mode::Cards, width, viewport, window, cx)],
            });

        Scroller::new("home-page", &self.scrollbar)
            .p(theme.metrics.inset)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_8()
                    .children(listen)
                    .child(quick)
                    .children(shelves),
            )
            .when_some(context_menu, |this, menu| this.child(menu))
    }
}
