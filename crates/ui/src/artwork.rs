use crate::skeleton::Skeleton;
use crate::theme::ActiveTheme as _;
use gpui::prelude::*;
use gpui::{
    App, Div, Hsla, Interactivity, Pixels, SharedString, SharedUri, StyleRefinement, Styled,
    Window, div, img, px, svg,
};

const FALLBACK_ICON: &str = "icons/music.svg";

#[derive(IntoElement)]
pub struct Artwork {
    url: Option<SharedString>,
    size: Pixels,
    rounded: Pixels,
    interactivity: Interactivity,
}

impl Artwork {
    #[track_caller]
    pub fn new(url: Option<impl Into<SharedString>>) -> Self {
        Self {
            url: url.map(Into::into),
            size: px(28.),
            rounded: px(4.),
            interactivity: Interactivity::new(),
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

impl Styled for Artwork {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.interactivity.base_style
    }
}

impl InteractiveElement for Artwork {
    fn interactivity(&mut self) -> &mut Interactivity {
        &mut self.interactivity
    }
}

impl RenderOnce for Artwork {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Self {
            url,
            size,
            rounded,
            interactivity,
        } = self;
        let muted = cx.theme().muted_foreground;
        let placeholder = move || blank(size, rounded, muted).into_any_element();

        match url {
            Some(url) => refined(
                img(SharedUri::from(url.to_string()))
                    .size(size)
                    .rounded(rounded)
                    .with_loading(move || {
                        Skeleton::new()
                            .size(size)
                            .rounded(rounded)
                            .into_any_element()
                    })
                    .with_fallback(placeholder),
                interactivity,
            )
            .into_any_element(),
            None => refined(blank(size, rounded, muted), interactivity).into_any_element(),
        }
    }
}

fn refined<T: Styled + InteractiveElement>(mut element: T, mut caller: Interactivity) -> T {
    let mut style = std::mem::take(element.style());
    style.refine(&caller.base_style);
    *caller.base_style = style;
    *element.interactivity() = caller;
    element
}

fn blank(size: Pixels, rounded: Pixels, muted: Hsla) -> Div {
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
