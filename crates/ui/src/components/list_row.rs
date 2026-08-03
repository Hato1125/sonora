use gpui::{App, IntoElement, RenderOnce, SharedString, Window, div, px};

use crate::prelude::*;

#[derive(IntoElement)]
pub struct ListRow {
    index: usize,
    primary: Option<SharedString>,
    secondary: Option<SharedString>,
    meta: Option<SharedString>,
    trailing: Option<SharedString>,
    loading: bool,
}

impl ListRow {
    pub fn new(index: usize, primary: impl Into<SharedString>) -> Self {
        Self {
            index,
            primary: Some(primary.into()),
            secondary: None,
            meta: None,
            trailing: None,
            loading: false,
        }
    }

    pub fn loading(index: usize) -> Self {
        Self {
            index,
            primary: None,
            secondary: None,
            meta: None,
            trailing: None,
            loading: true,
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

    fn stagger(&self) -> f32 {
        match self.index % 3 {
            0 => 1.0,
            1 => 0.78,
            _ => 0.62,
        }
    }
}

impl RenderOnce for ListRow {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::global(cx).clone();
        let stagger = self.stagger();
        let loading = self.loading;
        let index = self.index;

        div()
            .id(self.index)
            .flex()
            .w_full()
            .items_center()
            .gap_1()
            .h(px(44.))
            .px_5()
            .py_3()
            .when(!loading, |this| {
                this.hover(move |style| style.bg(theme.surface))
            })
            .child(
                div()
                    .w(px(28.))
                    .flex_none()
                    .when(loading, |this| {
                        this.child(Skeleton::new("index").w(px(12.)).h(px(10.)))
                    })
                    .when(!loading, |this| {
                        this.child(Label::new(format!("{}", index + 1)).tone(Tone::Muted))
                    }),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .when(loading, |this| {
                        this.gap_1p5()
                            .child(Skeleton::new("primary").w(px(220. * stagger)).h(px(11.)))
                            .child(Skeleton::new("secondary").w(px(120. * stagger)).h(px(9.)))
                    })
                    .when_some(self.primary, |this, primary| {
                        this.child(Label::new(primary).truncate())
                    })
                    .when_some(self.secondary, |this, secondary| {
                        this.child(
                            Label::new(secondary)
                                .tone(Tone::Muted)
                                .size(px(11.))
                                .truncate(),
                        )
                    }),
            )
            .child(
                div()
                    .w(px(220.))
                    .flex_none()
                    .when(loading, |this| {
                        this.child(Skeleton::new("meta").w(px(150. * stagger)).h(px(10.)))
                    })
                    .when_some(self.meta, |this, meta| {
                        this.child(Label::new(meta).tone(Tone::Muted).truncate())
                    }),
            )
            .child(
                div()
                    .w(px(72.))
                    .flex_none()
                    .flex()
                    .justify_end()
                    .when(loading, |this| {
                        this.child(Skeleton::new("trailing").w(px(32.)).h(px(10.)))
                    })
                    .when_some(self.trailing, |this, trailing| {
                        this.child(Label::new(trailing).tone(Tone::Muted))
                    }),
            )
    }
}
