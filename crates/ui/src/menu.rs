use gpui::prelude::*;
use gpui::{
    App, ClickEvent, Div, ElementId, Interactivity, MouseButton, MouseDownEvent, SharedString,
    Stateful, StyleRefinement, Window, deferred, div,
};

use crate::theme::ActiveTheme as _;

const CHECK: &str = "✓";

type Press = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;
type Dismiss = Box<dyn Fn(&MouseDownEvent, &mut Window, &mut App) + 'static>;

pub struct MenuItem {
    id: ElementId,
    label: SharedString,
    selected: bool,
    press: Option<Press>,
}

impl MenuItem {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            selected: false,
            press: None,
        }
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.press = Some(Box::new(handler));
        self
    }
}

#[derive(IntoElement)]
pub struct Menu {
    base: Stateful<Div>,
    items: Vec<MenuItem>,
    dismiss: Option<Dismiss>,
    priority: usize,
}

impl Menu {
    #[track_caller]
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: div().id(id),
            items: Vec::new(),
            dismiss: None,
            priority: 1,
        }
    }

    pub fn item(mut self, item: MenuItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn items(mut self, items: impl IntoIterator<Item = MenuItem>) -> Self {
        self.items.extend(items);
        self
    }

    pub fn on_dismiss(
        mut self,
        handler: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.dismiss = Some(Box::new(handler));
        self
    }

    pub fn priority(mut self, priority: usize) -> Self {
        self.priority = priority;
        self
    }
}

impl Styled for Menu {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl InteractiveElement for Menu {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for Menu {}

impl RenderOnce for Menu {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Self {
            mut base,
            items,
            dismiss,
            priority,
        } = self;

        let theme = *cx.theme();
        let overrides = std::mem::take(base.style());

        let rows = items.into_iter().map(move |item| {
            let MenuItem {
                id,
                label,
                selected,
                press,
            } = item;

            div()
                .id(id)
                .flex()
                .items_center()
                .justify_between()
                .px_3()
                .py_1()
                .rounded(theme.radius)
                .cursor_pointer()
                .when(selected, |this| this.bg(theme.secondary_active))
                .hover(move |this| this.bg(theme.secondary_hover))
                .child(label)
                .when(selected, |this| this.child(CHECK))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .when_some(press, |this, press| {
                    this.on_click(move |event, window, cx| press(event, window, cx))
                })
        });

        let mut menu = base
            .absolute()
            .flex()
            .flex_col()
            .p_1()
            .rounded(theme.radius)
            .border_1()
            .gap_1()
            .border_color(theme.border)
            .bg(theme.secondary)
            .text_color(theme.popover_foreground)
            .occlude()
            .when_some(dismiss, |this, dismiss| {
                this.on_mouse_down_out(move |event, window, cx| dismiss(event, window, cx))
            })
            .children(rows);

        menu.style().refine(&overrides);

        deferred(menu).with_priority(priority)
    }
}
