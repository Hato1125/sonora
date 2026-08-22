use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui::{Pixels, Point, ScrollHandle, Window, point, px};

const EASE: f32 = 0.12;
const HERTZ: f32 = 180.;
const STALL: Duration = Duration::from_millis(64);
const REST: Pixels = px(0.5);

#[derive(Default)]
struct Drift {
    shown: Point<Pixels>,
    target: Point<Pixels>,
    gliding: bool,
    armed: bool,
    beat: Option<Instant>,
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
        self.schedule_frame(scroll, window);
    }

    pub fn aim(&self, scroll: &ScrollHandle, to: Point<Pixels>, window: &mut Window) {
        {
            let mut drift = self.0.borrow_mut();
            drift.target = held(to, scroll);
            drift.gliding = true;
        }
        self.schedule_frame(scroll, window);
    }

    pub fn goal(&self, scroll: &ScrollHandle) -> Point<Pixels> {
        let drift = self.0.borrow();

        match drift.gliding {
            true => drift.target,
            false => scroll.offset(),
        }
    }

    fn schedule_frame(&self, scroll: &ScrollHandle, window: &mut Window) {
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

            let now = Instant::now();
            let elapsed = drift
                .beat
                .replace(now)
                .map(|beat| now.duration_since(beat).min(STALL))
                .unwrap_or(Duration::from_secs_f32(1. / HERTZ));
            let ease = 1. - (1. - EASE).powf(elapsed.as_secs_f32() * HERTZ);

            let target = held(drift.target, scroll);
            let step = target - drift.shown;
            match step.x.abs() < REST && step.y.abs() < REST {
                true => {
                    drift.shown = target;
                    drift.gliding = false;
                    drift.beat = None;
                    target
                }
                false => {
                    drift.shown += point(step.x * ease, step.y * ease);
                    drift.shown
                }
            }
        };

        scroll.set_offset(landed);
        window.refresh();
        self.schedule_frame(scroll, window);
    }
}

fn held(at: Point<Pixels>, scroll: &ScrollHandle) -> Point<Pixels> {
    let reach = scroll.max_offset();

    point(
        at.x.clamp(-reach.x.max(Pixels::ZERO), Pixels::ZERO),
        at.y.clamp(-reach.y.max(Pixels::ZERO), Pixels::ZERO),
    )
}
