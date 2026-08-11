// SPDX-License-Identifier: GPL-3.0-or-later

use gpui::prelude::*;
use gpui::{App, MouseButton, SharedString, Window, div};
use i18n::t;
use ui::{ActiveTheme as _, Button, Shield, Text, heading};

type Pick = Box<dyn Fn(&SharedString, &mut Window, &mut App)>;
type Cancel = Box<dyn Fn(&(), &mut Window, &mut App)>;

#[derive(IntoElement)]
pub(crate) struct BrowserPicker {
    names: Vec<SharedString>,
    pick: Option<Pick>,
    cancel: Option<Cancel>,
}

impl BrowserPicker {
    pub(crate) fn new(names: Vec<SharedString>) -> Self {
        Self {
            names,
            pick: None,
            cancel: None,
        }
    }

    pub(crate) fn on_pick(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.pick = Some(Box::new(handler));
        self
    }

    pub(crate) fn on_cancel(
        mut self,
        handler: impl Fn(&(), &mut Window, &mut App) + 'static,
    ) -> Self {
        self.cancel = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for BrowserPicker {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = *cx.theme();
        let Self {
            names,
            pick,
            cancel,
        } = self;
        let pick: Option<std::rc::Rc<Pick>> = pick.map(std::rc::Rc::new);
        let cancel: Option<std::rc::Rc<Cancel>> = cancel.map(std::rc::Rc::new);
        let dismissed = cancel.clone();

        div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .child(
                Shield::new("browser-picker-shield")
                    .absolute()
                    .inset_0()
                    .bg(theme.background.opacity(0.8))
                    .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                        if let Some(dismissed) = &dismissed {
                            dismissed(&(), window, cx);
                        }
                    }),
            )
            .child(
                div()
                    .relative()
                    .w(theme.metrics.cover * 2.4)
                    .flex()
                    .flex_col()
                    .gap_4()
                    .p(theme.metrics.inset)
                    .rounded(theme.radius)
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.popover)
                    .child(heading(t!("login-browser-title"), cx))
                    .child(
                        div()
                            .text_size(theme.text(Text::Small))
                            .text_color(theme.muted_foreground)
                            .child(t!("login-browser-detail")),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .children(names.into_iter().map(|name| {
                                let pick = pick.clone();
                                Button::new(SharedString::from(format!("browser-{name}")))
                                    .label(name.clone())
                                    .outline()
                                    .w_full()
                                    .on_click(move |_, window, cx| {
                                        if let Some(pick) = &pick {
                                            pick(&name, window, cx);
                                        }
                                    })
                            })),
                    )
                    .child(
                        div().flex().justify_end().child(
                            Button::new("cancel-browser")
                                .ghost()
                                .label(t!("common-cancel"))
                                .on_click(move |_, window, cx| {
                                    if let Some(cancel) = &cancel {
                                        cancel(&(), window, cx);
                                    }
                                }),
                        ),
                    ),
            )
    }
}
