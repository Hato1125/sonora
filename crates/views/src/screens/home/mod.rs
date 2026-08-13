use crate::chrome::Chrome;
use crate::shared::menu::ItemMenu;
use gpui::prelude::*;
use gpui::{Context, Entity, Pixels, Point, Render, ScrollHandle, Window, div, px};
use state::{Home, Playback};
use ui::{ActiveTheme as _, Mode, Popup, Scrollbar, Scroller, Viewport, scrolled};

use crate::shared::cells;
use crate::shared::picks::{Picks, Shape};
use crate::shared::shelves::Shelves;
use crate::shared::tracks::{PlaybackStatus, playback_status};

const STEADY: Pixels = px(0.5);

pub(crate) struct HomeView {
    home: Entity<Home>,
    playback: Entity<Playback>,
    playback_status: PlaybackStatus,
    shelves: Entity<Shelves>,
    width: Pixels,
    quick_picks_columns: usize,
    quick_picks_page: usize,
    scrollbar: Entity<Scrollbar>,
    track_menu: ItemMenu,
    context_menu: Option<(usize, Point<Pixels>)>,
}

impl HomeView {
    pub(crate) fn new(
        home: Entity<Home>,
        playback: Entity<Playback>,
        cx: &mut Context<Self>,
    ) -> Self {
        let playlist_scrollbar = cx.new(|_| {
            Scrollbar::new(ScrollHandle::new())
                .always_visible()
                .track_inset(px(4.))
        });
        let track_menu = ItemMenu::new(playlist_scrollbar);

        cx.observe(&home, |this, _, cx| {
            this.quick_picks_page = 0;
            this.track_menu.reset(cx);
            this.context_menu = None;
            cx.notify();
        })
        .detach();
        let chrome = Chrome::entity(cx);
        cx.observe(&chrome, |_, _, cx| cx.notify()).detach();

        let shelves = cx.new(|_| Shelves::new("home-shelf", playback.clone()));
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
            quick_picks_columns: 0,
            quick_picks_page: 0,
            scrollbar: cx.new(|_| Scrollbar::new(ScrollHandle::new())),
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
        let tracks = self.home.read(cx).quick_picks();
        let shape = Shape::new(available, tracks.len());
        if self.quick_picks_columns != shape.columns {
            self.quick_picks_columns = shape.columns;
            self.quick_picks_page = 0;
        }

        let pages = shape.pages;
        self.quick_picks_page = self.quick_picks_page.min(pages.saturating_sub(1));
        let page = self.quick_picks_page;
        let home = cx.entity().downgrade();
        let selected = self.context_menu.and_then(|(place, position)| {
            tracks.get(place).cloned().map(|track| (track, position))
        });
        let context_menu = selected.map(|(track, position)| {
            Popup::new(position, self.track_menu.for_track(&track, cx)).on_close(cx.listener(
                |this, _, _, cx| {
                    this.context_menu = None;
                    cx.notify();
                },
            ))
        });

        let sections = self.home.read(cx).sections().to_vec();
        let feeding = self.home.read(cx).is_feeding();
        let width = self.width;
        let scroll = self.scrollbar.read(cx).scroll().clone();
        let above = self.shelves.read(cx).above();
        let seen = scroll.bounds().size.height;
        let viewport = Viewport {
            top: (scrolled(&scroll) - above).max(Pixels::ZERO),
            height: match seen > Pixels::ZERO {
                true => seen,
                false => window.viewport_size().height,
            },
        };
        let shelves = self
            .shelves
            .update(cx, |shelves, cx| match sections.is_empty() && feeding {
                true => shelves.pending(width, cx),
                false => vec![shelves.render(&sections, Mode::Cards, width, viewport, window, cx)],
            });

        Scroller::new("home-page", &self.scrollbar)
            .p(theme.metrics.inset)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_8()
                    .child(
                        Picks::new(
                            "quick-pick",
                            tracks,
                            self.playback.clone(),
                            self.playback_status.0.clone(),
                            available,
                            page,
                        )
                        .title("home-quick-picks")
                        .eyebrow("home-quick-picks-eyebrow")
                        .vacancy("home-quick-picks-empty")
                        .loading(self.home.read(cx).is_loading(cx))
                        .on_previous(cx.listener(|this, _, _, cx| {
                            this.quick_picks_page = this.quick_picks_page.saturating_sub(1);
                            this.context_menu = None;
                            cx.notify();
                        }))
                        .on_next(cx.listener(move |this, _, _, cx| {
                            this.quick_picks_page =
                                (this.quick_picks_page + 1).min(pages.saturating_sub(1));
                            this.context_menu = None;
                            cx.notify();
                        }))
                        .on_context_menu(move |place, event, _, cx| {
                            let Some(home) = home.upgrade() else {
                                return;
                            };
                            home.update(cx, |this, cx| {
                                this.track_menu.reset(cx);
                                this.context_menu = Some((place, event.position));
                                cx.notify();
                            });
                        }),
                    )
                    .children(shelves),
            )
            .when_some(context_menu, |this, menu| this.child(menu))
    }
}
