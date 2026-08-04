use std::time::Duration;

use gpui::prelude::*;
use gpui::{AnyElement, Hsla, Pixels, SharedString, Window, px};
use ui::{Artwork, Cell};

pub(crate) const NUMBER: Pixels = px(44.);
pub(crate) const TRAILING: Pixels = px(72.);
pub(crate) const YEAR: Pixels = px(64.);
pub(crate) const ARTWORK: Pixels = px(28.);
pub(crate) const ARTWORK_COLUMN: Pixels = px(28. + 8. * 2.);
pub(crate) const ROUNDED: Pixels = px(4.);

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

pub(crate) fn length(value: Duration) -> String {
    let total = value.as_secs();
    format!("{}:{:02}", total / 60, total % 60)
}

pub(crate) fn table_width(window: &Window, inset: Pixels) -> Pixels {
    const CHROME: f32 = 240.;
    (window.viewport_size().width - px(CHROME) - inset).max(px(320.))
}
