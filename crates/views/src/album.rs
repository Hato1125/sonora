use gpui::prelude::*;
use gpui::{
    AnyElement, App, Context, Entity, FontWeight, Pixels, Render, SharedString, Window, div, px,
};
use gpui_component::ActiveTheme as _;
use gpui_component::table::{TableEvent, TableState};
use spotify::{Album, Track};
use state::{AlbumDetail, Playback};
use ui::{Artwork, GridDelegate, GridState, grid};

use crate::cells;
use crate::tracks::{ALBUM_COLUMNS, TrackSource, Tracks};

const INSET: Pixels = px(48.);
const COVER: Pixels = px(140.);

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
    width: Pixels,
    table: Entity<GridState<TrackSource>>,
}

impl AlbumView {
    pub fn new(
        detail: Entity<AlbumDetail>,
        playback: Entity<Playback>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let width = cells::table_width(window, INSET);

        let table = cx.new(|cx| {
            let source = TrackSource::new(ALBUM_COLUMNS, DetailTracks(detail.clone()));
            TableState::new(GridDelegate::new(source, width, cx), window, cx).col_selectable(false)
        });

        cx.observe(&detail, |this, _, cx| {
            this.rebuild(cx);
            cx.notify();
        })
        .detach();

        cx.subscribe(&table, |this, _, event, cx| {
            if let TableEvent::DoubleClickedRow(display) = event {
                this.play(*display, cx);
            }
        })
        .detach();

        Self {
            detail,
            playback,
            width,
            table,
        }
    }

    fn play(&mut self, display: usize, cx: &mut Context<Self>) {
        let track = {
            let state = self.table.read(cx);
            let row = state.delegate().row(display);
            state.delegate().source().at(row, cx)
        };
        let Some(track) = track else {
            return;
        };
        self.playback
            .update(cx, |playback, cx| playback.play(&track, cx));
    }

    fn rebuild(&mut self, cx: &mut Context<Self>) {
        let width = self.width;
        self.table.update(cx, |table, cx| {
            table.delegate_mut().set_width(width, cx);
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
                    .child(
                        div()
                            .text_color(muted)
                            .truncate()
                            .children(album.map(meta)),
                    ),
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
        let width = cells::table_width(window, INSET);
        if (width - self.width).abs() > px(1.) {
            self.width = width;
            self.rebuild(cx);
        }

        div()
            .flex()
            .flex_col()
            .size_full()
            .px_6()
            .pt_6()
            .child(self.header(cx))
            .child(div().flex_1().min_h_0().child(grid(&self.table)))
    }
}
