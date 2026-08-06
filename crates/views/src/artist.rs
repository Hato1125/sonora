use gpui::prelude::*;
use gpui::{
    AnyElement, App, Context, Entity, FontWeight, Pixels, Render, ScrollHandle, SharedString,
    Window, div, px,
};

use spotify::{ReleaseType, Track};
use state::{ArtistDetail, Playback};
use ui::ActiveTheme as _;
use ui::{
    Artwork, Button, ColumnSpec, GridDelegate, GridEvent, GridState, Scrollbar, Text, Viewport,
    grid, scrolled,
};
use workspace::Sidebar;

use crate::cells;
use crate::release_card::ReleaseCard;
use crate::tracks::{PlaybackStatus, TrackField, TrackSource, Tracks, playback_status};

const FRAME: Pixels = px(1.);

fn reserved(inset: Pixels) -> Pixels {
    inset * 2. + px(2.)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ReleaseFilter {
    All,
    Albums,
    Singles,
    Eps,
}

impl ReleaseFilter {
    const ALL: [Self; 4] = [Self::All, Self::Singles, Self::Albums, Self::Eps];

    fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Albums => "Albums",
            Self::Singles => "Singles",
            Self::Eps => "EPs",
        }
    }

    fn matches(self, kind: ReleaseType) -> bool {
        self == Self::All
            || matches!(
                (self, kind),
                (Self::Albums, ReleaseType::Album)
                    | (Self::Singles, ReleaseType::Single)
                    | (Self::Eps, ReleaseType::Ep)
            )
    }
}

struct ArtistTracks(Entity<ArtistDetail>);

impl Tracks for ArtistTracks {
    fn tracks<'a>(&self, cx: &'a App) -> &'a [Track] {
        self.0.read(cx).tracks()
    }

    fn is_loading(&self, cx: &App) -> bool {
        self.0.read(cx).is_loading()
    }
}

pub(crate) struct ArtistView {
    detail: Entity<ArtistDetail>,
    playback: Entity<Playback>,
    playback_status: PlaybackStatus,
    release_filter: ReleaseFilter,
    sidebar: Entity<Sidebar>,
    width: Pixels,
    scrollbar: Entity<Scrollbar>,
    table: Entity<GridState<TrackSource>>,
}

impl ArtistView {
    pub(crate) fn new(
        detail: Entity<ArtistDetail>,
        playback: Entity<Playback>,
        sidebar: Entity<Sidebar>,
        columns: &'static [ColumnSpec<TrackField>],
        cx: &mut Context<Self>,
    ) -> Self {
        let width = px(200.);
        let scrollbar = cx.new(|_| Scrollbar::new(ScrollHandle::new()));
        let scroll = scrollbar.read(cx).scroll().clone();
        let table = cx.new(|cx| {
            let source = TrackSource::new(columns, ArtistTracks(detail.clone()), playback.clone());
            GridState::new(GridDelegate::new(source, width, cx), cx).follow(scroll)
        });

        cx.observe(&detail, |this, _, cx| {
            this.release_filter = ReleaseFilter::All;
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
            release_filter: ReleaseFilter::All,
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
        let artist = self.detail.read(cx).artist();
        let title = artist
            .map(|artist| SharedString::from(artist.name.clone()))
            .unwrap_or_default();

        div()
            .flex()
            .flex_none()
            .items_end()
            .gap_5()
            .pb_6()
            .child(
                Artwork::new(artist.and_then(|artist| artist.cover_large.clone()))
                    .size(theme.metrics.cover)
                    .circle(),
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
                            .text_color(theme.muted_foreground)
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("ARTIST"),
                    )
                    .child(
                        div()
                            .text_size(theme.text(Text::Display))
                            .font_weight(FontWeight::BOLD)
                            .truncate()
                            .child(title),
                    ),
            )
            .into_any_element()
    }

    fn releases(&self, cx: &Context<Self>) -> Option<AnyElement> {
        let theme = *cx.theme();
        let albums = self.detail.read(cx).albums();
        if albums.is_empty() {
            return None;
        }

        Some(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .pt_6()
                .child(
                    div()
                        .text_size(theme.text(Text::Title))
                        .font_weight(FontWeight::BOLD)
                        .child("Releases"),
                )
                .child(
                    div()
                        .flex()
                        .gap_1()
                        .children(ReleaseFilter::ALL.map(|filter| {
                            Button::new(SharedString::from(format!(
                                "release-filter-{}",
                                filter.label().to_lowercase()
                            )))
                            .label(filter.label())
                            .small()
                            .outline()
                            .selected(self.release_filter == filter)
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    this.release_filter = filter;
                                    cx.notify();
                                },
                            ))
                        })),
                )
                .child(
                    div().flex().flex_wrap().gap_8().children(
                        albums
                            .iter()
                            .filter(|album| self.release_filter.matches(album.release_type))
                            .cloned()
                            .enumerate()
                            .map(|(index, album)| ReleaseCard::new(index, album)),
                    ),
                )
                .into_any_element(),
        )
    }

    fn failure(&self, cx: &Context<Self>) -> Option<AnyElement> {
        let error = self.detail.read(cx).error()?.to_owned();
        Some(
            div()
                .pb_4()
                .text_color(cx.theme().danger)
                .child(error)
                .into_any_element(),
        )
    }
}

impl Render for ArtistView {
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
                    .id("artist-page")
                    .size_full()
                    .overflow_y_scroll()
                    .track_scroll(&scroll)
                    .px(inset)
                    .pt(inset)
                    .pb(inset)
                    .child(
                        div()
                            .child(self.header(cx))
                            .children(self.failure(cx))
                            .child(
                                div()
                                    .pb_3()
                                    .text_size(theme.text(Text::Title))
                                    .font_weight(FontWeight::BOLD)
                                    .child("Popular"),
                            ),
                    )
                    .child(
                        grid(&self.table)
                            .rounded(theme.radius)
                            .border_1()
                            .border_color(theme.border),
                    )
                    .children(self.releases(cx)),
            )
            .child(self.scrollbar.clone())
    }
}
