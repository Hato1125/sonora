use gpui::{App, FontWeight, IntoElement, Pixels, RenderOnce, SharedString, Window, div, px};

use crate::prelude::*;

#[derive(IntoElement)]
pub struct Label {
    text: SharedString,
    tone: Tone,
    size: Pixels,
    weight: FontWeight,
    truncate: bool,
}

impl Label {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            text: text.into(),
            tone: Tone::default(),
            size: px(13.),
            weight: FontWeight::NORMAL,
            truncate: false,
        }
    }

    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    pub fn size(mut self, size: Pixels) -> Self {
        self.size = size;
        self
    }

    pub fn weight(mut self, weight: FontWeight) -> Self {
        self.weight = weight;
        self
    }

    pub fn truncate(mut self) -> Self {
        self.truncate = true;
        self
    }
}

impl RenderOnce for Label {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let color = self.tone.color(cx.theme());

        div()
            .text_size(self.size)
            .font_weight(self.weight)
            .text_color(color)
            .when(self.truncate, |this| this.truncate())
            .child(self.text)
    }
}
