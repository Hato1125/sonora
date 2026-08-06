use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use gpui::prelude::*;
use gpui::{
    App, ClickEvent, Div, ElementId, Interactivity, MouseButton, MouseDownEvent, SharedString,
    Stateful, StyleRefinement, Window, deferred, div, px,
};

use crate::theme::ActiveTheme as _;

const CHECK: &str = "✓";
const SUBMENU_HANDOFF: Duration = Duration::from_millis(120);

type Press = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;
type Dismiss = Box<dyn Fn(&MouseDownEvent, &mut Window, &mut App) + 'static>;
type Action = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

#[derive(Clone, Default)]
pub struct SubmenuState {
    open: Rc<Cell<bool>>,
    generation: Rc<Cell<u64>>,
}

impl SubmenuState {
    fn is_open(&self) -> bool {
        self.open.get()
    }

    fn hover(&self, hovered: bool, cx: &mut App) {
        let generation = self.generation.get().wrapping_add(1);
        self.generation.set(generation);

        if hovered {
            if !self.open.replace(true) {
                cx.refresh_windows();
            }
            return;
        }

        let state = self.clone();
        cx.spawn(async move |cx| {
            cx.background_executor().timer(SUBMENU_HANDOFF).await;
            cx.update(|cx| {
                if state.generation.get() == generation && state.open.replace(false) {
                    cx.refresh_windows();
                }
            });
        })
        .detach();
    }
}

struct Submenu {
    menu: Box<Menu>,
    state: SubmenuState,
}

pub struct MenuItem {
    id: ElementId,
    label: SharedString,
    selected: bool,
    disabled: bool,
    press: Option<Press>,
    submenu: Option<Submenu>,
}

impl MenuItem {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            selected: false,
            disabled: false,
            press: None,
            submenu: None,
        }
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.press = Some(Box::new(handler));
        self
    }

    pub fn submenu(mut self, menu: Menu, state: SubmenuState) -> Self {
        self.submenu = Some(Submenu {
            menu: Box::new(menu),
            state,
        });
        self
    }
}

#[derive(IntoElement)]
pub struct Menu {
    base: Stateful<Div>,
    items: Vec<MenuItem>,
    dismiss: Option<Dismiss>,
    action: Option<Action>,
    priority: usize,
    deferred: bool,
}

impl Menu {
    #[track_caller]
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            base: div().id(id),
            items: Vec::new(),
            dismiss: None,
            action: None,
            priority: 1,
            deferred: true,
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

    pub fn on_action(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.action = Some(Rc::new(handler));
        self
    }

    pub fn priority(mut self, priority: usize) -> Self {
        self.priority = priority;
        self
    }

    fn inline(mut self) -> Self {
        self.deferred = false;
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
            action,
            priority,
            deferred: should_defer,
        } = self;

        let theme = *cx.theme();
        let overrides = std::mem::take(base.style());

        let rows = items.into_iter().map(move |item| {
            let MenuItem {
                id,
                label,
                selected,
                disabled,
                press,
                submenu,
            } = item;
            let action = action.clone();
            let press_action = action.clone();
            let submenu_state = submenu.as_ref().map(|submenu| submenu.state.clone());

            div()
                .id(id)
                .relative()
                .flex()
                .items_center()
                .justify_between()
                .px_3()
                .py_1()
                .rounded(theme.radius)
                .when_else(
                    disabled,
                    |this| this.text_color(theme.muted_foreground).cursor_default(),
                    |this| this.cursor_pointer(),
                )
                .when(selected, |this| this.bg(theme.secondary_active))
                .when(!disabled, |this| {
                    this.hover(move |this| this.bg(theme.secondary_hover))
                })
                .child(label)
                .when(selected, |this| this.child(CHECK))
                .when(submenu.is_some(), |this| this.child("›"))
                .when_some(submenu_state, |this, state| {
                    this.on_hover(move |hovered, _, cx| state.hover(*hovered, cx))
                })
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .when_some(press, |this, press| {
                    this.on_click(move |event, window, cx| {
                        press(event, window, cx);
                        if let Some(action) = press_action.as_ref() {
                            action(event, window, cx);
                        }
                    })
                })
                .when_some(submenu, |this, mut submenu| {
                    if submenu.menu.action.is_none() {
                        submenu.menu.action = action.clone();
                    }
                    let open = submenu.state.is_open();
                    let state = submenu.state.clone();
                    this.child(
                        submenu
                            .menu
                            .inline()
                            .top(px(-4.))
                            .left_full()
                            .ml_1()
                            .when(!open, |this| this.invisible())
                            .on_hover(move |hovered, _, cx| state.hover(*hovered, cx)),
                    )
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

        if should_defer {
            deferred(menu).with_priority(priority).into_any_element()
        } else {
            menu.into_any_element()
        }
    }
}
