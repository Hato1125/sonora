use gpui::prelude::*;
use gpui::{
    AnyElement, App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId, LayoutId,
    Pixels, Window, point, px,
};

// paints off layout
pub struct Slip {
    child: AnyElement,
    by: Pixels,
}

pub fn slip(child: impl IntoElement, by: Pixels) -> Slip {
    Slip {
        child: child.into_any_element(),
        by,
    }
}

impl IntoElement for Slip {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for Slip {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        (self.child.request_layout(window, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        // never rounded, or a slowing offset steps and stalls
        window.with_pixel_snapping(false, |window| {
            window.with_element_offset(point(px(0.), self.by), |window| {
                self.child.prepaint(window, cx);
            });
        });
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.paint(window, cx);
    }
}
