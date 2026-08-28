use gpui::prelude::*;
use gpui::{
    AnyElement, App, Context, Entity, Pixels, Render, ScrollHandle, SharedString, Window, div,
};
use i18n::t;
use music::Track;
use state::{History, HistoryState, Playback};
use ui::{
    ActiveTheme as _, GridDelegate, GridEvent, GridState, Scrollbar, Scroller, Table as _, grid,
    vacant,
};

use crate::chrome::{Searchable, Toolbar, Tooled};
use crate::shared::cells;
use crate::shared::page;
use crate::shared::tracks::{HISTORY_COLUMNS, TrackSource, Tracks};

struct HistoryTracks(Entity<History>);

impl Tracks for HistoryTracks {
    fn tracks<'a>(&self, cx: &'a App) -> &'a [Track] {
        self.0.read(cx).tracks()
    }

    fn is_loading(&self, cx: &App) -> bool {
        matches!(self.0.read(cx).state(), HistoryState::Loading)
    }
}

pub(crate) struct HistoryView {
    history: Entity<History>,
    playback: Entity<Playback>,
    width: Pixels,
    scrollbar: Entity<Scrollbar>,
    table: Entity<GridState<TrackSource>>,
    toolbar: Entity<Toolbar>,
}

impl HistoryView {
    pub(crate) fn new(
        history: Entity<History>,
        playback: Entity<Playback>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let width = cells::content_width(window, Pixels::ZERO, cx);
        let id = cx.entity_id();
        let scrollbar = cx.new(|_| Scrollbar::new(ScrollHandle::new()).watching(id));
        let scroll = scrollbar.read(cx).scroll().clone();
        let table = cx.new(|cx| {
            let menu = cx.new(|_| Scrollbar::inset().watching(id));
            let source = TrackSource::new(
                HISTORY_COLUMNS,
                HistoryTracks(history.clone()),
                playback.clone(),
                menu,
            )
            .table(cx.weak_entity());
            GridState::new(GridDelegate::new(source, width, cx), cx).follow(scroll)
        });

        cx.observe(&history, |this, _, cx| {
            this.table.rebuild(cx);
            cx.notify();
        })
        .detach();
        cx.observe(&playback, |this, _, cx| {
            this.table.refresh(cx);
            cx.notify();
        })
        .detach();
        cx.subscribe(&table, |this, _, event, cx| {
            if let GridEvent::DoubleClicked(display) = event {
                page::play(&this.table, &this.playback, *display, cx);
            }
        })
        .detach();

        let toolbar = Toolbar::searchable(&cx.entity(), cx);
        Self {
            history,
            playback,
            width,
            scrollbar,
            table,
            toolbar,
        }
    }

    pub(crate) fn refresh(&mut self, cx: &mut Context<Self>) {
        self.history.update(cx, |history, cx| history.refresh(cx));
    }

    fn note(&self, cx: &App) -> Option<SharedString> {
        let history = self.history.read(cx);
        match history.state() {
            HistoryState::Loading => None,
            HistoryState::Failed => Some(t!("history-not-loaded")),
            _ if self.table.row_count(cx) > 0 => None,
            _ if self.table.filtering(cx) => Some(t!("library-no-matches")),
            _ => Some(t!("history-empty")),
        }
    }
}

impl Render for HistoryView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let inset = cx.theme().metrics.inset;
        let width = cells::content_width(window, Pixels::ZERO, cx);
        if (width - self.width).abs() >= gpui::px(0.5) {
            self.width = width;
            self.table.set_width(width, cx);
        }

        let scroll = self.scrollbar.read(cx).scroll().clone();
        let viewport = page::viewport(&scroll, inset, window);
        self.table
            .update(cx, |table, _| table.set_viewport(viewport));

        let note = self.note(cx);
        let page = Scroller::new("history-page", &self.scrollbar)
            .pt(inset)
            .pb(inset)
            .child(grid(&self.table))
            .when_some(note, |this, note| this.child(vacant(note, cx)));

        div().size_full().child(page)
    }
}

impl Searchable for HistoryView {
    fn search(&mut self, query: &str, cx: &mut Context<Self>) {
        self.table.set_filter(query, cx);
        cx.notify();
    }

    fn hint() -> SharedString {
        "filter-history".into()
    }
}

impl Tooled for HistoryView {
    fn toolbar(&self) -> Entity<Toolbar> {
        self.toolbar.clone()
    }

    fn tools(&self, _cx: &App) -> Vec<AnyElement> {
        Vec::new()
    }
}
