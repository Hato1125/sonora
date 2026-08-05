use gpui::prelude::*;
use gpui::{AnyElement, App, Hsla, MouseButton, Pixels, SharedString, Window, div, px, svg};
use ui::{Artwork, Cell, ROW_GROUP};

pub(crate) const NUMBER: Pixels = px(44.);
pub(crate) const TRAILING: Pixels = px(72.);
pub(crate) const YEAR: Pixels = px(64.);
pub(crate) const ARTWORK: Pixels = px(28.);
pub(crate) const ARTWORK_COLUMN: Pixels = px(28. + 8. * 2.);
pub(crate) const ROUNDED: Pixels = px(4.);
pub(crate) const GLYPH: Pixels = px(11.);
pub(crate) const HIT: Pixels = px(18.);

pub(crate) const ALWAYS: Pixels = Pixels::ZERO;
pub(crate) const WIDE: Pixels = px(740.);
pub(crate) const ROOMY: Pixels = px(620.);
pub(crate) const SNUG: Pixels = px(420.);

pub(crate) struct Transport {
    pub(crate) icon: &'static str,
    pub(crate) color: Hsla,
    pub(crate) press: Option<Box<dyn Fn(&mut App)>>,
}

pub(crate) fn transport<F>(cell: &Cell<F>, resting: AnyElement, hover: Transport) -> AnyElement {
    let Transport { icon, color, press } = hover;
    let enabled = press.is_some();

    cell.frame()
        .h_full()
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .relative()
                .flex()
                .items_center()
                .justify_center()
                .size(HIT)
                .child(
                    div()
                        .flex()
                        .group_hover(ROW_GROUP, |style| style.invisible())
                        .child(resting),
                )
                .child(
                    div()
                        .id(("transport", cell.row))
                        .absolute()
                        .top_0()
                        .left_0()
                        .right_0()
                        .bottom_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .invisible()
                        .group_hover(ROW_GROUP, |style| style.visible())
                        .child(svg().path(icon).size(GLYPH).text_color(color))
                        .when_else(
                            enabled,
                            |this| this.cursor_pointer(),
                            |this| this.cursor_not_allowed(),
                        )
                        .when_some(press, |this, press| {
                            this.on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                                .on_click(move |_, _, cx| press(cx))
                        }),
                ),
        )
        .into_any_element()
}

pub(crate) fn text<F>(cell: &Cell<F>, value: impl Into<SharedString>) -> AnyElement {
    cell.frame().child(value.into()).into_any_element()
}

pub(crate) fn dim<F>(cell: &Cell<F>, value: impl Into<SharedString>, muted: Hsla) -> AnyElement {
    cell.frame()
        .text_color(muted)
        .child(value.into())
        .into_any_element()
}

pub(crate) fn artwork<F>(cell: &Cell<F>, url: Option<String>) -> AnyElement {
    cell.middle()
        .child(Artwork::new(url).size(ARTWORK).rounded(ROUNDED))
        .into_any_element()
}

pub(crate) fn blank<F>(cell: &Cell<F>) -> AnyElement {
    cell.frame().into_any_element()
}

pub(crate) fn content_width(window: &Window, sidebar: Pixels, inset: Pixels) -> Pixels {
    (window.viewport_size().width - sidebar - inset).max(px(200.))
}
