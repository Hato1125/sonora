use gpui::prelude::*;
use gpui::{
    App, ClickEvent, Div, ElementId, Interactivity, Stateful, StyleRefinement, Window, div, px,
};

use crate::motion::{Motion, Motioned as _};
use crate::theme::ActiveTheme as _;

const SCALE: f32 = 0.85;
const INSET: f32 = 2.;
const WIDTH: f32 = 1.75;
const BORDER: f32 = 1.;

type Click = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

#[derive(IntoElement)]
pub struct Switch {
    base: Stateful<Div>,
    checked: bool,
    disabled: bool,
    on_click: Option<Click>,
}

impl Switch {
    #[track_caller]
    pub fn new(id: impl Into<ElementId>, checked: bool) -> Self {
        Self {
            base: div().id(id),
            checked,
            disabled: false,
            on_click: None,
        }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
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

impl Styled for Switch {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl InteractiveElement for Switch {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl RenderOnce for Switch {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Self {
            mut base,
            checked,
            disabled,
            on_click,
        } = self;
        let theme = cx.theme();
        let height = theme.metrics.control_small * SCALE;
        let thumb = height - px(INSET * 2.);
        let background = match checked {
            true => theme.primary,
            false => theme.muted,
        };
        let overrides = std::mem::take(base.style());

        let travel = height * (WIDTH - 1.) - px(BORDER * 2.);
        let (from, to) = match checked {
            true => (0., 1.),
            false => (1., 0.),
        };
        let (track_was, track_is) = match checked {
            true => (theme.muted, theme.primary),
            false => (theme.primary, theme.muted),
        };
        let (edge_was, edge_is) = match checked {
            true => (theme.border, theme.primary),
            false => (theme.primary, theme.border),
        };
        let (hover_was, hover_is) = match checked {
            true => (theme.secondary_hover, theme.primary_hover),
            false => (theme.primary_hover, theme.secondary_hover),
        };
        let (knob_was, knob_is) = match checked {
            true => (theme.muted_foreground, theme.primary_foreground),
            false => (theme.primary_foreground, theme.muted_foreground),
        };

        let mut switch = base
            .flex()
            .flex_none()
            .items_center()
            .w(height * WIDTH)
            .h(height)
            .p(px(INSET))
            .rounded(height / 2.)
            .bg(background)
            .border_1()
            .border_color(match checked {
                true => theme.primary,
                false => theme.border,
            })
            .when(disabled, |this| this.opacity(0.4))
            .when(!disabled, |this| this.cursor_pointer())
            .child(div().size(thumb).flex_none().rounded(thumb / 2.).motion(
                ("thumb", usize::from(checked)),
                Motion::Control,
                move |knob, t| {
                    knob.ml(travel * (from + (to - from) * t))
                        .bg(crate::motion::mix(knob_was, knob_is, t))
                },
            ));

        switch.style().refine(&overrides);
        if !disabled && let Some(handler) = on_click {
            switch = switch.on_click(handler);
        }
        switch.motion(
            ("track", usize::from(checked)),
            Motion::Control,
            move |track, t| {
                let hover = crate::motion::mix(hover_was, hover_is, t);
                track
                    .bg(crate::motion::mix(track_was, track_is, t))
                    .border_color(crate::motion::mix(edge_was, edge_is, t))
                    .when(!disabled, move |this| {
                        this.hover(move |style| style.bg(hover))
                    })
            },
        )
    }
}
