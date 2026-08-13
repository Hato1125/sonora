use gpui::prelude::*;
use gpui::{
    AnyElement, App, Div, ElementId, Entity, Interactivity, ScrollWheelEvent, StyleRefinement,
    Window, div,
};

use crate::scrollbar::Scrollbar;

#[derive(IntoElement)]
pub struct Scroller {
    base: Div,
    id: ElementId,
    bar: Entity<Scrollbar>,
    children: Vec<AnyElement>,
}

impl Scroller {
    #[track_caller]
    pub fn new(id: impl Into<ElementId>, bar: &Entity<Scrollbar>) -> Self {
        Self {
            base: div(),
            id: id.into(),
            bar: bar.clone(),
            children: Vec::new(),
        }
    }
}

impl Styled for Scroller {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl InteractiveElement for Scroller {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl ParentElement for Scroller {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Scroller {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Self {
            mut base,
            id,
            bar,
            children,
        } = self;

        let scroll = bar.read(cx).scroll().clone();
        let overrides = std::mem::take(base.style());
        bar.update(cx, |bar, _| bar.glide(window));
        let gliding = bar.clone();

        let mut surface = base
            .id(id)
            .size_full()
            .overflow_y_scroll()
            .restrict_scroll_to_axis()
            .track_scroll(&scroll)
            .on_scroll_wheel(move |event: &ScrollWheelEvent, _, cx| {
                if event.delta.precise() {
                    return;
                }
                gliding.update(cx, |bar, _| bar.nudge());
            })
            .children(children);

        surface.style().refine(&overrides);

        div().relative().size_full().child(surface).child(bar)
    }
}
