use std::cell::Cell as Slot;

use gpui::prelude::*;
use gpui::{
    App, Context, Div, DragMoveEvent, Empty, MouseButton, MouseDownEvent, Pixels, ScrollHandle,
    Stateful, Window, div, point, px,
};

use crate::theme::ActiveTheme as _;

const BAR: Pixels = px(6.);
const MIN_THUMB: Pixels = px(24.);

#[derive(Clone)]
struct Grab {
    start: Slot<Pixels>,
    offset: Slot<Pixels>,
}

impl gpui::Render for Grab {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

pub fn scrolled(scroll: &ScrollHandle) -> Pixels {
    (-scroll.offset().y).clamp(Pixels::ZERO, scroll.max_offset().y)
}

pub fn scrollbar(scroll: &ScrollHandle, cx: &App) -> Option<Stateful<Div>> {
    let theme = *cx.theme();
    let viewport = scroll.bounds().size.height;
    let hidden = scroll.max_offset().y;
    let offset = scrolled(scroll);

    if viewport <= Pixels::ZERO || hidden <= Pixels::ZERO {
        return None;
    }

    let content = viewport + hidden;
    let progress = (offset / hidden).clamp(0., 1.);
    let thumb = (viewport * (viewport / content)).max(MIN_THUMB);
    let travel = viewport - thumb;

    let drag = scroll.clone();
    let jump = scroll.clone();

    Some(
        div()
            .id("scrollbar")
            .occlude()
            .absolute()
            .top_0()
            .right_0()
            .w(BAR)
            .h_full()
            .on_mouse_down(MouseButton::Left, move |event: &MouseDownEvent, _, _| {
                let local = event.position.y - jump.bounds().origin.y - thumb / 2.;
                let fraction = (local / travel).clamp(0., 1.);
                jump.set_offset(point(Pixels::ZERO, Pixels::ZERO - hidden * fraction));
            })
            .child(
                div()
                    .id("scrollbar-thumb")
                    .absolute()
                    .top(travel * progress)
                    .w(BAR)
                    .h(thumb)
                    .rounded_full()
                    .bg(theme.muted_foreground.opacity(0.35))
                    .hover(move |style| style.bg(theme.muted_foreground.opacity(0.55)))
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_drag(
                        Grab {
                            start: Slot::new(Pixels::ZERO),
                            offset: Slot::new(offset),
                        },
                        |grab, _, window, cx| {
                            grab.start.set(window.mouse_position().y);
                            cx.new(|_| grab.clone())
                        },
                    )
                    .on_drag_move(move |event: &DragMoveEvent<Grab>, _, cx| {
                        let grab = event.drag(cx);
                        let moved = event.event.position.y - grab.start.get();
                        let scrolled = grab.offset.get() + moved * (hidden / travel);
                        let clamped = scrolled.clamp(Pixels::ZERO, hidden);
                        drag.set_offset(point(Pixels::ZERO, Pixels::ZERO - clamped));
                    }),
            ),
    )
}
