use gpui::prelude::*;
use gpui::{Context, Entity, MouseUpEvent, Render};
use gpui::{SharedString, Window, div, px};
use gpui_component::ActiveTheme as _;
use gpui_component::Icon;
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::label::Label;
use gpui_component::{Disableable as _, Sizable as _};
use state::{Playback, PlaybackState};

use ui::{Artwork, Scrubber, ScrubberState};

const HEIGHT: f32 = 68.;
const TRACK_WIDTH: f32 = 420.;
const VOLUME_WIDTH: f32 = 120.;

pub struct PlayerBar {
    playback: Entity<Playback>,
    seek: ScrubberState,
    volume: ScrubberState,
    pending: Option<f32>,
}

impl PlayerBar {
    pub fn new(playback: Entity<Playback>, cx: &mut Context<Self>) -> Self {
        cx.observe(&playback, |_, _, cx| cx.notify()).detach();

        Self {
            playback,
            seek: ScrubberState::new("seek"),
            volume: ScrubberState::new("volume"),
            pending: None,
        }
    }

    fn commit_seek(&mut self, cx: &mut Context<Self>) {
        let Some(fraction) = self.pending.take() else {
            return;
        };
        self.playback
            .update(cx, |playback, cx| playback.seek_fraction(fraction, cx));
    }

    fn transport(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(Self::skip("previous", "icons/skip-back.svg"))
            .child(self.toggle(cx))
            .child(Self::skip("next", "icons/skip-forward.svg"))
    }

    fn skip(id: &'static str, icon: &'static str) -> Button {
        Button::new(id)
            .ghost()
            .small()
            .icon(Icon::default().path(icon))
            .disabled(true)
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
            .icon(Icon::default().path(icon))
            .disabled(idle)
            .on_click(cx.listener(|this, _, _, cx| {
                this.playback
                    .update(cx, |playback, cx| playback.toggle_play(cx));
            }))
    }

    fn now_playing(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        let track = self.playback.read(cx).track().cloned();
        let cover = track.as_ref().and_then(|track| track.cover.clone());

        let width = window.viewport_size().width / 3. - px(100.);

        div()
            .flex()
            .gap_4()
            .child(Artwork::new(cover).size(px(48.)).rounded(px(6.)))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .justify_center()
                    .w(width)
                    .flex_none()
                    .min_w_0()
                    .child(match &track {
                        Some(track) => Label::new(SharedString::from(track.name.clone()))
                            .w(width)
                            .truncate(),
                        None => Label::new("Nothing playing").text_color(muted),
                    })
                    .when_some(track, |this, track| {
                        this.child(
                            Label::new(SharedString::from(track.artists))
                                .w(width)
                                .text_color(muted)
                                .text_size(px(11.))
                                .truncate(),
                        )
                    }),
            )
    }
}

impl Render for PlayerBar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let playback = self.playback.read(cx);
        let seekable = playback.track().is_some();
        let progress = self.pending.unwrap_or_else(|| playback.progress());
        let level = playback.volume();

        div()
            .flex()
            .items_center()
            .gap_4()
            .w_full()
            .h(px(HEIGHT))
            .flex_none()
            .px_5()
            .bg(theme.secondary)
            .border_t_1()
            .border_color(theme.border)
            .child(self.now_playing(window, cx))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .flex_col()
                    .items_center()
                    .gap_1()
                    .child(self.transport(cx))
                    .child(
                        div().w_full().max_w(px(TRACK_WIDTH)).child(
                            Scrubber::new(&self.seek, progress)
                                .colors(theme.progress_bar, theme.muted, theme.foreground)
                                .enabled(seekable)
                                .on_move(cx.listener(|this, fraction: &f32, _, cx| {
                                    this.pending = Some(*fraction);
                                    cx.notify();
                                }))
                                .on_release(cx.listener(|this, _: &MouseUpEvent, _, cx| {
                                    this.commit_seek(cx)
                                })),
                        ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_end()
                    .w(px(240.))
                    .flex_none()
                    .child(
                        div().w(px(VOLUME_WIDTH)).child(
                            Scrubber::new(&self.volume, level)
                                .colors(theme.progress_bar, theme.muted, theme.foreground)
                                .on_move(cx.listener(|this, fraction: &f32, _, cx| {
                                    let level = *fraction;
                                    this.playback
                                        .update(cx, |playback, cx| playback.set_volume(level, cx));
                                })),
                        ),
                    ),
            )
    }
}
