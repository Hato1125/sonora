use gpui::prelude::*;
use gpui::{AnyElement, Hsla, Pixels, SharedString, Window, px, svg};
use ui::{Artwork, Cell};

pub(crate) const NUMBER: Pixels = px(44.);
pub(crate) const TRAILING: Pixels = px(72.);
pub(crate) const YEAR: Pixels = px(64.);
pub(crate) const ARTWORK: Pixels = px(28.);
pub(crate) const ARTWORK_COLUMN: Pixels = px(28. + 8. * 2.);
pub(crate) const ROUNDED: Pixels = px(4.);

pub(crate) const ALWAYS: Pixels = Pixels::ZERO;
pub(crate) const WIDE: Pixels = px(740.);
pub(crate) const ROOMY: Pixels = px(620.);
pub(crate) const SNUG: Pixels = px(420.);

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

pub(crate) fn icon<F>(cell: &Cell<F>, path: &'static str, color: Hsla) -> AnyElement {
    cell.frame()
        .child(svg().path(path).size(px(11.)).flex_none().text_color(color))
        .h_full()
        .flex()
        .items_center()
        .into_any_element()
}

pub(crate) fn blank<F>(cell: &Cell<F>) -> AnyElement {
    cell.frame().into_any_element()
}

pub(crate) fn content_width(window: &Window, sidebar: Pixels, inset: Pixels) -> Pixels {
    (window.viewport_size().width - sidebar - inset).max(px(200.))
}
