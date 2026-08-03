use gpui::{App, ClickEvent, ElementId, IntoElement, RenderOnce, SharedString, Window, div, px};

use crate::prelude::*;

type ClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

#[derive(IntoElement)]
pub struct Tab {
    id: ElementId,
    label: SharedString,
    selected: bool,
    on_click: Option<ClickHandler>,
}

impl Tab {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            selected: false,
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
            .cursor_pointer()
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
            .hover(move |style| style.text_color(theme.text))
            .child(self.label);

        if let Some(handler) = self.on_click {
            tab = tab.on_click(move |event, window, cx| handler(event, window, cx));
        }

        tab
    }
}
