use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::LazyLock;
use std::time::{Duration, Instant};

use gpui::prelude::*;
use gpui::{App, IntoElement, Pixels, RenderOnce, SharedString, Window, div, px};
use ui::{ActiveTheme as _, Text};

const SAMPLES: usize = 120;
const RECENT: Duration = Duration::from_secs(1);
const HEADROOM: f32 = 4.;
const BAR: Pixels = px(2.);
const GRAPH: Pixels = px(40.);
const FLOOR: f32 = 0.05;

#[derive(Clone, Copy, Default, PartialEq)]
enum Corner {
    #[default]
    Right,
    Left,
}

impl Corner {
    fn other(self) -> Self {
        match self {
            Self::Right => Self::Left,
            Self::Left => Self::Right,
        }
    }
}

#[derive(Default)]
struct Frames {
    opened: Option<Instant>,
    closed: Option<Instant>,
    costs: VecDeque<(Instant, f32)>,
    gaps: VecDeque<f32>,
    corner: Corner,
}

#[derive(Clone, Default)]
pub struct FrameStats(Rc<RefCell<Frames>>);

pub struct Reading {
    pub last: f32,
    pub worst: f32,
    pub fps: f32,
    pub budget: f32,
    pub costs: Vec<f32>,
}

impl FrameStats {
    pub fn wanted() -> bool {
        static ON: LazyLock<bool> = LazyLock::new(|| std::env::var_os("SONORA_STATS").is_some());
        *ON
    }

    pub fn open(&self) {
        self.0.borrow_mut().opened = Some(Instant::now());
    }

    pub fn close(&self) {
        let now = Instant::now();
        let mut frames = self.0.borrow_mut();
        let Some(opened) = frames.opened.take() else {
            return;
        };
        if let Some(closed) = frames.closed.replace(now) {
            let gap = millis(now - closed);
            if gap > FLOOR {
                frames.gaps.push_back(gap);
                while frames.gaps.len() > SAMPLES {
                    frames.gaps.pop_front();
                }
            }
        }
        frames.costs.push_back((now, millis(now - opened)));
        while frames.costs.len() > SAMPLES {
            frames.costs.pop_front();
        }
    }

    fn dodge(&self) {
        let mut frames = self.0.borrow_mut();
        frames.corner = frames.corner.other();
    }

    fn corner(&self) -> Corner {
        self.0.borrow().corner
    }

    fn read(&self) -> Reading {
        let frames = self.0.borrow();
        let budget = frames.gaps.iter().copied().fold(f32::MAX, f32::min);
        let recent: Vec<f32> = frames
            .costs
            .iter()
            .filter(|(at, _)| at.elapsed() <= RECENT)
            .map(|(_, cost)| *cost)
            .collect();
        let worst = recent.iter().copied().fold(0., f32::max);
        let mean = match recent.is_empty() {
            true => 0.,
            false => recent.iter().sum::<f32>() / recent.len() as f32,
        };
        let ceiling = match budget.is_finite() {
            true => 1000. / budget,
            false => f32::MAX,
        };
        Reading {
            last: frames.costs.back().map_or(0., |(_, cost)| *cost),
            worst,
            fps: match mean > 0. {
                true => (1000. / mean).min(ceiling),
                false => 0.,
            },
            budget: match budget.is_finite() {
                true => budget,
                false => 0.,
            },
            costs: frames.costs.iter().map(|(_, cost)| *cost).collect(),
        }
    }
}

#[derive(IntoElement)]
pub struct Stats {
    stats: FrameStats,
}

impl Stats {
    pub fn new(stats: FrameStats) -> Self {
        Self { stats }
    }
}

impl RenderOnce for Stats {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = *cx.theme();
        let reading = self.stats.read();
        let scale = match reading.budget > 0. {
            true => reading.budget * HEADROOM,
            false => reading.worst.max(1.),
        };
        let over = |cost: f32| reading.budget > 0. && cost > reading.budget;
        let corner = self.stats.corner();
        let dodge = self.stats.clone();

        div()
            .id("frame-stats")
            .absolute()
            .top(theme.metrics.pad)
            .when_else(
                corner == Corner::Right,
                |this| this.right(theme.metrics.pad),
                |this| this.left(theme.metrics.pad),
            )
            .on_hover(move |hovered, window, _| {
                if *hovered {
                    dodge.dodge();
                    window.refresh();
                }
            })
            .flex()
            .flex_col()
            .gap_1()
            .p(theme.metrics.pad)
            .rounded(theme.radius)
            .bg(theme.popover)
            .border_1()
            .border_color(theme.border)
            .text_size(theme.text(Text::Tiny))
            .text_color(theme.muted_foreground)
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(readout("fps", format!("{:.0}", reading.fps)))
                    .child(readout("ms", format!("{:.1}", reading.last)))
                    .child(readout("max", format!("{:.1}", reading.worst))),
            )
            .child(div().flex().items_end().gap(px(1.)).h(GRAPH).children(
                reading.costs.into_iter().map(|cost| {
                    div()
                        .w(BAR)
                        .h(GRAPH * (cost / scale).clamp(0.02, 1.))
                        .rounded(px(1.))
                        .bg(match over(cost) {
                            true => theme.danger,
                            false => theme.primary,
                        })
                }),
            ))
    }
}

fn readout(label: &'static str, value: String) -> impl IntoElement {
    div()
        .flex()
        .gap_1()
        .child(SharedString::from(label))
        .child(SharedString::from(value))
}

fn millis(span: Duration) -> f32 {
    span.as_secs_f32() * 1000.
}
