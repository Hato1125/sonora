use gpui::prelude::*;
use gpui::{
    AnyElement, App, Context, Entity, FontWeight, Pixels, Render, ScrollHandle, SharedString,
    Window, div, px,
};
use spotify::Track;
use state::{Detail, Playback};
use ui::ActiveTheme as _;
use ui::{
    Artwork, ColumnSpec, GridDelegate, GridEvent, GridState, Scrollbar, Viewport, grid, scrolled,
};

use crate::cells;
use crate::tracks::{TrackField, TrackSource, Tracks};
use workspace::Sidebar;

const PADDING: Pixels = px(24.);
const INSET: Pixels = px(50.);
const COVER: Pixels = px(140.);
const FRAME: Pixels = px(1.);

struct DetailTracks(Entity<Detail>);

impl Tracks for DetailTracks {
    fn tracks<'a>(&self, cx: &'a App) -> &'a [Track] {
        self.0.read(cx).tracks()
    }

    fn is_loading(&self, cx: &App) -> bool {
        self.0.read(cx).is_loading()
    }
}

pub(crate) struct DetailView {
    detail: Entity<Detail>,
    playback: Entity<Playback>,
    sidebar: Entity<Sidebar>,
    width: Pixels,
    scroll: ScrollHandle,
    scrollbar: Entity<Scrollbar>,
    table: Entity<GridState<TrackSource>>,
}

impl DetailView {
    pub(crate) fn new(
        detail: Entity<Detail>,
        playback: Entity<Playback>,
        sidebar: Entity<Sidebar>,
        columns: &'static [ColumnSpec<TrackField>],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let width = cells::content_width(window, sidebar.read(cx).occupied_width(), INSET);

        let table = cx.new(|cx| {
            let source = TrackSource::new(columns, DetailTracks(detail.clone()), playback.clone());
            GridState::new(GridDelegate::new(source, width, cx))
        });

        cx.observe(&detail, |this, _, cx| {
            this.scroll.set_offset(gpui::Point::default());
            this.rebuild(cx);
            cx.notify();
        })
        .detach();

        cx.observe(&sidebar, |_, _, cx| cx.notify()).detach();

        cx.observe(&playback, |this, _, cx| {
            this.table.update(cx, |table, cx| table.refresh(cx));
        })
        .detach();

        cx.subscribe(&table, |this, _, event, cx| {
            let GridEvent::DoubleClicked(display) = event;
            this.play(*display, cx);
        })
        .detach();

        let scroll = ScrollHandle::new();
        let scrollbar = cx.new(|_| Scrollbar::new(scroll.clone()));

        Self {
            detail,
            playback,
            sidebar,
            width,
            scroll,
            scrollbar,
            table,
        }
    }

    fn play(&mut self, display: usize, cx: &mut Context<Self>) {
        let queued = {
            let state = self.table.read(cx);
            let delegate = state.delegate();
            (0..delegate.row_count())
                .filter_map(|row| delegate.source().at(delegate.row(row), cx))
                .collect::<Vec<_>>()
        };
        self.playback
            .update(cx, |playback, cx| playback.start(queued, display, cx));
    }

    fn resize(&mut self, window: &Window, cx: &mut Context<Self>) {
        let sidebar = self.sidebar.read(cx).occupied_width();
        let width = cells::content_width(window, sidebar, INSET);
        if (width - self.width).abs() < px(0.5) {
            return;
        }
        self.width = width;
        self.table.update(cx, |table, cx| {
            table.delegate_mut().set_width(width);
            table.refresh(cx);
        });
    }

    fn viewport(&self, window: &Window) -> Viewport {
        let hero = self
            .scroll
            .bounds_for_item(0)
            .map(|bounds| bounds.size.height)
            .unwrap_or_default();
        let visible = self.scroll.bounds().size.height;

        Viewport {
            top: (scrolled(&self.scroll) - PADDING - hero - FRAME).max(Pixels::ZERO),
            height: match visible > Pixels::ZERO {
                true => visible,
                false => window.viewport_size().height,
            },
        }
    }

    fn rebuild(&mut self, cx: &mut Context<Self>) {
        self.table.update(cx, |table, cx| {
            table.delegate_mut().rebuild(cx);
            table.refresh(cx);
        });
    }

    fn header(&self, cx: &Context<Self>) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let header = self.detail.read(cx).header();
        let kind = header.map(|header| header.kind).unwrap_or_default();
        let title = header
            .map(|header| SharedString::from(header.title.clone()))
            .unwrap_or_default();
        let meta = header.map(|header| SharedString::from(header.meta.clone()));

        div()
            .flex()
            .flex_none()
            .items_end()
            .gap_5()
            .pb_6()
            .child(
                Artwork::new(header.and_then(|header| header.cover.clone()))
                    .size(COVER)
                    .rounded(px(8.)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .min_w_0()
                    .gap_2()
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(muted)
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(kind),
                    )
                    .child(
                        div()
                            .text_size(px(28.))
                            .font_weight(FontWeight::BOLD)
                            .truncate()
                            .child(title),
                    )
                    .child(div().text_color(muted).truncate().children(meta)),
            )
            .into_any_element()
    }
}

impl Render for DetailView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.resize(window, cx);
        let viewport = self.viewport(window);
        self.table
            .update(cx, |table, _| table.set_viewport(viewport));

        div()
            .size_full()
            .child(
                div()
                    .id("detail-page")
                    .size_full()
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .px(PADDING)
                    .pt(PADDING)
                    .pb(PADDING)
                    .child(self.header(cx))
                    .child(
                        grid(&self.table)
                            .rounded_xl()
                            .border_1()
                            .border_color(cx.theme().border),
                    ),
            )
            .child(self.scrollbar.clone())
    }
}
