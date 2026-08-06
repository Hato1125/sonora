use std::rc::Rc;

use gpui::prelude::*;
use gpui::{App, ClickEvent, Entity, FontWeight, Pixels, SharedString, Window, div};
use spotify::Track;
use state::Playback;
use ui::{ActiveTheme as _, Artwork, Button, ExplicitBadge, Skeleton, Text, snapped};

use crate::cells;

const ROWS_PER_COLUMN: usize = 6;
const MAX_COLUMNS: usize = 3;
const MIN_COLUMN_WIDTH: Pixels = gpui::px(280.);

type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

pub(crate) fn column_count(width: Pixels) -> usize {
    ((width / MIN_COLUMN_WIDTH).floor().max(1.) as usize).min(MAX_COLUMNS)
}

pub(crate) fn page_count(track_count: usize, width: Pixels) -> usize {
    track_count
        .div_ceil(column_count(width) * ROWS_PER_COLUMN)
        .max(1)
}

#[derive(IntoElement)]
pub(crate) struct QuickPicks {
    tracks: Rc<Vec<Track>>,
    playback: Entity<Playback>,
    active: Option<String>,
    width: Pixels,
    page: usize,
    on_previous: Option<ClickHandler>,
    on_next: Option<ClickHandler>,
}

impl QuickPicks {
    pub(crate) fn new(
        tracks: Rc<Vec<Track>>,
        playback: Entity<Playback>,
        active: Option<String>,
        width: Pixels,
        page: usize,
    ) -> Self {
        Self {
            tracks,
            playback,
            active,
            width,
            page,
            on_previous: None,
            on_next: None,
        }
    }

    pub(crate) fn on_previous(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_previous = Some(Rc::new(handler));
        self
    }

    pub(crate) fn on_next(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_next = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for QuickPicks {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = *cx.theme();
        let columns = column_count(self.width);
        let page_size = columns * ROWS_PER_COLUMN;
        let pages = page_count(self.tracks.len(), self.width);
        let page = self.page.min(pages.saturating_sub(1));
        let start = page * page_size;
        let end = (start + page_size).min(self.tracks.len());
        let tracks = self.tracks;
        let empty = tracks.is_empty();
        let on_previous = self.on_previous;
        let on_next = self.on_next;

        div()
            .w_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .rounded(theme.radius)
            .border_1()
            .border_color(theme.border)
            .child(
                div()
                    .flex()
                    .items_end()
                    .justify_between()
                    .gap_4()
                    .px_3()
                    .py_2()
                    .border_b_1()
                    .border_color(theme.border)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_0p5()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(theme.muted_foreground)
                                    .child("START FROM A SONG"),
                            )
                            .child(
                                div()
                                    .text_size(theme.text(Text::Title))
                                    .font_weight(FontWeight::BOLD)
                                    .child("Quick picks"),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                Button::new("quick-picks-previous")
                                    .small()
                                    .outline()
                                    .icon("icons/chevron-left.svg")
                                    .disabled(empty || page == 0)
                                    .when_some(on_previous, |button, handler| {
                                        button.on_click(move |event, window, cx| {
                                            handler(event, window, cx)
                                        })
                                    }),
                            )
                            .child(
                                Button::new("quick-picks-next")
                                    .small()
                                    .outline()
                                    .icon("icons/chevron-right.svg")
                                    .disabled(empty || page + 1 >= pages)
                                    .when_some(on_next, |button, handler| {
                                        button.on_click(move |event, window, cx| {
                                            handler(event, window, cx)
                                        })
                                    }),
                            ),
                    ),
            )
            .child(div().flex().gap_2().p_2().when_else(
                empty,
                |this| {
                    this.children((0..columns).map(|column| {
                        column_shell(column, theme.border)
                            .children((0..ROWS_PER_COLUMN).map(|_| skeleton(window, cx)))
                    }))
                },
                |this| {
                    this.children(tracks[start..end].chunks(ROWS_PER_COLUMN).enumerate().map(
                        |(column, column_tracks)| {
                            column_shell(column, theme.border).children(
                                column_tracks.iter().enumerate().map(|(row, track)| {
                                    let place = start + column * ROWS_PER_COLUMN + row;
                                    pick(
                                        track,
                                        place,
                                        tracks.clone(),
                                        self.playback.clone(),
                                        self.active.as_deref(),
                                        window,
                                        cx,
                                    )
                                }),
                            )
                        },
                    ))
                },
            ))
    }
}

fn column_shell(column: usize, border: gpui::Hsla) -> gpui::Div {
    div()
        .flex()
        .flex_1()
        .min_w_0()
        .flex_col()
        .gap_1()
        .when(column > 0, |this| {
            this.border_l_1().border_color(border).pl_2()
        })
}

fn skeleton(window: &Window, cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    let height = snapped(theme.metrics.list_row, window);
    let artwork = height - theme.metrics.pad * 2.;

    div()
        .flex()
        .items_center()
        .h(height)
        .gap_3()
        .px_2()
        .child(Skeleton::new().size(artwork))
        .child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(Skeleton::new().w(gpui::px(140.)).h(gpui::px(11.)))
                .child(Skeleton::new().w(gpui::px(90.)).h(gpui::px(9.))),
        )
}

fn pick(
    track: &Track,
    place: usize,
    tracks: Rc<Vec<Track>>,
    playback: Entity<Playback>,
    active: Option<&str>,
    window: &Window,
    cx: &App,
) -> impl IntoElement {
    let theme = *cx.theme();
    let height = snapped(theme.metrics.list_row, window);
    let artwork = height - theme.metrics.pad * 2.;
    let tint = match track.id.as_deref() == active {
        true => theme.primary,
        false => theme.foreground,
    };

    div()
        .id(("quick-pick", place))
        .flex()
        .items_center()
        .min_w_0()
        .h(height)
        .gap_3()
        .px_2()
        .rounded(theme.radius)
        .cursor_pointer()
        .hover(move |style| style.bg(theme.table_hover))
        .on_click(move |_, _, cx| {
            playback.update(cx, |playback, cx| playback.play_radio(&tracks[place], cx));
        })
        .child(Artwork::new(track.cover.clone()).size(artwork))
        .child(
            div()
                .flex()
                .flex_1()
                .min_w_0()
                .flex_col()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1p5()
                        .min_w_0()
                        .text_color(tint)
                        .child(
                            div()
                                .min_w_0()
                                .truncate()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(SharedString::from(track.name.clone())),
                        )
                        .when(track.explicit, |this| {
                            this.child(div().flex_none().child(ExplicitBadge::new()))
                        }),
                )
                .child(
                    cells::artist_links(
                        SharedString::from(format!("quick-pick-artist-{place}")),
                        track.artist_refs.clone(),
                        track.artists.clone(),
                        theme.muted_foreground,
                    )
                    .text_size(theme.text(Text::Small)),
                ),
        )
}

#[cfg(test)]
mod tests {
    use gpui::px;

    use super::{column_count, page_count};

    #[test]
    fn columns_follow_width_and_stop_at_three() {
        assert_eq!(column_count(px(279.)), 1);
        assert_eq!(column_count(px(560.)), 2);
        assert_eq!(column_count(px(840.)), 3);
        assert_eq!(column_count(px(2_000.)), 3);
    }

    #[test]
    fn pages_include_every_track() {
        assert_eq!(page_count(36, px(279.)), 6);
        assert_eq!(page_count(36, px(560.)), 3);
        assert_eq!(page_count(36, px(840.)), 2);
        assert_eq!(page_count(0, px(840.)), 1);
    }
}
