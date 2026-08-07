use std::cell::Cell;
use std::rc::Rc;

use gpui::prelude::*;
use gpui::{
    App, Bounds, DragMoveEvent, Empty, Hsla, MouseButton, MouseDownEvent, Pixels, Render,
    SharedString, Window, canvas, div, px,
};

use crate::theme::ActiveTheme as _;
use crate::time::clock;

const TRACK: f32 = 0.5;
const THUMB: f32 = 1.5;
const HIT: f32 = 2.;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    Clock,
    Plain,
}

impl Unit {
    pub fn say(self, value: f32) -> SharedString {
        match self {
            Unit::Clock => clock(std::time::Duration::from_secs_f32(value.max(0.))),
            Unit::Plain => SharedString::from(format!("{}", value.round() as i64)),
        }
    }
}

#[derive(Clone)]
pub struct RangeAxis {
    pub key: &'static str,
    pub label: SharedString,
    pub bounds: (f32, f32),
    pub value: (f32, f32),
    pub unit: Unit,
}

impl RangeAxis {
    pub fn span(&self) -> f32 {
        (self.bounds.1 - self.bounds.0).max(f32::EPSILON)
    }

    pub fn share(&self) -> (f32, f32) {
        (
            ((self.value.0 - self.bounds.0) / self.span()).clamp(0., 1.),
            ((self.value.1 - self.bounds.0) / self.span()).clamp(0., 1.),
        )
    }

    pub fn at(&self, share: (f32, f32)) -> (f32, f32) {
        (
            self.bounds.0 + share.0 * self.span(),
            self.bounds.0 + share.1 * self.span(),
        )
    }

    pub fn whole(&self) -> bool {
        let (low, high) = self.share();
        low <= f32::EPSILON && high >= 1. - f32::EPSILON
    }
}

#[derive(Clone)]
pub struct FlagAxis {
    pub key: &'static str,
    pub label: SharedString,
    pub on: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Handle {
    Low,
    High,
}

#[derive(Clone)]
struct Seize(SharedString);

impl Render for Seize {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

#[derive(Clone)]
pub struct RangeState {
    id: SharedString,
    bounds: Rc<Cell<Bounds<Pixels>>>,
    active: Rc<Cell<Handle>>,
}

impl RangeState {
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            bounds: Rc::new(Cell::new(Bounds::default())),
            active: Rc::new(Cell::new(Handle::Low)),
        }
    }

    fn share_at(&self, x: Pixels, pad: Pixels) -> f32 {
        let bounds = self.bounds.get();
        let pin = px((pad / px(1.) * THUMB).round());
        let travel = bounds.size.width - pin;
        if travel <= px(0.) {
            return 0.;
        }
        ((x - bounds.origin.x - pin / 2.) / travel).clamp(0., 1.)
    }
}

#[derive(IntoElement)]
pub struct RangeScrubber {
    id: SharedString,
    bounds: Rc<Cell<Bounds<Pixels>>>,
    active: Rc<Cell<Handle>>,
    value: (f32, f32),
    filled: Hsla,
    empty: Hsla,
    thumb: Hsla,
    on_change: Option<Box<dyn Fn(&(f32, f32), &mut Window, &mut App) + 'static>>,
}

impl RangeScrubber {
    pub fn new(state: &RangeState, value: (f32, f32)) -> Self {
        Self {
            id: state.id.clone(),
            bounds: state.bounds.clone(),
            active: state.active.clone(),
            value: (value.0.clamp(0., 1.), value.1.clamp(0., 1.)),
            filled: gpui::white(),
            empty: gpui::black(),
            thumb: gpui::white(),
            on_change: None,
        }
    }

    pub fn colors(mut self, filled: Hsla, empty: Hsla, thumb: Hsla) -> Self {
        self.filled = filled;
        self.empty = empty;
        self.thumb = thumb;
        self
    }

    pub fn on_change(
        mut self,
        handler: impl Fn(&(f32, f32), &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_change = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for RangeScrubber {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let pad = cx.theme().metrics.pad;
        let line = px((pad / px(1.) * TRACK).round());
        let pin = px((pad / px(1.) * THUMB).round());
        let reach = px((pad / px(1.) * HIT).round());

        let Self {
            id,
            bounds,
            active,
            value,
            filled,
            empty,
            thumb,
            on_change,
        } = self;

        let state = Rc::new(RangeState {
            id: id.clone(),
            bounds: bounds.clone(),
            active: active.clone(),
        });
        let on_change = on_change.map(Rc::new);

        let seize = {
            let state = state.clone();
            let on_change = on_change.clone();
            move |event: &MouseDownEvent, window: &mut Window, cx: &mut App| {
                let share = state.share_at(event.position.x, pad);
                let near = match (share - value.0).abs() <= (share - value.1).abs() {
                    true => Handle::Low,
                    false => Handle::High,
                };
                state.active.set(near);
                if let Some(handler) = on_change.as_ref() {
                    handler(&settle(value, near, share), window, cx);
                }
            }
        };

        let dragged = {
            let state = state.clone();
            let on_change = on_change.clone();
            let mine = id.clone();
            move |event: &DragMoveEvent<Seize>, window: &mut Window, cx: &mut App| {
                if event.drag(cx).0 != mine {
                    return;
                }
                let share = state.share_at(event.event.position.x, pad);
                if let Some(handler) = on_change.as_ref() {
                    handler(&settle(value, state.active.get(), share), window, cx);
                }
            }
        };

        let width = bounds.get().size.width;
        let travel = (width - pin).max(Pixels::ZERO);
        let measured = width > Pixels::ZERO;

        div()
            .id(gpui::ElementId::Name(id.clone()))
            .flex()
            .items_center()
            .w_full()
            .h(reach)
            .cursor_pointer()
            .on_mouse_down(MouseButton::Left, seize)
            .on_drag(Seize(id.clone()), |seize, _, _, cx| {
                cx.new(|_| seize.clone())
            })
            .on_drag_move(dragged)
            .child(
                div()
                    .relative()
                    .w_full()
                    .h(line)
                    .rounded_full()
                    .bg(empty)
                    .when(measured, |this| {
                        this.child(
                            div()
                                .absolute()
                                .top_0()
                                .h_full()
                                .left(pin / 2. + travel * value.0)
                                .w(travel * (value.1 - value.0).max(0.))
                                .bg(filled),
                        )
                        .child(handle(travel * value.0, line, pin, thumb))
                        .child(handle(travel * value.1, line, pin, thumb))
                    })
                    .child(
                        canvas(move |b, _, _| bounds.set(b), |_, _, _, _| {})
                            .absolute()
                            .size_full(),
                    ),
            )
    }
}

fn handle(left: Pixels, line: Pixels, pin: Pixels, thumb: Hsla) -> impl IntoElement {
    div()
        .absolute()
        .top((line - pin) / 2.)
        .left(left)
        .size(pin)
        .rounded_full()
        .bg(thumb)
}

fn settle(value: (f32, f32), handle: Handle, share: f32) -> (f32, f32) {
    match handle {
        Handle::Low => (share.min(value.1), value.1),
        Handle::High => (value.0, share.max(value.0)),
    }
}

#[cfg(test)]
mod tests {
    use super::{RangeAxis, Unit};

    fn axis(bounds: (f32, f32), value: (f32, f32)) -> RangeAxis {
        RangeAxis {
            key: "test",
            label: "Test".into(),
            bounds,
            value,
            unit: Unit::Plain,
        }
    }

    #[test]
    fn shares_round_trip_through_values() {
        let axis = axis((60., 300.), (120., 240.));
        let (low, high) = axis.at(axis.share());

        assert!((low - 120.).abs() < 0.01);
        assert!((high - 240.).abs() < 0.01);
    }

    #[test]
    fn a_full_span_reads_as_untouched() {
        assert!(axis((0., 100.), (0., 100.)).whole());
        assert!(!axis((0., 100.), (10., 100.)).whole());
        assert!(!axis((0., 100.), (0., 90.)).whole());
    }

    #[test]
    fn a_collapsed_axis_never_divides_by_zero() {
        let axis = axis((5., 5.), (5., 5.));

        assert!(axis.share().0.is_finite());
        assert!(axis.share().1.is_finite());
    }

    #[test]
    fn clock_and_plain_format_differently() {
        assert_eq!(Unit::Clock.say(125.), "2:05");
        assert_eq!(Unit::Plain.say(2005.), "2005");
    }
}
