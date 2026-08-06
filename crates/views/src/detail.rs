use gpui::prelude::*;
use gpui::{
    AnyElement, App, Context, Entity, FontWeight, Pixels, Render, ScrollHandle, SharedString,
    Window, div, px,
};

use spotify::Track;
use state::{Detail, Playback};
use ui::ActiveTheme as _;
use ui::{
    Artwork, ColumnSpec, GridDelegate, GridEvent, GridState, Scrollbar, Text, Viewport, grid,
    scrolled,
};

use crate::cells;
use crate::tracks::{PlaybackStatus, TrackField, TrackSource, Tracks, playback_status};
use workspace::{Searchable, Sidebar};

const FRAME: Pixels = px(1.);

fn reserved(inset: Pixels) -> Pixels {
    inset * 2. + px(2.)
}

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
    playback_status: PlaybackStatus,
    sidebar: Entity<Sidebar>,
    width: Pixels,
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
        let inset = cx.theme().metrics.inset;
        let width =
            cells::content_width(window, sidebar.read(cx).occupied_width(), reserved(inset));

        let scrollbar = cx.new(|_| Scrollbar::new(ScrollHandle::new()));
        let scroll = scrollbar.read(cx).scroll().clone();

        let table = cx.new(|cx| {
            let source = TrackSource::new(columns, DetailTracks(detail.clone()), playback.clone());
            GridState::new(GridDelegate::new(source, width, cx), cx).follow(scroll)
        });

        cx.observe(&detail, |this, _, cx| {
            this.scrollbar
                .read(cx)
                .scroll()
                .set_offset(gpui::Point::default());
            this.rebuild(cx);
            cx.notify();
        })
        .detach();

        cx.observe(&sidebar, |_, _, cx| cx.notify()).detach();

        let current_playback = playback_status(&playback, cx);
        cx.observe(&playback, |this, playback, cx| {
            let current = playback_status(&playback, cx);
            if this.playback_status == current {
                return;
            }
            this.playback_status = current;
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
            playback_status: current_playback,
            sidebar,
            width,
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

    fn resize(&mut self, inset: Pixels, window: &Window, cx: &mut Context<Self>) {
        let sidebar = self.sidebar.read(cx).occupied_width();
        let width = cells::content_width(window, sidebar, reserved(inset));
        if (width - self.width).abs() < px(0.5) {
            return;
        }
        self.width = width;
        self.table.update(cx, |table, cx| {
            table.delegate_mut().set_width(width, cx);
            table.refresh(cx);
        });
    }

    fn viewport(scroll: &ScrollHandle, inset: Pixels, window: &Window) -> Viewport {
        let hero = scroll
            .bounds_for_item(0)
            .map(|bounds| bounds.size.height)
            .unwrap_or_default();
        let visible = scroll.bounds().size.height;

        Viewport {
            top: (scrolled(scroll) - inset - hero - FRAME).max(Pixels::ZERO),
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
        let theme = cx.theme();
        let muted = theme.muted_foreground;
        let header = self.detail.read(cx).header();
        let kind = header.map(|header| header.kind).unwrap_or_default();
        let title = header
            .map(|header| SharedString::from(header.title.clone()))
            .unwrap_or_default();
        let artist = header.and_then(|header| header.artist.clone());
        let artist_refs = header
            .map(|header| header.artist_refs.clone())
            .unwrap_or_default();
        let meta = header
            .map(|header| header.meta.clone())
            .filter(|meta| !meta.is_empty());
        let has_artist = artist.is_some();

        div()
            .flex()
            .flex_none()
            .items_end()
            .gap_5()
            .pb_6()
            .child(
                Artwork::new(header.and_then(|header| header.cover.clone()))
                    .size(theme.metrics.cover),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .min_w_0()
                    .gap_2()
                    .child(
                        div()
                            .text_size(theme.text(Text::Small))
                            .text_color(muted)
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(kind),
                    )
                    .child(
                        div()
                            .text_size(theme.text(Text::Display))
                            .font_weight(FontWeight::BOLD)
                            .truncate()
                            .child(title),
                    )
                    .child(
                        div()
                            .flex()
                            .min_w_0()
                            .text_color(muted)
                            .truncate()
                            .when_some(artist, |this, artist| {
                                this.child(cells::artist_links(
                                    "detail-artist",
                                    artist_refs,
                                    artist,
                                    muted,
                                ))
                            })
                            .when_some(meta, |this, meta| {
                                let meta = match has_artist {
                                    true => format!(" • {meta}"),
                                    false => meta,
                                };
                                this.child(div().flex_none().child(meta))
                            }),
                    ),
            )
            .into_any_element()
    }
}

impl Render for DetailView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.theme();
        let inset = theme.metrics.inset;
        self.resize(inset, window, cx);

        let scroll = self.scrollbar.read(cx).scroll().clone();
        let viewport = Self::viewport(&scroll, inset, window);
        self.table
            .update(cx, |table, _| table.set_viewport(viewport));

        div()
            .size_full()
            .child(
                div()
                    .id("detail-page")
                    .size_full()
                    .overflow_y_scroll()
                    .track_scroll(&scroll)
                    .px(inset)
                    .pt(inset)
                    .pb(inset)
                    .child(self.header(cx))
                    .child(
                        grid(&self.table)
                            .rounded(theme.radius)
                            .border_1()
                            .border_color(theme.border),
                    ),
            )
            .child(self.scrollbar.clone())
    }
}

impl Searchable for DetailView {
    fn search(&mut self, query: &str, cx: &mut Context<Self>) {
        self.table.update(cx, |table, cx| {
            table.delegate_mut().set_filter(query, cx);
            table.refresh(cx);
        });
        cx.notify();
    }

    fn hint() -> &'static str {
        "Filter album tracks"
    }
}
