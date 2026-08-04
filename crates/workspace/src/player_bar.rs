use gpui::{Context, Render};
use gpui_component::Icon;
use gpui_component::button::{Button, ButtonVariants as _};
use ui::prelude::*;

const HEIGHT: f32 = 68.;

pub struct PlayerBar {
    now_playing: Option<NowPlaying>,
}

pub struct NowPlaying {
    pub title: SharedString,
    pub artists: SharedString,
    pub position: f32,
}

impl PlayerBar {
    pub fn new() -> Self {
        Self { now_playing: None }
    }

    pub fn set_now_playing(&mut self, now_playing: Option<NowPlaying>, cx: &mut Context<Self>) {
        self.now_playing = now_playing;
        cx.notify();
    }
}

impl Default for PlayerBar {
    fn default() -> Self {
        Self::new()
    }
}

impl Render for PlayerBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
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
                        None => Label::new("Nothing playing").tone(Tone::Muted),
                    })
                    .when_some(self.now_playing.as_ref(), |this, playing| {
                        this.child(
                            Label::new(playing.artists.clone())
                                .tone(Tone::Muted)
                                .size(px(11.))
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
                    .child(
                        div().flex().items_center().gap_2().children(
                            [
                                ("previous", "icons/skip-back.svg"),
                                ("play", "icons/play.svg"),
                                ("next", "icons/skip-forward.svg"),
                            ]
                            .into_iter()
                            .map(|(id, icon)| {
                                Button::new(id).ghost().icon(Icon::default().path(icon))
                            }),
                        ),
                    )
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
