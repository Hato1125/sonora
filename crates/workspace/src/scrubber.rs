use std::cell::Cell;
use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    App, Bounds, DragMoveEvent, Empty, Hsla, MouseButton, MouseDownEvent, MouseUpEvent, Pixels,
    Render, SharedString, Window, canvas, div, px, relative,
};

const TRACK: f32 = 4.;
const THUMB: f32 = 12.;
const HIT: f32 = 16.;

#[derive(Clone)]
struct Grab(SharedString);

impl Render for Grab {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

pub struct ScrubberState {
    id: SharedString,
    bounds: Rc<Cell<Bounds<Pixels>>>,
}

impl ScrubberState {
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            bounds: Rc::new(Cell::new(Bounds::default())),
        }
    }

    fn fraction_at(&self, x: Pixels) -> f32 {
        let bounds = self.bounds.get();
        if bounds.size.width <= px(0.) {
            return 0.;
        }
        ((x - bounds.origin.x) / bounds.size.width).clamp(0., 1.)
    }
}

#[derive(IntoElement)]
pub struct Scrubber {
    id: SharedString,
    bounds: Rc<Cell<Bounds<Pixels>>>,
    fraction: f32,
    filled: Hsla,
    empty: Hsla,
    thumb: Hsla,
    enabled: bool,
    on_move: Option<Box<dyn Fn(&f32, &mut Window, &mut App) + 'static>>,
    on_release: Option<Box<dyn Fn(&MouseUpEvent, &mut Window, &mut App) + 'static>>,
}

impl Scrubber {
    pub fn new(state: &ScrubberState, fraction: f32) -> Self {
        Self {
            id: state.id.clone(),
            bounds: state.bounds.clone(),
            fraction: fraction.clamp(0., 1.),
            filled: gpui::white(),
            empty: gpui::black(),
            thumb: gpui::white(),
            enabled: true,
            on_move: None,
            on_release: None,
        }
    }

    pub fn colors(mut self, filled: Hsla, empty: Hsla, thumb: Hsla) -> Self {
        self.filled = filled;
        self.empty = empty;
        self.thumb = thumb;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn on_move(mut self, handler: impl Fn(&f32, &mut Window, &mut App) + 'static) -> Self {
        self.on_move = Some(Box::new(handler));
        self
    }

    pub fn on_release(
        mut self,
        handler: impl Fn(&MouseUpEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_release = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for Scrubber {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let Self {
            id,
            bounds,
            fraction,
            filled,
            empty,
            thumb,
            enabled,
            on_move,
            on_release,
        } = self;

        let state = Rc::new(ScrubberState {
            id: id.clone(),
            bounds: bounds.clone(),
        });
        let on_move = on_move.map(Rc::new);
        let on_release = on_release.map(Rc::new);

        let down = {
            let state = state.clone();
            let on_move = on_move.clone();
            move |event: &MouseDownEvent, window: &mut Window, cx: &mut App| {
                if let Some(handler) = on_move.as_ref() {
                    handler(&state.fraction_at(event.position.x), window, cx);
                }
            }
        };

        let dragged = {
            let state = state.clone();
            let on_move = on_move.clone();
            let mine = id.clone();
            move |event: &DragMoveEvent<Grab>, window: &mut Window, cx: &mut App| {
                if event.drag(cx).0 != mine {
                    return;
                }
                if let Some(handler) = on_move.as_ref() {
                    handler(&state.fraction_at(event.event.position.x), window, cx);
                }
            }
        };

        let released = move |event: &MouseUpEvent, window: &mut Window, cx: &mut App| {
            if let Some(handler) = on_release.as_ref() {
                handler(event, window, cx);
            }
        };

        div()
            .id(gpui::ElementId::Name(id.clone()))
            .flex()
            .items_center()
            .w_full()
            .h(px(HIT))
            .when(enabled, |this| {
                this.cursor_pointer()
                    .on_mouse_down(MouseButton::Left, down)
                    .on_drag(Grab(id.clone()), |grab, _, _, cx| cx.new(|_| grab.clone()))
                    .on_drag_move(dragged)
                    .on_mouse_up(MouseButton::Left, released.clone())
                    .on_mouse_up_out(MouseButton::Left, released)
            })
            .child(
                div()
                    .relative()
                    .w_full()
                    .h(px(TRACK))
                    .rounded_full()
                    .bg(empty)
                    .child(
                        div()
                            .w(relative(fraction))
                            .h_full()
                            .rounded_full()
                            .bg(filled),
                    )
                    .when(enabled, |this| {
                        this.child(
                            div()
                                .absolute()
                                .left(relative(fraction))
                                .top(px((TRACK - THUMB) / 2.))
                                .ml(px(-THUMB / 2.))
                                .size(px(THUMB))
                                .rounded_full()
                                .bg(thumb),
                        )
                    })
                    .child(
                        canvas(move |b, _, _| bounds.set(b), |_, _, _, _| {})
                            .absolute()
                            .size_full(),
                    ),
            )
    }
}
