use gpui::{App, ClickEvent, ElementId, IntoElement, RenderOnce, SharedString, Window, div, px};

use crate::prelude::*;

type ClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

#[derive(IntoElement)]
pub struct Tab {
    id: ElementId,
    label: SharedString,
    selected: bool,
    disabled: bool,
    on_click: Option<ClickHandler>,
}

impl Tab {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            selected: false,
            disabled: false,
            on_click: None,
        }
    }
}

impl Selectable for Tab {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

impl Disableable for Tab {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self.on_click = None;
        self
    }
}

impl Clickable for Tab {
    fn on_click(mut self, handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for Tab {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::global(cx).clone();

        let mut tab = div()
            .id(self.id)
            .px_3()
            .py_1p5()
            .rounded_full()
            .when_else(
                self.disabled,
                |style| style.cursor_not_allowed(),
                |style| style.cursor_pointer(),
            )
            .text_size(px(12.))
            .bg(if self.selected {
                theme.elevated
            } else {
                theme.background
            })
            .text_color(if self.selected {
                theme.text
            } else {
                theme.text_muted
            })
            .child(self.label);

        if let Some(handler) = self.on_click {
            tab = tab.on_click(move |event, window, cx| handler(event, window, cx));
        }

        tab
    }
}
