use gpui::{App, IntoElement, RenderOnce, SharedString, Window, div, px};

use crate::prelude::*;

#[derive(IntoElement)]
pub struct Message {
    text: SharedString,
    tone: Tone,
}

impl Message {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            text: text.into(),
            tone: Tone::Muted,
        }
    }

    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }
}

impl RenderOnce for Message {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .flex()
            .flex_1()
            .items_center()
            .justify_center()
            .px_6()
            .child(
                div()
                    .max_w(px(560.))
                    .text_center()
                    .child(Label::new(self.text).tone(self.tone)),
            )
    }
}
