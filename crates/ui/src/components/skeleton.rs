use std::time::Duration;

use gpui::{
    Animation, AnimationExt as _, App, DefiniteLength, ElementId, IntoElement, Pixels, RenderOnce,
    Window, div, pulsating_between, px, relative,
};

use crate::prelude::*;

const PULSE: Duration = Duration::from_millis(1400);

#[derive(IntoElement)]
pub struct Skeleton {
    id: ElementId,
    width: DefiniteLength,
    height: Pixels,
}

impl Skeleton {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            width: relative(1.).into(),
            height: px(12.),
        }
    }

    pub fn w(mut self, width: impl Into<DefiniteLength>) -> Self {
        self.width = width.into();
        self
    }

    pub fn h(mut self, height: Pixels) -> Self {
        self.height = height;
        self
    }
}

impl RenderOnce for Skeleton {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .w(self.width)
            .h(self.height)
            .rounded_sm()
            .bg(Theme::global(cx).elevated)
            .with_animation(
                self.id,
                Animation::new(PULSE)
                    .repeat()
                    .with_easing(pulsating_between(0.4, 1.0)),
                |this, delta| this.opacity(delta),
            )
    }
}
