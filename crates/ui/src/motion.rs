use std::time::Duration;

use gpui::{
    Animation, AnimationElement, AnimationExt as _, App, ElementId, IntoElement, SharedString,
    ease_in_out, ease_out_quint,
};
use i18n::t;

const QUICK: Duration = Duration::from_millis(120);
const BASE: Duration = Duration::from_millis(200);
const SLOW: Duration = Duration::from_millis(320);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Motion {
    Quick,
    Base,
    Slow,
}

impl Motion {
    pub fn span(self) -> Duration {
        match self {
            Self::Quick => QUICK,
            Self::Base => BASE,
            Self::Slow => SLOW,
        }
    }

    pub fn animation(self) -> Animation {
        let animation = Animation::new(self.span());

        match self {
            Self::Base => animation.with_easing(ease_in_out),
            Self::Quick | Self::Slow => animation.with_easing(ease_out_quint()),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Stillness {
    #[default]
    System,
    Always,
    Never,
}

impl Stillness {
    pub const ALL: [Self; 3] = [Self::System, Self::Always, Self::Never];

    pub fn id(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Always => "always",
            Self::Never => "never",
        }
    }

    pub fn label(self) -> SharedString {
        match self {
            Self::System => t!("motion-system"),
            Self::Always => t!("motion-always"),
            Self::Never => t!("motion-never"),
        }
    }

    pub fn from_id(id: &str) -> Self {
        match id {
            "always" => Self::Always,
            "never" => Self::Never,
            _ => Self::System,
        }
    }

    pub fn still(self) -> bool {
        match self {
            Self::System => system_still(),
            Self::Always => true,
            Self::Never => false,
        }
    }
}

pub trait Motioned: Sized {
    fn motion(
        self,
        id: impl Into<ElementId>,
        motion: Motion,
        animator: impl Fn(Self, f32) -> Self + 'static,
    ) -> AnimationElement<Self>;
}

impl<E: IntoElement + 'static> Motioned for E {
    fn motion(
        self,
        id: impl Into<ElementId>,
        motion: Motion,
        animator: impl Fn(Self, f32) -> Self + 'static,
    ) -> AnimationElement<Self> {
        self.with_animation(id, motion.animation(), animator)
    }
}

pub fn apply(stillness: Stillness, cx: &mut App) {
    cx.set_reduce_motion(stillness.still());
}

pub fn animates(cx: &App) -> bool {
    !cx.reduce_motion()
}

fn system_still() -> bool {
    false
}
