use gpui::prelude::*;
use gpui::{Context, Entity, Render, Window, div};
use state::{Home, Playback};
use ui::ActiveTheme as _;
use workspace::Sidebar;

use crate::quick_picks::{QuickPicks, column_count, page_count};
use crate::tracks::{PlaybackStatus, playback_status};

pub(crate) struct HomeView {
    home: Entity<Home>,
    playback: Entity<Playback>,
    playback_status: PlaybackStatus,
    quick_picks_columns: usize,
    quick_picks_page: usize,
    sidebar: Entity<Sidebar>,
}

impl HomeView {
    pub(crate) fn new(
        home: Entity<Home>,
        playback: Entity<Playback>,
        sidebar: Entity<Sidebar>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&home, |this, _, cx| {
            this.quick_picks_page = 0;
            cx.notify();
        })
        .detach();
        cx.observe(&sidebar, |_, _, cx| cx.notify()).detach();

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
            quick_picks_columns: 0,
            quick_picks_page: 0,
            sidebar,
        }
    }
}

impl Render for HomeView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.theme();
        let available = window.viewport_size().width
            - self.sidebar.read(cx).occupied_width()
            - theme.metrics.inset * 2.;
        let columns = column_count(available);
        if self.quick_picks_columns != columns {
            self.quick_picks_columns = columns;
            self.quick_picks_page = 0;
        }

        let tracks = self.home.read(cx).quick_picks();
        let pages = page_count(tracks.len(), available);
        self.quick_picks_page = self.quick_picks_page.min(pages.saturating_sub(1));
        let page = self.quick_picks_page;

        div()
            .id("home-page")
            .size_full()
            .overflow_y_scroll()
            .p(theme.metrics.inset)
            .child(
                div().flex().flex_col().gap_6().child(
                    QuickPicks::new(
                        tracks,
                        self.playback.clone(),
                        self.playback_status.0.clone(),
                        available,
                        page,
                    )
                    .on_previous(cx.listener(|this, _, _, cx| {
                        this.quick_picks_page = this.quick_picks_page.saturating_sub(1);
                        cx.notify();
                    }))
                    .on_next(cx.listener(move |this, _, _, cx| {
                        this.quick_picks_page =
                            (this.quick_picks_page + 1).min(pages.saturating_sub(1));
                        cx.notify();
                    })),
                ),
            )
    }
}
