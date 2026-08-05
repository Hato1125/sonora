use gpui::prelude::*;
use gpui::{
    AnyElement, App, Context, Entity, FontWeight, Pixels, Render, ScrollHandle, SharedString,
    Window, div, px,
};
use spotify::{Album, Track};
use state::{AlbumDetail, Playback};
use ui::ActiveTheme as _;
use ui::{Artwork, GridDelegate, GridEvent, GridState, Viewport, grid, scrollbar, scrolled};

use crate::cells;
use crate::tracks::{ALBUM_COLUMNS, TrackSource, Tracks};
use workspace::{Navigation, Sidebar};

const PADDING: Pixels = px(24.);
const INSET: Pixels = px(50.);
const COVER: Pixels = px(140.);
const FRAME: Pixels = px(1.);

struct DetailTracks(Entity<AlbumDetail>);

impl Tracks for DetailTracks {
    fn tracks<'a>(&self, cx: &'a App) -> &'a [Track] {
        self.0.read(cx).tracks()
    }

    fn is_loading(&self, cx: &App) -> bool {
        self.0.read(cx).is_loading()
    }
}

pub struct AlbumView {
    detail: Entity<AlbumDetail>,
    playback: Entity<Playback>,
    sidebar: Entity<Sidebar>,
    width: Pixels,
    scroll: ScrollHandle,
    table: Entity<GridState<TrackSource>>,
}

impl AlbumView {
    pub fn new(
        detail: Entity<AlbumDetail>,
        playback: Entity<Playback>,
        sidebar: Entity<Sidebar>,
        navigation: Entity<Navigation>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let width = cells::content_width(window, sidebar.read(cx).occupied_width(), INSET);

        let table = cx.new(|cx| {
            let source = TrackSource::new(
                ALBUM_COLUMNS,
                DetailTracks(detail.clone()),
                playback.clone(),
                navigation.clone(),
            );
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

        Self {
            detail,
            playback,
            sidebar,
            width,
            scroll: ScrollHandle::new(),
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
        let album = self.detail.read(cx).album();
        let name = album
            .map(|album| SharedString::from(album.name.clone()))
            .unwrap_or_else(|| SharedString::from("Album"));

        div()
            .flex()
            .flex_none()
            .items_end()
            .gap_5()
            .pb_6()
            .child(
                Artwork::new(album.and_then(|album| album.cover_large.clone()))
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
                            .child("ALBUM"),
                    )
                    .child(
                        div()
                            .text_size(px(28.))
                            .font_weight(FontWeight::BOLD)
                            .truncate()
                            .child(name),
                    )
                    .child(div().text_color(muted).truncate().children(album.map(meta))),
            )
            .into_any_element()
    }
}

fn meta(album: &Album) -> SharedString {
    let mut parts = vec![album.artists.clone()];
    if album.year > 0 {
        parts.push(format!("{}", album.year));
    }
    if album.track_count > 0 {
        parts.push(format!("{} songs", album.track_count));
    }
    SharedString::from(parts.join(" • "))
}

impl Render for AlbumView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.resize(window, cx);
        let viewport = self.viewport(window);
        self.table
            .update(cx, |table, _| table.set_viewport(viewport));

        div()
            .relative()
            .size_full()
            .child(
                div()
                    .id("album-page")
                    .size_full()
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .px(PADDING)
                    .pt(PADDING)
                    .pb(PADDING)
                    .child(self.header(cx))
                    .child(
                        div()
                            .border_1()
                            .border_color(cx.theme().border)
                            .child(grid(&self.table)),
                    ),
            )
            .children(scrollbar(&self.scroll, cx))
    }
}
