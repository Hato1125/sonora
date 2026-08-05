use std::cell::Cell as Slot;
use std::cmp::Ordering;

use gpui::prelude::*;
use gpui::{
    AnyElement, App, Context, Div, DragMoveEvent, Empty, Entity, EventEmitter, MouseButton,
    MouseDownEvent, Pixels, Stateful, TextAlign, UniformListScrollHandle, Window, div, point,
    px, svg, uniform_list,
};

use crate::theme::ActiveTheme as _;

const PADDING: Pixels = px(8.);
const TRAIL: Pixels = px(4.);
const MIN_CELL: Pixels = px(24.);
const MIN_FLEXIBLE: Pixels = px(120.);
const ROW: Pixels = px(32.);
const BAR: Pixels = px(6.);
const MIN_THUMB: Pixels = px(24.);
const SLACK: Pixels = px(2.);

#[derive(Clone, Copy)]
pub enum Width {
    Fixed(Pixels),
    Fill(f32),
}

#[derive(Clone, Copy, PartialEq)]
pub enum Sort {
    Ascending,
    Descending,
}

pub struct ColumnSpec<F: 'static> {
    pub field: F,
    pub key: &'static str,
    pub header: &'static str,
    pub align: TextAlign,
    pub width: Width,
    pub flush: bool,
    pub sortable: bool,
    pub hide_below: Pixels,
}

impl<F: 'static> ColumnSpec<F> {
    fn share(&self) -> f32 {
        match self.width {
            Width::Fixed(_) => 0.,
            Width::Fill(share) => share,
        }
    }

    fn resolve(&self, flexible: Pixels, shares: f32) -> Pixels {
        match self.width {
            Width::Fixed(width) => width,
            Width::Fill(share) if shares > 0. => flexible * (share / shares),
            Width::Fill(_) => Pixels::ZERO,
        }
    }
}

pub struct Cell<F> {
    pub field: F,
    pub width: Pixels,
    pub align: TextAlign,
    pub display: usize,
    pub row: usize,
}

impl<F> Cell<F> {
    pub fn frame(&self) -> Div {
        frame(self.width, self.align)
    }

    pub fn middle(&self) -> Div {
        div().w(self.width).h_full().flex().items_center()
    }
}

pub trait GridSource: 'static {
    type Field: Copy + PartialEq + 'static;

    fn columns(&self) -> &'static [ColumnSpec<Self::Field>];
    fn rows(&self, cx: &App) -> usize;
    fn cell(&self, cell: Cell<Self::Field>, cx: &mut App) -> AnyElement;

    fn compare(&self, _field: Self::Field, a: usize, b: usize, _cx: &App) -> Ordering {
        a.cmp(&b)
    }

    fn is_loading(&self, _cx: &App) -> bool {
        false
    }
}

fn frame(width: Pixels, align: TextAlign) -> Div {
    let frame = div().w(width).flex_none().min_w_0();
    match align {
        TextAlign::Left => frame.truncate(),
        TextAlign::Center => frame.flex().justify_center(),
        TextAlign::Right => frame.flex().justify_end(),
    }
}

struct Resolved<F: 'static> {
    spec: &'static ColumnSpec<F>,
    width: Pixels,
}

pub struct GridDelegate<S: GridSource> {
    source: S,
    columns: Vec<Resolved<S::Field>>,
    width: Pixels,
    sort: Option<(S::Field, Sort)>,
    order: Vec<usize>,
}

impl<S: GridSource> GridDelegate<S> {
    pub fn new(source: S, width: Pixels, cx: &App) -> Self {
        let columns = build(source.columns(), width);
        let mut delegate = Self {
            source,
            columns,
            width,
            sort: None,
            order: Vec::new(),
        };
        delegate.reorder(cx);
        delegate
    }

    pub fn source(&self) -> &S {
        &self.source
    }

    pub fn set_width(&mut self, width: Pixels) {
        self.width = width;
        self.relayout();
    }

    pub fn rebuild(&mut self, cx: &App) {
        self.relayout();
        self.reorder(cx);
    }

    pub fn row(&self, display: usize) -> usize {
        self.order.get(display).copied().unwrap_or(display)
    }

    pub fn row_count(&self) -> usize {
        self.order.len()
    }

    fn relayout(&mut self) {
        self.columns = build(self.source.columns(), self.width);
    }

    fn reorder(&mut self, cx: &App) {
        let mut order: Vec<usize> = (0..self.source.rows(cx)).collect();

        if let Some((field, direction)) = self.sort {
            match direction {
                Sort::Ascending => order.sort_by(|&a, &b| self.source.compare(field, a, b, cx)),
                Sort::Descending => order.sort_by(|&a, &b| self.source.compare(field, b, a, cx)),
            }
        }

        self.order = order;
    }

    fn inner_width(&self, col_ix: usize) -> Pixels {
        let trailing = col_ix + 1 == self.columns.len();
        let gutter = if trailing { TRAIL } else { Pixels::ZERO };
        (self.columns[col_ix].width - PADDING * 2. - gutter).max(MIN_CELL)
    }

    fn direction(&self, field: S::Field) -> Option<Sort> {
        self.sort
            .filter(|(sorted, _)| *sorted == field)
            .map(|(_, direction)| direction)
    }
}

fn build<F: Copy + PartialEq + 'static>(
    specs: &'static [ColumnSpec<F>],
    room: Pixels,
) -> Vec<Resolved<F>> {
    let available = (room - SLACK).max(MIN_FLEXIBLE);
    let mut visible: Vec<_> = specs
        .iter()
        .filter(|spec| available >= spec.hide_below)
        .collect();
    if visible.is_empty() {
        visible.extend(specs.iter().take(1));
    }

    let fixed = visible
        .iter()
        .map(|spec| spec.resolve(Pixels::ZERO, 0.))
        .fold(Pixels::ZERO, |total, width| total + width);
    let shares: f32 = visible.iter().map(|spec| spec.share()).sum();
    let flexible = (available - fixed).max(MIN_FLEXIBLE);

    let mut columns: Vec<Resolved<F>> = visible
        .iter()
        .map(|spec| Resolved {
            spec,
            width: spec.resolve(flexible, shares),
        })
        .collect();

    let total = columns
        .iter()
        .map(|column| column.width)
        .fold(Pixels::ZERO, |total, width| total + width);
    let leftover = available - total;
    let stretchy = visible
        .iter()
        .rposition(|spec| spec.share() > 0.)
        .unwrap_or(visible.len().saturating_sub(1));
    if leftover > Pixels::ZERO {
        if let Some(column) = columns.get_mut(stretchy) {
            column.width += leftover;
        }
    }

    columns
}

pub enum GridEvent {
    DoubleClicked(usize),
}

#[derive(Clone)]
struct Grab {
    start: Slot<Pixels>,
    offset: Slot<Pixels>,
}

impl gpui::Render for Grab {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Empty
    }
}

pub struct GridState<S: GridSource> {
    delegate: GridDelegate<S>,
    scroll: UniformListScrollHandle,
}

impl<S: GridSource> EventEmitter<GridEvent> for GridState<S> {}

impl<S: GridSource> GridState<S> {
    pub fn new(delegate: GridDelegate<S>, _window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {
            delegate,
            scroll: UniformListScrollHandle::new(),
        }
    }

    pub fn delegate(&self) -> &GridDelegate<S> {
        &self.delegate
    }

    pub fn delegate_mut(&mut self) -> &mut GridDelegate<S> {
        &mut self.delegate
    }

    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        cx.notify();
    }

    fn toggle_sort(&mut self, col_ix: usize, cx: &mut Context<Self>) {
        let Some(column) = self.delegate.columns.get(col_ix) else {
            return;
        };
        if !column.spec.sortable {
            return;
        }

        let field = column.spec.field;
        self.delegate.sort = match self.delegate.direction(field) {
            None => Some((field, Sort::Ascending)),
            Some(Sort::Ascending) => Some((field, Sort::Descending)),
            Some(Sort::Descending) => None,
        };
        self.delegate.rebuild(cx);
        cx.notify();
    }

    fn header(&self, cx: &mut Context<Self>) -> Div {
        let theme = *cx.theme();
        let heads: Vec<_> = self
            .delegate
            .columns
            .iter()
            .enumerate()
            .map(|(ix, column)| {
                (
                    ix,
                    column.width,
                    self.delegate.inner_width(ix),
                    column.spec.align,
                    column.spec.header,
                    column.spec.sortable,
                    self.delegate.direction(column.spec.field),
                )
            })
            .collect();

        div()
            .flex()
            .flex_none()
            .h(ROW)
            .bg(theme.table_head)
            .border_b_1()
            .border_color(theme.table_row_border)
            .text_color(theme.table_head_foreground)
            .children(heads.into_iter().map(
                |(ix, width, inner, align, header, sortable, direction)| {
                    div()
                        .id(("head", ix))
                        .flex()
                        .flex_none()
                        .items_center()
                        .gap_1()
                        .w(width)
                        .h_full()
                        .px(PADDING)
                        .when(sortable, |this| {
                            this.cursor_pointer()
                                .hover(move |style| style.text_color(theme.foreground))
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(move |this, _: &MouseDownEvent, _, cx| {
                                        this.toggle_sort(ix, cx)
                                    }),
                                )
                        })
                        .child(frame(inner - sort_room(sortable), align).child(header))
                        .when(sortable, |this| {
                            this.child(
                                svg()
                                    .path(sort_icon(direction))
                                    .size(px(12.))
                                    .flex_none()
                                    .text_color(match direction {
                                        Some(_) => theme.foreground,
                                        None => theme.table_head_foreground,
                                    }),
                            )
                        })
                },
            ))
    }

    fn rows(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.theme();
        let count = self.delegate.order.len();

        uniform_list(
            "grid-rows",
            count,
            cx.processor(move |this, range: std::ops::Range<usize>, _window, cx| {
                range
                    .map(|display| {
                        let row = this.delegate.row(display);
                        let cells: Vec<AnyElement> = (0..this.delegate.columns.len())
                            .map(|ix| {
                                let column = &this.delegate.columns[ix];
                                let cell = Cell {
                                    field: column.spec.field,
                                    width: this.delegate.inner_width(ix),
                                    align: column.spec.align,
                                    display,
                                    row,
                                };
                                let width = column.width;
                                div()
                                    .flex_none()
                                    .w(width)
                                    .h_full()
                                    .px(PADDING)
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .line_height(ROW)
                                    .child(this.delegate.source.cell(cell, cx))
                                    .into_any_element()
                            })
                            .collect();

                        div()
                            .id(("row", display))
                            .flex()
                            .items_center()
                            .h(ROW)
                            .border_b_1()
                            .border_color(theme.table_row_border)
                            .hover(move |style| style.bg(theme.table_hover))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |_, event: &MouseDownEvent, _, cx| {
                                    if event.click_count >= 2 {
                                        cx.emit(GridEvent::DoubleClicked(display));
                                    }
                                }),
                            )
                            .children(cells)
                    })
                    .collect()
            }),
        )
        .track_scroll(self.scroll.clone())
        .size_full()
    }

    fn scrollbar(&self, cx: &mut Context<Self>) -> Option<Stateful<Div>> {
        let theme = *cx.theme();
        let (viewport, hidden, offset) = {
            let state = self.scroll.0.borrow();
            (
                state.base_handle.bounds().size.height,
                state.base_handle.max_offset().height,
                -state.base_handle.offset().y,
            )
        };
        if viewport <= Pixels::ZERO || hidden <= Pixels::ZERO {
            return None;
        }

        let content = viewport + hidden;
        let progress = (offset / hidden).clamp(0., 1.);
        let thumb = (viewport * (viewport / content)).max(MIN_THUMB);
        let travel = viewport - thumb;

        let scroll = self.scroll.clone();
        let jump = self.scroll.clone();

        Some(
            div()
                .id("grid-scrollbar")
                .occlude()
                .absolute()
                .top_0()
                .right_0()
                .w(BAR)
                .h_full()
                .on_mouse_down(
                    MouseButton::Left,
                    move |event: &MouseDownEvent, _, _| {
                        let handle = jump.0.borrow().base_handle.clone();
                        let bounds = handle.bounds();
                        let local = event.position.y - bounds.origin.y - thumb / 2.;
                        let fraction = (local / (viewport - thumb)).clamp(0., 1.);
                        handle.set_offset(point(Pixels::ZERO, -hidden * fraction));
                    },
                )
                .child(
                    div()
                        .id("grid-thumb")
                        .absolute()
                        .top(travel * progress)
                        .w(BAR)
                        .h(thumb)
                        .rounded_full()
                        .bg(theme.muted_foreground.opacity(0.35))
                        .hover(move |style| style.bg(theme.muted_foreground.opacity(0.55)))
                        .cursor_pointer()
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .on_drag(
                            Grab {
                                start: Slot::new(Pixels::ZERO),
                                offset: Slot::new(offset),
                            },
                            |grab, _, window, cx| {
                                grab.start.set(window.mouse_position().y);
                                cx.new(|_| grab.clone())
                            },
                        )
                        .on_drag_move(move |event: &DragMoveEvent<Grab>, _, cx| {
                            let grab = event.drag(cx);
                            let moved = event.event.position.y - grab.start.get();
                            let scrolled = grab.offset.get() + moved * (hidden / travel);
                            let clamped = scrolled.clamp(Pixels::ZERO, hidden);
                            scroll
                                .0
                                .borrow()
                                .base_handle
                                .set_offset(point(Pixels::ZERO, -clamped));
                        }),
                ),
        )
    }
}

fn sort_room(sortable: bool) -> Pixels {
    if sortable { px(16.) } else { Pixels::ZERO }
}

fn sort_icon(direction: Option<Sort>) -> &'static str {
    match direction {
        Some(Sort::Ascending) => "icons/chevron-up.svg",
        Some(Sort::Descending) => "icons/chevron-down.svg",
        None => "icons/chevrons-up-down.svg",
    }
}

impl<S: GridSource> Render for GridState<S> {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .overflow_hidden()
            .child(self.header(cx))
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .child(self.rows(cx))
                    .children(self.scrollbar(cx)),
            )
    }
}

pub fn grid<S: GridSource>(state: &Entity<GridState<S>>) -> impl IntoElement {
    state.clone()
}
