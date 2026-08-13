use gpui::{Pixels, Point, ScrollHandle, Window, point, px};

const EASE: f32 = 0.22;
const REST: Pixels = px(0.5);

#[derive(Default)]
pub struct Glide {
    shown: Point<Pixels>,
    target: Point<Pixels>,
    gliding: bool,
}

impl Glide {
    pub fn nudge(&mut self, scroll: &ScrollHandle) {
        let landed = scroll.offset();
        let step = landed - self.shown;
        let from = match self.gliding {
            true => self.target,
            false => self.shown,
        };

        self.target = held(from + step, scroll);
        self.gliding = true;
        scroll.set_offset(self.shown);
    }

    pub fn aim(&mut self, scroll: &ScrollHandle, to: Point<Pixels>) {
        self.target = held(to, scroll);
        self.gliding = true;
    }

    pub fn goal(&self, scroll: &ScrollHandle) -> Point<Pixels> {
        match self.gliding {
            true => self.target,
            false => scroll.offset(),
        }
    }

    pub fn step(&mut self, scroll: &ScrollHandle, window: &mut Window) {
        if !self.gliding {
            self.shown = scroll.offset();
            return;
        }

        let target = held(self.target, scroll);
        let step = target - self.shown;
        if step.x.abs() < REST && step.y.abs() < REST {
            self.shown = target;
            self.gliding = false;
            scroll.set_offset(target);
            return;
        }

        self.shown += point(step.x * EASE, step.y * EASE);
        scroll.set_offset(self.shown);
        window.request_animation_frame();
    }
}

fn held(at: Point<Pixels>, scroll: &ScrollHandle) -> Point<Pixels> {
    let reach = scroll.max_offset();

    point(
        at.x.clamp(-reach.x.max(Pixels::ZERO), Pixels::ZERO),
        at.y.clamp(-reach.y.max(Pixels::ZERO), Pixels::ZERO),
    )
}
