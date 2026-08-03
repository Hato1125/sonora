use gpui::{App, IntoElement, RenderOnce, SharedString, Window, div, px};

use crate::prelude::*;

#[derive(IntoElement)]
pub struct ListRow {
    index: usize,
    primary: SharedString,
    secondary: Option<SharedString>,
    meta: Option<SharedString>,
    trailing: Option<SharedString>,
}

impl ListRow {
    pub fn new(index: usize, primary: impl Into<SharedString>) -> Self {
        Self {
            index,
            primary: primary.into(),
            secondary: None,
            meta: None,
            trailing: None,
        }
    }

    pub fn secondary(mut self, secondary: impl Into<SharedString>) -> Self {
        self.secondary = Some(secondary.into());
        self
    }

    pub fn meta(mut self, meta: impl Into<SharedString>) -> Self {
        self.meta = Some(meta.into());
        self
    }

    pub fn trailing(mut self, trailing: impl Into<SharedString>) -> Self {
        self.trailing = Some(trailing.into());
        self
    }
}

impl RenderOnce for ListRow {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::global(cx).clone();

        div()
            .id(self.index)
            .flex()
            .w_full()
            .items_center()
            .gap_3()
            .h(px(44.))
            .px_5()
            .hover(move |style| style.bg(theme.surface))
            .child(
                div()
                    .w(px(28.))
                    .flex_none()
                    .child(Label::new(format!("{}", self.index + 1)).tone(Tone::Muted)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .child(Label::new(self.primary).truncate())
                    .when_some(self.secondary, |this, secondary| {
                        this.child(
                            Label::new(secondary)
                                .tone(Tone::Muted)
                                .size(px(11.))
                                .truncate(),
                        )
                    }),
            )
            .when_some(self.meta, |this, meta| {
                this.child(
                    div()
                        .w(px(220.))
                        .flex_none()
                        .child(Label::new(meta).tone(Tone::Muted).truncate()),
                )
            })
            .when_some(self.trailing, |this, trailing| {
                this.child(
                    div()
                        .w(px(72.))
                        .flex_none()
                        .text_right()
                        .child(Label::new(trailing).tone(Tone::Muted)),
                )
            })
    }
}
