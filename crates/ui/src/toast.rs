// SPDX-License-Identifier: GPL-3.0-or-later

use gpui::prelude::*;
use gpui::{App, ClickEvent, ElementId, Pixels, SharedString, Window, div, px, svg};

use crate::button::Button;
use crate::metrics::Text;
use crate::theme::ActiveTheme as _;

const ICON: Pixels = px(16.);

type Press = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

#[derive(IntoElement)]
pub struct Toast {
    id: ElementId,
    message: SharedString,
    failed: bool,
    dismiss: Option<Press>,
}

impl Toast {
    #[track_caller]
    pub fn new(id: impl Into<ElementId>, message: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            message: message.into(),
            failed: false,
            dismiss: None,
        }
    }

    pub fn failed(mut self) -> Self {
        self.failed = true;
        self
    }

    pub fn on_dismiss(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.dismiss = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for Toast {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let Self {
            id,
            message,
            failed,
            dismiss,
        } = self;

        let theme = *cx.theme();
        let (tint, icon) = match failed {
            true => (theme.danger, "icons/circle-alert.svg"),
            false => (theme.primary, "icons/circle-check.svg"),
        };

        div()
            .id(id)
            .flex()
            .items_center()
            .justify_between()
            .gap_4()
            .py(theme.metrics.pad)
            .pl(theme.metrics.pad * 2)
            .pr(theme.metrics.pad)
            .rounded(theme.radius)
            .border_1()
            .shadow_md()
            .border_color(theme.border)
            .bg(theme.popover)
            .text_size(theme.text(Text::Small))
            .text_color(theme.foreground)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .min_w_0()
                    .child(svg().path(icon).size(ICON).flex_none().text_color(tint))
                    .child(div().min_w_0().truncate().child(message)),
            )
            .children(dismiss.map(|dismiss| {
                Button::new("dismiss-toast")
                    .ghost()
                    .small()
                    .icon("icons/x.svg")
                    .on_click(move |event, window, cx| dismiss(event, window, cx))
            }))
    }
}
