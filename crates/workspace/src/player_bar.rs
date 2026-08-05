use router::{Destination, Link};
use std::time::Duration;
use ui::ActiveTheme as _;

use gpui::prelude::*;
use gpui::{Context, Entity, MouseMoveEvent, MouseUpEvent, Render, SharedString, svg};
use gpui::{Window, div, px};
use state::{Playback, PlaybackState, Queue};

use ui::{Artwork, Button, Scrubber, ScrubberState, clock};

const SEEK_MAX: f32 = 560.;
const VOLUME_WIDTH: f32 = 110.;
const CLOCK_CHARS: f32 = 3.4;
const VOLUME_BREAKPOINT: f32 = 720.;
const CLOCK_BREAKPOINT: f32 = 560.;
const TRACK_BREAKPOINT: f32 = 460.;
const STEP: f32 = 0.004;

pub struct PlayerBar {
    playback: Entity<Playback>,
    queue: Entity<Queue>,
    seek: ScrubberState,
    volume: ScrubberState,
    pending: Option<f32>,
    over_seek: Option<f32>,
    over_volume: Option<f32>,
}

impl PlayerBar {
    pub fn new(playback: Entity<Playback>, queue: Entity<Queue>, cx: &mut Context<Self>) -> Self {
        cx.observe(&playback, |_, _, cx| cx.notify()).detach();
        cx.observe(&queue, |_, _, cx| cx.notify()).detach();

        Self {
            playback,
            queue,
            seek: ScrubberState::new("seek"),
            volume: ScrubberState::new("volume"),
            pending: None,
            over_seek: None,
            over_volume: None,
        }
    }

    fn commit_seek(&mut self, cx: &mut Context<Self>) {
        let Some(fraction) = self.pending.take() else {
            return;
        };
        self.playback
            .update(cx, |playback, cx| playback.seek_fraction(fraction, cx));
    }

    fn hover(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        let seek = self.seek.hovered(event.position);
        let volume = self.volume.hovered(event.position);

        if moved(self.over_seek, seek) || moved(self.over_volume, volume) {
            self.over_seek = seek;
            self.over_volume = volume;
            cx.notify();
        }
    }

    fn transport(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(self.previous(cx))
            .child(self.toggle(cx))
            .child(self.next(cx))
    }

    fn previous(&self, cx: &mut Context<Self>) -> Button {
        let enabled = self.queue.read(cx).has_previous();

        Button::new("previous")
            .ghost()
            .small()
            .icon("icons/skip-back.svg")
            .disabled(!enabled)
            .on_click(cx.listener(|this, _, _, cx| {
                this.playback
                    .update(cx, |playback, cx| playback.previous(cx));
            }))
    }

    fn next(&self, cx: &mut Context<Self>) -> Button {
        let enabled = self.queue.read(cx).has_next();

        Button::new("next")
            .ghost()
            .small()
            .icon("icons/skip-forward.svg")
            .disabled(!enabled)
            .on_click(cx.listener(|this, _, _, cx| {
                this.playback.update(cx, |playback, cx| playback.next(cx));
            }))
    }

    fn toggle(&self, cx: &mut Context<Self>) -> Button {
        let state = self.playback.read(cx).state();
        let playing = matches!(state, PlaybackState::Playing);
        let idle = matches!(state, PlaybackState::Idle | PlaybackState::Failed(_));

        let (id, icon) = if playing {
            ("pause", "icons/pause.svg")
        } else {
            ("play", "icons/play.svg")
        };

        Button::new(id)
            .ghost()
            .small()
            .icon(icon)
            .disabled(idle)
            .on_click(cx.listener(|this, _, _, cx| {
                this.playback
                    .update(cx, |playback, cx| playback.toggle_play(cx));
            }))
    }

    fn now_playing(&self, room: bool, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let muted = theme.muted_foreground;
        let artwork = ui::snapped(theme.metrics.row, window);
        let artists = theme.text(ui::Text::Small);
        let track = self.playback.read(cx).track().cloned();
        let cover = track.as_ref().and_then(|track| track.cover.clone());

        div()
            .flex()
            .items_center()
            .gap_3()
            .flex_1()
            .min_w_0()
            .child(Artwork::new(cover).size(artwork))
            .when(room, |this| {
                this.child(
                    div()
                        .flex()
                        .flex_col()
                        .justify_center()
                        .flex_1()
                        .min_w_0()
                        .child(match &track {
                            Some(track) => div()
                                .id("now-playing-album")
                                .when_some(track.album_id.clone(), |this, album| {
                                    this.hover(|style| style.underline())
                                        .link(Destination::Album(album.into()))
                                })
                                .child(SharedString::from(track.name.clone()))
                                .w_full()
                                .truncate(),
                            None => div()
                                .id("now-playing-album")
                                .child("Nothing playing")
                                .w_full()
                                .text_color(muted)
                                .truncate(),
                        })
                        .when_some(track, |this, track| {
                            this.child(
                                div()
                                    .child(SharedString::from(track.artists))
                                    .w_full()
                                    .text_color(muted)
                                    .text_size(artists)
                                    .truncate(),
                            )
                        }),
                )
            })
    }
}

fn moved(before: Option<f32>, after: Option<f32>) -> bool {
    match (before, after) {
        (Some(before), Some(after)) => (before - after).abs() > STEP,
        (before, after) => before.is_some() != after.is_some(),
    }
}

fn volume_icon(level: f32) -> &'static str {
    if level <= 0.001 {
        "icons/volume-x.svg"
    } else if level < 0.5 {
        "icons/volume-1.svg"
    } else {
        "icons/volume-2.svg"
    }
}

fn percent(fraction: f32) -> SharedString {
    SharedString::from(format!("{}%", (fraction * 100.).round()))
}

impl Render for PlayerBar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.theme();
        let muted = theme.muted_foreground;
        let empty = muted.opacity(0.3);
        let height = ui::snapped(theme.metrics.player_bar, window);
        let clock_text = theme.text(ui::Text::Tiny);
        let clock_width = clock_text * CLOCK_CHARS;

        let viewport = window.viewport_size().width;
        let show_volume = viewport >= px(VOLUME_BREAKPOINT);
        let show_clocks = viewport >= px(CLOCK_BREAKPOINT);
        let show_track = viewport >= px(TRACK_BREAKPOINT);

        let playback = self.playback.read(cx);
        let seekable = playback.track().is_some();
        let progress = self.pending.unwrap_or_else(|| playback.progress());
        let elapsed = playback.position();
        let total = playback
            .track()
            .map(|track| track.duration)
            .unwrap_or(Duration::ZERO);
        let level = playback.volume();

        let seek_bubble = self
            .over_seek
            .or(self.pending)
            .map(|at| (at, clock(total.mul_f32(at))));
        let volume_bubble = self.over_volume.map(|at| (at, percent(at)));

        let clock_label = |value: Duration, align_end: bool| {
            div()
                .child(clock(value))
                .w(clock_width)
                .flex_none()
                .text_size(clock_text)
                .text_color(muted)
                .when_else(align_end, |this| this.text_right(), |this| this.text_left())
        };

        div()
            .flex()
            .items_center()
            .gap_4()
            .w_full()
            .h(height)
            .flex_none()
            .px_5()
            .bg(theme.secondary)
            .border_t_1()
            .border_color(theme.border)
            .on_mouse_move(cx.listener(Self::hover))
            .child(self.now_playing(show_track, window, cx))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_1()
                    .flex_1()
                    .min_w_0()
                    .max_w(px(SEEK_MAX))
                    .child(self.transport(cx))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .w_full()
                            .when(show_clocks, |this| this.child(clock_label(elapsed, true)))
                            .child(
                                div().flex_1().min_w_0().child(
                                    Scrubber::new(&self.seek, progress)
                                        .colors(theme.progress_bar, empty, theme.foreground)
                                        .enabled(seekable)
                                        .when_some(seek_bubble, |this, (at, text)| {
                                            this.bubble(at, text)
                                        })
                                        .on_move(cx.listener(|this, fraction: &f32, _, cx| {
                                            this.pending = Some(*fraction);
                                            cx.notify();
                                        }))
                                        .on_release(cx.listener(
                                            |this, _: &MouseUpEvent, _, cx| this.commit_seek(cx),
                                        )),
                                ),
                            )
                            .when(show_clocks, |this| this.child(clock_label(total, false))),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap_2()
                    .flex_1()
                    .min_w_0()
                    .child(
                        svg()
                            .path(volume_icon(level))
                            .size_4()
                            .flex_none()
                            .text_color(muted),
                    )
                    .when(show_volume, |this| {
                        this.child(
                            div().w(px(VOLUME_WIDTH)).flex_none().child(
                                Scrubber::new(&self.volume, level)
                                    .colors(theme.progress_bar, empty, theme.foreground)
                                    .when_some(volume_bubble, |this, (at, text)| {
                                        this.bubble(at, text)
                                    })
                                    .on_move(cx.listener(|this, fraction: &f32, _, cx| {
                                        let level = *fraction;
                                        this.playback.update(cx, |playback, cx| {
                                            playback.set_volume(level, cx)
                                        });
                                    })),
                            ),
                        )
                    }),
            )
    }
}
