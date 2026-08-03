use gpui::{App, ClickEvent, ElementId, IntoElement, RenderOnce, SharedString, Window, div, px};

use crate::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonVariant {
    #[default]
    Primary,
    Ghost,
    Destructive,
}

type ClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

#[derive(IntoElement)]
pub struct Button {
    id: ElementId,
    label: SharedString,
    variant: ButtonVariant,
    disabled: bool,
    on_click: Option<ClickHandler>,
}

impl Button {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            variant: ButtonVariant::default(),
            disabled: false,
            on_click: None,
        }
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }
}

impl Disableable for Button {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Clickable for Button {
    fn on_click(mut self, handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for Button {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = Theme::global(cx);
        let (background, hovered, foreground) = match self.variant {
            ButtonVariant::Primary => (theme.accent, theme.accent_hover, theme.on_accent),
            ButtonVariant::Ghost => (theme.elevated, theme.border, theme.text),
            ButtonVariant::Destructive => (
                theme.destructive,
                theme.destructive_hover,
                theme.on_destructive,
            ),
        };

        let mut button = div()
            .id(self.id)
            .flex()
            .items_center()
            .justify_center()
            .h(px(34.))
            .px_4()
            .rounded_full()
            .text_size(px(13.))
            .bg(if self.disabled {
                theme.surface
            } else {
                background
            })
            .text_color(if self.disabled {
                theme.text_muted
            } else {
                foreground
            });

        if !self.disabled {
            button = button
                .cursor_pointer()
                .hover(move |style| style.bg(hovered));

            if let Some(handler) = self.on_click {
                button = button.on_click(move |event, window, cx| handler(event, window, cx));
            }
        }

        button.child(self.label)
    }
}
