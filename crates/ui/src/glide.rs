use std::cell::RefCell;
use std::rc::Rc;

use gpui::{Pixels, Point, ScrollHandle, Window, point, px};

const EASE: f32 = 0.12;
const REST: Pixels = px(0.5);

#[derive(Default)]
struct Drift {
    shown: Point<Pixels>,
    target: Point<Pixels>,
    gliding: bool,
    armed: bool,
}

#[derive(Clone, Default)]
pub struct Glide(Rc<RefCell<Drift>>);

impl Glide {
    pub fn sync(&self, scroll: &ScrollHandle) {
        let mut drift = self.0.borrow_mut();
        if !drift.gliding {
            drift.shown = scroll.offset();
        }
    }

    pub fn nudge(&self, scroll: &ScrollHandle, window: &mut Window) {
        {
            let mut drift = self.0.borrow_mut();
            let landed = scroll.offset();
            let step = landed - drift.shown;
            let from = match drift.gliding {
                true => drift.target,
                false => drift.shown,
            };

            drift.target = held(from + step, scroll);
            drift.gliding = true;
            scroll.set_offset(drift.shown);
        }
        self.arm(scroll, window);
    }

    pub fn aim(&self, scroll: &ScrollHandle, to: Point<Pixels>, window: &mut Window) {
        {
            let mut drift = self.0.borrow_mut();
            drift.target = held(to, scroll);
            drift.gliding = true;
        }
        self.arm(scroll, window);
    }

    pub fn goal(&self, scroll: &ScrollHandle) -> Point<Pixels> {
        let drift = self.0.borrow();

        match drift.gliding {
            true => drift.target,
            false => scroll.offset(),
        }
    }

    fn arm(&self, scroll: &ScrollHandle, window: &mut Window) {
        {
            let mut drift = self.0.borrow_mut();
            if drift.armed {
                return;
            }
            drift.armed = true;
        }

        let glide = self.clone();
        let scroll = scroll.clone();
        window.on_next_frame(move |window, _| glide.step(&scroll, window));
    }

    fn step(&self, scroll: &ScrollHandle, window: &mut Window) {
        let landed = {
            let mut drift = self.0.borrow_mut();
            drift.armed = false;
            if !drift.gliding {
                return;
            }

            let target = held(drift.target, scroll);
            let step = target - drift.shown;
            match step.x.abs() < REST && step.y.abs() < REST {
                true => {
                    drift.shown = target;
                    drift.gliding = false;
                    target
                }
                false => {
                    drift.shown += point(step.x * EASE, step.y * EASE);
                    drift.shown
                }
            }
        };

        scroll.set_offset(landed);
        window.refresh();
        self.arm(scroll, window);
    }
}

fn held(at: Point<Pixels>, scroll: &ScrollHandle) -> Point<Pixels> {
    let reach = scroll.max_offset();

    point(
        at.x.clamp(-reach.x.max(Pixels::ZERO), Pixels::ZERO),
        at.y.clamp(-reach.y.max(Pixels::ZERO), Pixels::ZERO),
    )
}
