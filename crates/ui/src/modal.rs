use std::rc::Rc;

use gpui::prelude::*;
use gpui::{AnyElement, App, ElementId, MouseButton, Pixels, SharedString, Window, div};

use crate::label::heading;
use crate::metrics::Text;
use crate::shield::Shield;
use crate::theme::ActiveTheme as _;

const BACKDROP: f32 = 0.8;
const WIDTH: f32 = 2.4;

type Dismiss = Rc<dyn Fn(&(), &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct Modal {
    id: ElementId,
    title: SharedString,
    detail: Option<SharedString>,
    body: Vec<AnyElement>,
    actions: Vec<AnyElement>,
    width: Option<Pixels>,
    dismiss: Option<Dismiss>,
}

impl Modal {
    #[track_caller]
    pub fn new(id: impl Into<ElementId>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            detail: None,
            body: Vec::new(),
            actions: Vec::new(),
            width: None,
            dismiss: None,
        }
    }

    pub fn detail(mut self, detail: impl Into<SharedString>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn width(mut self, width: Pixels) -> Self {
        self.width = Some(width);
        self
    }

    pub fn child(mut self, body: impl IntoElement) -> Self {
        self.body.push(body.into_any_element());
        self
    }

    pub fn action(mut self, action: impl IntoElement) -> Self {
        self.actions.push(action.into_any_element());
        self
    }

    pub fn on_dismiss(mut self, handler: impl Fn(&(), &mut Window, &mut App) + 'static) -> Self {
        self.dismiss = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Modal {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = *cx.theme();
        let Self {
            id,
            title,
            detail,
            body,
            actions,
            width,
            dismiss,
        } = self;
        let outside = dismiss.clone();

        div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .child(
                Shield::new(id)
                    .absolute()
                    .inset_0()
                    .bg(theme.background.opacity(BACKDROP))
                    .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
                    .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                        cx.stop_propagation();
                        if let Some(outside) = &outside {
                            outside(&(), window, cx);
                        }
                    }),
            )
            .child(
                div()
                    .relative()
                    .occlude()
                    .w(width.unwrap_or(theme.metrics.cover * WIDTH))
                    .flex()
                    .flex_col()
                    .gap_4()
                    .p(theme.metrics.inset)
                    .rounded(theme.radius)
                    .border_1()
                    .border_color(theme.border)
                    .bg(theme.popover)
                    .child(heading(title, cx))
                    .when_some(detail, |this, detail| {
                        this.child(
                            div()
                                .text_size(theme.text(Text::Small))
                                .text_color(theme.muted_foreground)
                                .child(detail),
                        )
                    })
                    .children(body)
                    .when(!actions.is_empty(), |this| {
                        this.child(div().flex().justify_end().gap_2().children(actions))
                    }),
            )
    }
}
