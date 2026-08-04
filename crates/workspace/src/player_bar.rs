use gpui::prelude::*;
use gpui::{Context, Entity, Render};
use gpui::{SharedString, Window, div, px, relative};
use gpui_component::ActiveTheme as _;
use gpui_component::Icon;
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::label::Label;
use gpui_component::{Disableable as _, Sizable as _};
use state::{Playback, PlaybackState, Spotty};

const HEIGHT: f32 = 68.;

pub struct PlayerBar {
    playback: Entity<Playback>,
    now_playing: Option<NowPlaying>,
}

pub struct NowPlaying {
    pub title: SharedString,
    pub artists: SharedString,
    pub position: f32,
}

impl PlayerBar {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let playback = Spotty::global(cx).playback.clone();
        cx.observe(&playback, |_, _, cx| cx.notify()).detach();

        Self {
            playback,
            now_playing: None,
        }
    }

    pub fn set_now_playing(&mut self, now_playing: Option<NowPlaying>, cx: &mut Context<Self>) {
        self.now_playing = now_playing;
        cx.notify();
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
}

impl Render for PlayerBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let muted = theme.muted_foreground;
        let position = self
            .now_playing
            .as_ref()
            .map(|playing| playing.position.clamp(0., 1.))
            .unwrap_or_default();

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
            .child(
                div()
                    .flex()
                    .flex_col()
                    .w(px(240.))
                    .flex_none()
                    .min_w_0()
                    .child(match &self.now_playing {
                        Some(playing) => Label::new(playing.title.clone()).truncate(),
                        None => Label::new("Nothing playing").text_color(muted),
                    })
                    .when_some(self.now_playing.as_ref(), |this, playing| {
                        this.child(
                            Label::new(playing.artists.clone())
                                .text_color(muted)
                                .text_size(px(11.))
                                .truncate(),
                        )
                    }),
            )
            .child(
                div()
                    .flex()
                    .flex_1()
                    .flex_col()
                    .items_center()
                    .gap_2()
                    .child(self.transport(cx))
                    .child(
                        div()
                            .w_full()
                            .max_w(px(420.))
                            .h(px(4.))
                            .rounded_full()
                            .bg(theme.muted)
                            .child(
                                div()
                                    .w(relative(position))
                                    .h_full()
                                    .rounded_full()
                                    .bg(theme.progress_bar),
                            ),
                    ),
            )
            .child(div().w(px(240.)).flex_none())
    }
}
