use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui::{App, EntityId, Pixels, Point, ScrollHandle, Window, point, px};

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
    eased: Option<f32>,
    beat: Option<Instant>,
}

#[derive(Clone)]
pub struct Glide {
    drift: Rc<RefCell<Drift>>,
    pace: f32,
    watched: Option<EntityId>,
}

impl Default for Glide {
    fn default() -> Self {
        Self::paced(EASE)
    }
}

impl Glide {
    pub fn paced(pace: f32) -> Self {
        Self {
            drift: Rc::default(),
            pace,
            watched: None,
        }
    }

    pub fn set_pace(&mut self, pace: f32) {
        self.pace = pace;
    }

    pub fn watch(&mut self, view: EntityId) {
        self.watched = Some(view);
    }

    pub fn sync(&self, scroll: &ScrollHandle) {
        let mut drift = self.drift.borrow_mut();
        if !drift.gliding {
            drift.shown = scroll.offset();
        }
    }

    pub fn nudge(&self, scroll: &ScrollHandle, window: &mut Window) {
        {
            let mut drift = self.drift.borrow_mut();
            let landed = scroll.offset();
            let step = landed - drift.shown;
            let from = match drift.gliding {
                true => drift.target,
                false => drift.shown,
            };

            drift.target = held(from + step, scroll);
            drift.gliding = true;
            drift.eased = None;
            scroll.set_offset(drift.shown);
        }
        self.schedule_frame(scroll, window);
    }

    pub fn aim(&self, scroll: &ScrollHandle, to: Point<Pixels>, window: &mut Window) {
        {
            let mut drift = self.drift.borrow_mut();
            drift.target = held(to, scroll);
            drift.gliding = true;
            drift.eased = Some(self.pace);
        }
        self.schedule_frame(scroll, window);
    }

    pub fn jump(&self, scroll: &ScrollHandle, to: Point<Pixels>) {
        let landed = {
            let mut drift = self.drift.borrow_mut();
            drift.target = held(to, scroll);
            drift.shown = drift.target;
            drift.gliding = false;
            drift.beat = None;
            drift.eased = None;
            drift.shown
        };
        scroll.set_offset(landed);
    }

    pub fn goal(&self, scroll: &ScrollHandle) -> Point<Pixels> {
        let drift = self.drift.borrow();

        match drift.gliding {
            true => drift.target,
            false => scroll.offset(),
        }
    }

    fn schedule_frame(&self, scroll: &ScrollHandle, window: &mut Window) {
        {
            let mut drift = self.drift.borrow_mut();
            if drift.armed {
                return;
            }
            drift.armed = true;
        }

        let glide = self.clone();
        let scroll = scroll.clone();
        window.on_next_frame(move |window, cx| glide.step(&scroll, window, cx));
    }

    fn step(&self, scroll: &ScrollHandle, window: &mut Window, cx: &mut App) {
        let landed = {
            let mut drift = self.drift.borrow_mut();
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
            let pace = drift.eased.unwrap_or(EASE);
            let ease = 1. - (1. - pace).powf(elapsed.as_secs_f32() * HERTZ);

            let target = held(drift.target, scroll);
            let step = target - drift.shown;
            match step.x.abs() < REST && step.y.abs() < REST {
                true => {
                    drift.shown = target;
                    drift.gliding = false;
                    drift.beat = None;
                    drift.eased = None;
                    target
                }
                false => {
                    drift.shown += point(step.x * ease, step.y * ease);
                    drift.shown
                }
            }
        };

        scroll.set_offset(landed);
        match self.watched {
            Some(view) => cx.notify(view),
            None => window.refresh(),
        }
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
