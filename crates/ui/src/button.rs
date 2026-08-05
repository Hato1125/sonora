use gpui::prelude::*;
use gpui::{App, ClickEvent, ElementId, Hsla, MouseButton, SharedString, Window, div, px, svg};

use crate::theme::ActiveTheme as _;

#[derive(Clone, Copy, PartialEq)]
enum Variant {
    Secondary,
    Ghost,
    Outline,
    Primary,
    Danger,
}

#[derive(IntoElement)]
pub struct Button {
    id: ElementId,
    label: Option<SharedString>,
    icon: Option<SharedString>,
    variant: Variant,
    small: bool,
    disabled: bool,
    selected: bool,
    on_click: Option<Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>>,
}

impl Button {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            label: None,
            icon: None,
            variant: Variant::Secondary,
            small: false,
            disabled: false,
            selected: false,
            on_click: None,
        }
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn icon(mut self, path: impl Into<SharedString>) -> Self {
        self.icon = Some(path.into());
        self
    }

    pub fn ghost(mut self) -> Self {
        self.variant = Variant::Ghost;
        self
    }

    pub fn outline(mut self) -> Self {
        self.variant = Variant::Outline;
        self
    }

    pub fn primary(mut self) -> Self {
        self.variant = Variant::Primary;
        self
    }

    pub fn danger(mut self) -> Self {
        self.variant = Variant::Danger;
        self
    }

    pub fn small(mut self) -> Self {
        self.small = true;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }
}

struct Palette {
    background: Option<Hsla>,
    hover: Option<Hsla>,
    active: Option<Hsla>,
    foreground: Hsla,
    border: Option<Hsla>,
}

impl RenderOnce for Button {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let palette = match self.variant {
            Variant::Secondary => Palette {
                background: Some(theme.secondary),
                hover: Some(theme.secondary_hover),
                active: Some(theme.secondary_active),
                foreground: theme.foreground,
                border: Some(theme.border),
            },
            Variant::Ghost => Palette {
                background: None,
                hover: Some(theme.secondary),
                active: Some(theme.secondary_active),
                foreground: theme.foreground,
                border: None,
            },
            Variant::Outline => Palette {
                background: None,
                hover: Some(theme.secondary),
                active: Some(theme.secondary_active),
                foreground: theme.foreground,
                border: Some(theme.border),
            },
            Variant::Primary => Palette {
                background: Some(theme.primary),
                hover: Some(theme.primary_hover),
                active: Some(theme.primary_hover),
                foreground: theme.primary_foreground,
                border: None,
            },
            Variant::Danger => Palette {
                background: Some(theme.danger),
                hover: Some(theme.danger_hover),
                active: Some(theme.danger_hover),
                foreground: theme.danger_foreground,
                border: None,
            },
        };

        let selected_background = theme.secondary_active;
        let radius = theme.radius;
        let interactive = !self.disabled;
        let (height, padding, gap) = if self.small {
            (px(26.), px(8.), px(4.))
        } else {
            (px(32.), px(12.), px(6.))
        };

        div()
            .id(self.id)
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .gap(gap)
            .h(height)
            .px(padding)
            .rounded(radius)
            .text_color(palette.foreground)
            .when(self.small, |this| this.text_size(px(12.)))
            .when(self.disabled, |this| this.opacity(0.4))
            .when_some(palette.background, |this, background| this.bg(background))
            .when(self.selected, |this| this.bg(selected_background))
            .when_some(palette.border, |this, border| {
                this.border_1().border_color(border)
            })
            .when(interactive, |this| {
                this.cursor_pointer()
                    .when_some(palette.hover, |this, hover| {
                        this.hover(move |style| style.bg(hover))
                    })
                    .when_some(palette.active, |this, active| {
                        this.active(move |style| style.bg(active))
                    })
            })
            .when_some(self.icon, |this, path| {
                this.child(
                    svg()
                        .path(path)
                        .size(px(16.))
                        .flex_none()
                        .text_color(palette.foreground),
                )
            })
            .when_some(self.label, |this, label| this.child(label))
            .when(interactive, |this| {
                this.when_some(self.on_click, |this, handler| {
                    this.on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .on_click(move |event, window, cx| handler(event, window, cx))
                })
            })
    }
}
