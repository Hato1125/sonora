use crate::skeleton::Skeleton;
use crate::theme::ActiveTheme as _;
use gpui::prelude::*;
use gpui::{
    AnyElement, App, Div, IntoElement, Pixels, SharedString, SharedUri, Window, div, img, px, svg,
};

const FALLBACK_ICON: &str = "icons/music.svg";

#[derive(IntoElement)]
pub struct Artwork {
    url: Option<SharedString>,
    size: Pixels,
    rounded: Pixels,
}

impl Artwork {
    pub fn new(url: Option<impl Into<SharedString>>) -> Self {
        Self {
            url: url.map(Into::into),
            size: px(28.),
            rounded: px(4.),
        }
    }

    pub fn size(mut self, size: Pixels) -> Self {
        self.size = size;
        self
    }

    pub fn rounded(mut self, rounded: Pixels) -> Self {
        self.rounded = rounded;
        self
    }
}

impl RenderOnce for Artwork {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        let size = self.size;
        let rounded = self.rounded;
        let placeholder = move || blank(size, rounded, muted).into_any_element();

        let Some(url) = self.url else {
            return placeholder();
        };

        img(SharedUri::from(url.to_string()))
            .size(size)
            .rounded(rounded)
            .with_loading(move || skeleton(size, rounded))
            .with_fallback(placeholder)
            .into_any_element()
    }
}

fn skeleton(size: Pixels, rounded: Pixels) -> AnyElement {
    Skeleton::new()
        .size(size)
        .rounded(rounded)
        .into_any_element()
}

fn blank(size: Pixels, rounded: Pixels, muted: gpui::Hsla) -> Div {
    div()
        .size(size)
        .rounded(rounded)
        .bg(muted.opacity(0.12))
        .flex()
        .items_center()
        .justify_center()
        .child(
            svg()
                .path(FALLBACK_ICON)
                .size(size * 0.46)
                .text_color(muted.opacity(0.5)),
        )
}
