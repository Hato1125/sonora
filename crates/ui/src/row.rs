use gpui::prelude::*;
use gpui::{AnyElement, App, Hsla, Pixels, SharedString, Window, div, px};

use crate::ExplicitBadge;
use crate::artwork::Artwork;
use crate::metrics::{Text, snapped};
use crate::theme::ActiveTheme as _;

const TITLE: Pixels = px(120.);

#[derive(IntoElement)]
pub struct Row {
    title: SharedString,
    meta: Option<SharedString>,
    cover: Option<String>,
    circle: bool,
    tint: Option<Hsla>,
    trailing: Option<AnyElement>,
    explicit: bool,
}

impl Row {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            meta: None,
            cover: None,
            circle: false,
            tint: None,
            trailing: None,
            explicit: false,
        }
    }

    pub fn cover(mut self, cover: Option<String>) -> Self {
        self.cover = cover;
        self
    }

    pub fn circle(mut self) -> Self {
        self.circle = true;
        self
    }

    pub fn tint(mut self, tint: Hsla) -> Self {
        self.tint = Some(tint);
        self
    }

    pub fn meta(mut self, meta: impl Into<SharedString>) -> Self {
        self.meta = Some(meta.into());
        self
    }

    pub fn explicit(mut self) -> Self {
        self.explicit = true;
        self
    }

    pub fn trailing(mut self, trailing: impl IntoElement) -> Self {
        self.trailing = Some(trailing.into_any_element());
        self
    }
}

impl RenderOnce for Row {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = *cx.theme();
        let height = snapped(theme.metrics.list_row, window);
        let art = theme.metrics.list_row - theme.metrics.pad * 2.;

        div()
            .flex()
            .items_center()
            .flex_none()
            .gap_3()
            .h(height)
            .px_2()
            .rounded(theme.radius)
            .hover(move |style| style.bg(theme.table_hover))
            .child(
                div().flex_none().child(
                    Artwork::new(self.cover)
                        .size(art)
                        .when(self.circle, Artwork::circle),
                ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .min_w(TITLE)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1p5()
                            .min_w_0()
                            .text_color(self.tint.unwrap_or(theme.foreground))
                            .child(div().min_w_0().truncate().child(self.title))
                            .when(self.explicit, |this| {
                                this.child(div().flex_none().child(ExplicitBadge::new()))
                            }),
                    )
                    .children(self.meta.map(|meta| {
                        div()
                            .truncate()
                            .text_size(theme.text(Text::Small))
                            .text_color(theme.muted_foreground)
                            .child(meta)
                    })),
            )
            .children(
                self.trailing
                    .map(|trailing| div().flex_shrink(1.).min_w_0().child(trailing)),
            )
    }
}
