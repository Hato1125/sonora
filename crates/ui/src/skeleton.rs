use std::time::Duration;

use gpui::prelude::*;
use gpui::{
    Animation, AnimationExt as _, App, Length, Pixels, SharedString, Window, div, ease_in_out,
};

use crate::theme::ActiveTheme as _;

const PULSE: Duration = Duration::from_millis(1400);

#[derive(IntoElement)]
pub struct Skeleton {
    width: Option<Length>,
    height: Option<Length>,
    rounded: Option<Pixels>,
    circle: bool,
}

impl Default for Skeleton {
    fn default() -> Self {
        Self::new()
    }
}

impl Skeleton {
    pub fn new() -> Self {
        Self {
            width: None,
            height: None,
            rounded: None,
            circle: false,
        }
    }

    pub fn w(mut self, width: impl Into<Length>) -> Self {
        self.width = Some(width.into());
        self
    }

    pub fn h(mut self, height: impl Into<Length>) -> Self {
        self.height = Some(height.into());
        self
    }

    pub fn size(mut self, size: Pixels) -> Self {
        self.width = Some(size.into());
        self.height = Some(size.into());
        self
    }

    pub fn rounded(mut self, radius: Pixels) -> Self {
        self.rounded = Some(radius);
        self
    }

    pub fn circle(mut self) -> Self {
        self.circle = true;
        self
    }
}

impl RenderOnce for Skeleton {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let radius = self.rounded.unwrap_or(cx.theme().radius);

        div()
            .bg(cx.theme().muted)
            .when_some(self.width, |this, width| this.w(width))
            .when_some(self.height, |this, height| this.h(height))
            .when_else(
                self.circle,
                |this| this.rounded_full(),
                |this| this.rounded(radius),
            )
            .with_animation(
                "skeleton",
                Animation::new(PULSE).repeat().with_easing(ease_in_out),
                |this, delta| {
                    let fade = 1. - (delta * std::f32::consts::TAU).cos().abs() * 0.5;
                    this.opacity(0.4 + fade * 0.3)
                },
            )
    }
}

#[derive(IntoElement)]
pub struct Initials {
    name: SharedString,
    size: Pixels,
}

impl Initials {
    pub fn new(name: impl Into<SharedString>, size: Pixels) -> Self {
        Self {
            name: name.into(),
            size,
        }
    }
}

impl RenderOnce for Initials {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let letters: String = self
            .name
            .split_whitespace()
            .filter_map(|word| word.chars().next())
            .take(2)
            .collect::<String>()
            .to_uppercase();

        div()
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .size(self.size)
            .rounded_full()
            .bg(cx.theme().secondary)
            .text_size(self.size * 0.34)
            .text_color(cx.theme().muted_foreground)
            .child(letters)
    }
}
