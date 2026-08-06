use gpui::prelude::*;
use gpui::{
    AnyElement, App, Context, Entity, FontWeight, Pixels, Render, ScrollHandle, SharedString,
    Window, div, px,
};

use spotify::Track;
use state::{Detail, Playback};
use ui::ActiveTheme as _;
use ui::{Card, ColumnSpec, GridDelegate, GridEvent, GridState, Scrollbar, Scroller, Text, grid};

use crate::tracks::{PlaybackStatus, TrackField, TrackSource, Tracks, playback_status};
use crate::{cells, page};
use workspace::{Searchable, Sidebar};

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
        let width = cells::content_width(
            window,
            sidebar.read(cx).occupied_width(),
            page::reserved(inset),
        );

        let scrollbar = cx.new(|_| Scrollbar::new(ScrollHandle::new()));
        let scroll = scrollbar.read(cx).scroll().clone();

        let table = cx.new(|cx| {
            let playlist_scrollbar = cx.new(|_| {
                Scrollbar::new(ScrollHandle::new())
                    .always_visible()
                    .track_inset(px(4.))
            });
            let source = TrackSource::new(
                columns,
                DetailTracks(detail.clone()),
                playback.clone(),
                playlist_scrollbar,
            );
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
            page::play(&this.table, &this.playback, *display, cx);
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

    fn rebuild(&mut self, cx: &mut Context<Self>) {
        self.table.update(cx, |table, cx| {
            table.delegate_mut().clear_selection();
            table.rebuild(cx);
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

        Card::new("detail-hero", title)
            .art(theme.metrics.cover)
            .cover(header.and_then(|header| header.cover.clone()))
            .eyebrow(kind)
            .size(Text::Display)
            .weight(FontWeight::BOLD)
            .bare_meta(
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
            )
            .spacing(theme.metrics.pad)
            .flat()
            .flex_none()
            .items_end()
            .gap_5()
            .px_0()
            .pb_6()
            .into_any_element()
    }
}

impl Render for DetailView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.theme();
        let inset = theme.metrics.inset;
        page::resize(
            &self.table,
            &self.sidebar,
            &mut self.width,
            inset,
            window,
            cx,
        );

        let scroll = self.scrollbar.read(cx).scroll().clone();
        let viewport = page::viewport(&scroll, inset, window);
        self.table
            .update(cx, |table, _| table.set_viewport(viewport));

        Scroller::new("detail-page", &self.scrollbar)
            .px(inset)
            .pt(inset)
            .pb(inset)
            .child(self.header(cx))
            .child(
                grid(&self.table)
                    .rounded(theme.radius)
                    .border_1()
                    .border_color(theme.border),
            )
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
