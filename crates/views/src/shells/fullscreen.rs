use std::time::Duration;

use gpui::prelude::*;
use gpui::{
    AnyView, App, Context, Entity, FocusHandle, MouseUpEvent, Pixels, Render, SharedString,
};
use gpui::{Window, div, px};
use i18n::t;
use input::ToggleFullscreen;
use state::{Cover, Playback, Queue, SideTab, Sonora};
use ui::{
    ActiveTheme as _, Artwork, Button, Motion, Motioned as _, Room, Scrubber, ScrubberState, Text,
    clock, snapped,
};

use crate::chrome::{Aside, TitleBarOptions};
use crate::shared::transport::{like, transport};
use crate::shells::Shell;

const COVER_TALL: f32 = 0.46;
const COVER_WIDE: f32 = 0.34;
const COVER_MIN: f32 = 160.;
const COVER_MAX: f32 = 520.;
const PANEL: f32 = 460.;
const PILL_GAP: f32 = 3.;
const SEEK_MAX: f32 = 720.;
const CLOCK_SHORT: f32 = 3.4;
const CLOCK_LONG: f32 = 5.4;

pub struct FullscreenView {
    playback: Entity<Playback>,
    queue: Entity<Queue>,
    cover: Entity<Cover>,
    aside: Entity<Aside>,
    panel: Option<SideTab>,
    seek: ScrubberState,
    pending: Option<f32>,
    large: Option<SharedString>,
    revision: usize,
    focus: FocusHandle,
}

impl FullscreenView {
    pub fn new(playback: Entity<Playback>, queue: Entity<Queue>, cx: &mut Context<Self>) -> Self {
        cx.observe(&playback, |_, _, cx| cx.notify()).detach();
        let cover = Sonora::global(cx).cover.clone();
        cx.observe(&cover, |_, _, cx| cx.notify()).detach();
        let aside = cx.new(|cx| Aside::new(queue.clone(), playback.clone(), SideTab::Lyrics, cx));
        aside.update(cx, |aside, _| aside.strip());
        cx.observe(&aside, |_, _, cx| cx.notify()).detach();

        Self {
            playback,
            queue,
            cover,
            aside,
            panel: None,
            seek: ScrubberState::new("fullscreen-seek"),
            pending: None,
            large: None,
            revision: 0,
            focus: cx.focus_handle(),
        }
    }

    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        window.focus(&self.focus, cx);
    }

    fn show(&mut self, panel: Option<SideTab>, cx: &mut Context<Self>) {
        self.panel = panel;
        if let Some(tab) = panel {
            self.aside.update(cx, |aside, cx| aside.show(tab, cx));
        }
        cx.notify();
    }

    fn commit_seek(&mut self, cx: &mut Context<Self>) {
        let Some(fraction) = self.pending.take() else {
            return;
        };
        self.playback
            .update(cx, |playback, cx| playback.seek_fraction(fraction, cx));
    }

    fn artwork(&mut self, side: Pixels, cx: &mut Context<Self>) -> impl IntoElement {
        let radius = cx.theme().radius * 2.;
        let track = self.playback.read(cx).track().cloned();
        let small = track.as_ref().and_then(|track| track.cover.clone());
        let large = self
            .cover
            .read(cx)
            .large()
            .filter(|url| Some(*url) != small.as_deref())
            .map(SharedString::from);

        if self.large != large {
            self.large = large.clone();
            self.revision += 1;
        }
        let revision = self.revision;

        div()
            .relative()
            .size(side)
            .flex_none()
            .child(Artwork::new(small).size(side).corner_radius(radius))
            .when_some(large, |this, url| {
                this.child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .child(Artwork::new(Some(url)).size(side).corner_radius(radius))
                        .motion(("cover-large", revision), Motion::Slow, |art, t| {
                            art.opacity(t)
                        }),
                )
            })
    }

    fn meta(&self, width: Pixels, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.theme();
        let track = self.playback.read(cx).track().cloned();
        let title = match &track {
            Some(track) => SharedString::from(track.name.clone()),
            None => t!("player-nothing-playing"),
        };
        let artists = track
            .as_ref()
            .map(|track| SharedString::from(track.artists.clone()));

        div()
            .flex()
            .flex_col()
            .items_center()
            .gap_1()
            .max_w(width)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .min_w_0()
                    .child(div().w(theme.metrics.control_small).flex_none())
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_size(theme.text(Text::Title))
                            .child(title),
                    )
                    .child(like(track.clone(), cx)),
            )
            .when_some(artists, |this, artists| {
                this.child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_size(theme.text(Text::Body))
                        .text_color(theme.muted_foreground)
                        .child(artists),
                )
            })
    }

    fn seek(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.theme();
        let muted = theme.muted_foreground;
        let empty = muted.opacity(0.3);
        let text = theme.text(Text::Tiny);
        let playback = self.playback.read(cx);
        let seekable = playback.track().is_some();
        let progress = self.pending.unwrap_or_else(|| playback.progress());
        let elapsed = playback.position();
        let total = playback
            .track()
            .map(|track| track.duration)
            .unwrap_or(Duration::ZERO);
        let width = text
            * match total.as_secs() >= 3600 {
                true => CLOCK_LONG,
                false => CLOCK_SHORT,
            };

        let label = move |value: Duration, align_end: bool| {
            div()
                .child(clock(value))
                .w(width)
                .flex_none()
                .whitespace_nowrap()
                .text_size(text)
                .text_color(muted)
                .when_else(align_end, |this| this.text_right(), |this| this.text_left())
        };

        div()
            .flex()
            .items_center()
            .gap_2()
            .w_full()
            .child(label(elapsed, true))
            .child(
                div().flex_1().min_w_0().child(
                    Scrubber::new(&self.seek, progress)
                        .colors(theme.progress_bar, empty, theme.foreground)
                        .enabled(seekable)
                        .on_move(cx.listener(|this, fraction: &f32, _, cx| {
                            this.pending = Some(*fraction);
                            cx.notify();
                        }))
                        .on_release(
                            cx.listener(|this, _: &MouseUpEvent, _, cx| this.commit_seek(cx)),
                        ),
                ),
            )
            .child(label(total, false))
    }

    fn controls(&self, width: Pixels, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .items_center()
            .gap_3()
            .w_full()
            .max_w(width)
            .flex_none()
            .child(self.seek(cx))
            .child(transport(&self.playback, &self.queue, true, cx))
    }

    fn pill(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.theme();
        let gap = px(PILL_GAP);

        let tab = move |id: &'static str, icon: &'static str, hint: &'static str, panel| {
            let showing = self.panel == panel;

            Button::new(id)
                .ghost()
                .small()
                .icon(icon)
                .tooltip_above(hint)
                .selected(showing)
                .rounded(theme.radius)
                .tint(match showing {
                    true => theme.foreground,
                    false => theme.muted_foreground,
                })
                .on_click(cx.listener(move |this, _, _, cx| this.show(panel, cx)))
        };

        div()
            .absolute()
            .bottom_3()
            .w_full()
            .flex()
            .justify_center()
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(gap)
                    .p(gap)
                    .rounded(theme.radius + gap)
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.popover)
                    .block_mouse_except_scroll()
                    .child(tab(
                        "fullscreen-artwork",
                        "icons/disc-3.svg",
                        "fullscreen-artwork",
                        None,
                    ))
                    .child(tab(
                        "fullscreen-lyrics",
                        "icons/mic-vocal.svg",
                        "lyrics-title",
                        Some(SideTab::Lyrics),
                    ))
                    .child(tab(
                        "fullscreen-queue",
                        "icons/list-music.svg",
                        "queue-title",
                        Some(SideTab::Queue),
                    )),
            )
    }

    fn leave(&self) -> Button {
        Button::new("leave-fullscreen")
            .ghost()
            .small()
            .icon("icons/chevron-down.svg")
            .tooltip_above("player-fullscreen-leave")
            .on_click(|_, window, cx| window.dispatch_action(Box::new(ToggleFullscreen), cx))
    }
}

impl Shell for FullscreenView {
    fn title_bar(&self, _content: Option<AnyView>, _cx: &App) -> TitleBarOptions {
        TitleBarOptions {
            navigation: false,
            sidebar_open: false,
            sidebar_right: None,
            offset: Pixels::ZERO,
            border: false,
            content: None,
        }
    }
}

impl Render for FullscreenView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.theme();
        let viewport = window.viewport_size();
        let room = Room::of(viewport.width);
        let split = room.fits(Room::Wide) && self.panel.is_some();
        let side = snapped(
            (viewport.height * COVER_TALL)
                .min(viewport.width * COVER_WIDE)
                .clamp(px(COVER_MIN), px(COVER_MAX)),
            window,
        );
        let staged = self.panel.is_none() || split;

        div()
            .id("fullscreen")
            .track_focus(&self.focus)
            .relative()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .w_full()
            .gap_6()
            .px_8()
            .pb_6()
            .bg(theme.background)
            .child(
                div()
                    .relative()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .items_center()
                    .justify_center()
                    .gap_8()
                    .when(staged, |this| {
                        this.child(
                            div()
                                .flex()
                                .flex_col()
                                .items_center()
                                .justify_center()
                                .gap_5()
                                .min_w_0()
                                .when(split, |this| this.flex_1().h_full())
                                .child(self.artwork(side, cx))
                                .child(self.meta(side, cx))
                                .when(split, |this| this.child(self.controls(side, cx))),
                        )
                    })
                    .when(self.panel.is_some(), |this| {
                        this.child(
                            div()
                                .relative()
                                .flex()
                                .flex_col()
                                .flex_1()
                                .min_w_0()
                                .min_h_0()
                                .h_full()
                                .when(split, |this| this.max_w(px(PANEL)))
                                .child(self.aside.clone())
                                .child(self.pill(cx)),
                        )
                    })
                    .when(self.panel.is_none(), |this| this.child(self.pill(cx))),
            )
            .when(!split, |this| this.child(self.controls(px(SEEK_MAX), cx)))
            .child(div().absolute().top_0().right_0().p_2().child(self.leave()))
    }
}
