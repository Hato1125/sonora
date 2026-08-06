use std::cell::Cell as Slot;
use std::time::Duration;

use gpui::prelude::*;
use gpui::{
    Context, DragMoveEvent, Empty, MouseButton, MouseDownEvent, Pixels, Render, ScrollHandle, Task,
    Window, div, point, px,
};

use crate::theme::ActiveTheme as _;

const BAR: Pixels = px(6.);
const MIN_THUMB: Pixels = px(24.);
const LINGER: Duration = Duration::from_secs(2);
const IDLE: f32 = 0.;
const RESTING: f32 = 0.35;
const ACTIVE: f32 = 0.55;

#[derive(Clone)]
struct Grab {
    start: Slot<Pixels>,
    offset: Slot<Pixels>,
}

impl Render for Grab {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

pub fn scrolled(scroll: &ScrollHandle) -> Pixels {
    (-scroll.offset().y).clamp(Pixels::ZERO, scroll.max_offset().y)
}

pub struct Scrollbar {
    scroll: ScrollHandle,
    seen: Pixels,
    awake: bool,
    hovered: bool,
    linger: Option<Task<()>>,
}

impl Scrollbar {
    pub fn new(scroll: ScrollHandle) -> Self {
        Self {
            scroll,
            seen: Pixels::ZERO,
            awake: false,
            hovered: false,
            linger: None,
        }
    }

    pub fn scroll(&self) -> &ScrollHandle {
        &self.scroll
    }

    fn wake(&mut self, cx: &mut Context<Self>) {
        self.awake = true;
        cx.notify();
        self.linger = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(LINGER).await;
            this.update(cx, |this, cx| {
                this.awake = false;
                cx.notify();
            })
            .ok();
        }));
    }
}

impl Render for Scrollbar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let viewport = self.scroll.bounds().size.height;
        let hidden = self.scroll.max_offset().y;
        let offset = scrolled(&self.scroll);

        if offset != self.seen {
            self.seen = offset;
            self.wake(cx);
        }

        if viewport <= Pixels::ZERO || hidden <= Pixels::ZERO {
            return div().into_any_element();
        }

        let theme = *cx.theme();
        let content = viewport + hidden;
        let progress = (offset / hidden).clamp(0., 1.);
        let thumb = (viewport * (viewport / content)).max(MIN_THUMB);
        let travel = viewport - thumb;
        let resting = match self.awake || self.hovered {
            true => RESTING,
            false => IDLE,
        };

        let jump = self.scroll.clone();
        let drag = self.scroll.clone();

        div()
            .id("scrollbar")
            .occlude()
            .absolute()
            .top_0()
            .right_0()
            .w(BAR)
            .h_full()
            .on_hover(cx.listener(|this, hovered: &bool, _, cx| {
                this.hovered = *hovered;
                this.wake(cx);
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                    let local = event.position.y - jump.bounds().origin.y - thumb / 2.;
                    let fraction = (local / travel).clamp(0., 1.);
                    jump.set_offset(point(Pixels::ZERO, Pixels::ZERO - hidden * fraction));
                    this.wake(cx);
                }),
            )
            .child(
                div()
                    .id("scrollbar-thumb")
                    .absolute()
                    .top(travel * progress)
                    .right_1()
                    .w(BAR)
                    .h(thumb)
                    .rounded_full()
                    .bg(theme.muted_foreground.opacity(resting))
                    .hover(move |style| style.bg(theme.muted_foreground.opacity(ACTIVE)))
                    .cursor_pointer()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _: &MouseDownEvent, _, cx| {
                            this.wake(cx);
                            cx.stop_propagation();
                        }),
                    )
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
                    .on_drag_move(
                        cx.listener(move |this, event: &DragMoveEvent<Grab>, _, cx| {
                            let (start, base) = {
                                let grab = event.drag(cx);
                                (grab.start.get(), grab.offset.get())
                            };
                            let moved = event.event.position.y - start;
                            let scrolled = base + moved * (hidden / travel);
                            let clamped = scrolled.clamp(Pixels::ZERO, hidden);
                            drag.set_offset(point(Pixels::ZERO, Pixels::ZERO - clamped));
                            this.wake(cx);
                        }),
                    ),
            )
            .into_any_element()
    }
}
