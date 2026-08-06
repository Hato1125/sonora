use gpui::prelude::*;
use gpui::{AnyElement, App, Div, Hsla, MouseButton, Pixels, SharedString, Window, div, px, svg};
use router::{Destination, Link as _};
use ui::{ActiveTheme as _, Artwork, Cell, ExplicitBadge, ROW_GROUP, Theme};

pub(crate) const NUMBER: Pixels = px(44.);
pub(crate) const TRAILING: Pixels = px(72.);
pub(crate) const YEAR: Pixels = px(64.);
pub(crate) const GLYPH: Pixels = px(11.);
pub(crate) const HIT: Pixels = px(18.);

pub(crate) const ALWAYS: Pixels = Pixels::ZERO;
pub(crate) const WIDE: Pixels = px(740.);
pub(crate) const ROOMY: Pixels = px(620.);
pub(crate) const SNUG: Pixels = px(420.);

pub(crate) fn glyph(theme: &Theme) -> Pixels {
    px((theme.metrics.row / px(1.) * 0.23).round())
}

#[derive(IntoElement)]
struct Glyph {
    icon: &'static str,
    color: Hsla,
}

impl RenderOnce for Glyph {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        svg()
            .path(self.icon)
            .size(glyph(cx.theme()))
            .text_color(self.color)
    }
}

#[derive(IntoElement)]
struct Thumb {
    url: Option<String>,
}

impl RenderOnce for Thumb {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();

        Artwork::new(self.url).size(theme.metrics.thumb)
    }
}

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
                        .child(Glyph { icon, color })
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

pub(crate) fn link<F>(
    cell: &Cell<F>,
    id: &'static str,
    value: impl Into<SharedString>,
    color: Hsla,
    to: Destination,
) -> AnyElement {
    cell.frame()
        .id((id, cell.row))
        .text_color(color)
        .hover(|style| style.underline())
        .link(to)
        .child(value.into())
        .into_any_element()
}

fn line<F>(cell: &Cell<F>, color: Option<Hsla>) -> Div {
    cell.frame()
        .when_some(color, |this, color| this.text_color(color))
}

pub(crate) fn text<F>(cell: &Cell<F>, value: impl Into<SharedString>) -> AnyElement {
    line(cell, None).child(value.into()).into_any_element()
}

pub(crate) fn dim<F>(cell: &Cell<F>, value: impl Into<SharedString>, muted: Hsla) -> AnyElement {
    line(cell, Some(muted))
        .child(value.into())
        .into_any_element()
}

pub(crate) fn title<F>(
    cell: &Cell<F>,
    value: impl Into<SharedString>,
    color: Option<Hsla>,
    explicit: bool,
) -> AnyElement {
    if !explicit {
        return line(cell, color).child(value.into()).into_any_element();
    }

    line(cell, color)
        .flex()
        .items_center()
        .gap_1p5()
        .child(div().min_w_0().truncate().child(value.into()))
        .child(div().flex_none().child(ExplicitBadge::new()))
        .into_any_element()
}

pub(crate) fn artwork<F>(cell: &Cell<F>, url: Option<String>) -> AnyElement {
    cell.middle().child(Thumb { url }).into_any_element()
}

pub(crate) fn blank<F>(cell: &Cell<F>) -> AnyElement {
    cell.frame().into_any_element()
}

pub(crate) fn content_width(window: &Window, sidebar: Pixels, inset: Pixels) -> Pixels {
    (window.viewport_size().width - sidebar - inset).max(px(200.))
}
