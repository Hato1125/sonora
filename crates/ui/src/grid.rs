use std::cmp::Ordering;

use gpui::prelude::*;
use gpui::{
    AbsoluteLength, AnyElement, App, Context, Corners, Div, Entity, EventEmitter, Interactivity,
    MouseButton, MouseDownEvent, Pixels, StyleRefinement, TextAlign, Window, div, px, svg,
};

use crate::theme::ActiveTheme as _;

const PADDING: Pixels = px(8.);
const TRAIL: Pixels = px(4.);
const MIN_CELL: Pixels = px(24.);
const MIN_FLEXIBLE: Pixels = px(120.);
const SLACK: Pixels = px(2.);
const OVERSCAN: usize = 2;

pub const ROW: Pixels = px(38.);
pub const ROW_GROUP: &str = "grid-row";

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

#[derive(Clone, Copy, Default)]
pub struct Viewport {
    pub top: Pixels,
    pub height: Pixels,
}

impl Viewport {
    fn rows(&self) -> usize {
        (self.height / ROW).ceil().max(0.) as usize + OVERSCAN
    }

    fn first(&self) -> usize {
        ((self.top - ROW) / ROW).floor().max(0.) as usize
    }
}

pub struct GridState<S: GridSource> {
    delegate: GridDelegate<S>,
    viewport: Viewport,
    corners: Corners<Pixels>,
}

impl<S: GridSource> EventEmitter<GridEvent> for GridState<S> {}

impl<S: GridSource> GridState<S> {
    pub fn new(delegate: GridDelegate<S>) -> Self {
        Self {
            delegate,
            viewport: Viewport::default(),
            corners: Corners::default(),
        }
    }

    pub fn set_viewport(&mut self, viewport: Viewport) {
        self.viewport = viewport;
    }

    pub fn height(&self) -> Pixels {
        ROW * (self.delegate.row_count() + 1) as f32
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

    fn header(&self, top: Corners<Pixels>, cx: &mut Context<Self>) -> Div {
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
            .rounded_tl(top.top_left)
            .rounded_tr(top.top_right)
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

    fn rows(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let theme = *cx.theme();
        let count = self.delegate.order.len();
        let first = self.viewport.first();
        let last = (first + self.viewport.rows()).min(count);
        let bottom = self.corners.bottom_left.max(self.corners.bottom_right);

        (first..last)
            .map(|display| {
                let row = self.delegate.row(display);
                let tail = display + 1 == count;
                let cells: Vec<AnyElement> = (0..self.delegate.columns.len())
                    .map(|ix| {
                        let column = &self.delegate.columns[ix];
                        let cell = Cell {
                            field: column.spec.field,
                            width: self.delegate.inner_width(ix),
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
                            .child(self.delegate.source.cell(cell, cx))
                            .into_any_element()
                    })
                    .collect();

                div()
                    .id(("row", display))
                    .group(ROW_GROUP)
                    .absolute()
                    .top(ROW * (display + 1) as f32)
                    .left_0()
                    .w_full()
                    .flex()
                    .items_center()
                    .h(ROW)
                    .when(tail, |this| {
                        this.rounded_bl(self.corners.bottom_left)
                            .rounded_br(self.corners.bottom_right)
                    })
                    .when(!tail || bottom == Pixels::ZERO, |this| {
                        this.border_b_1().border_color(theme.table_row_border)
                    })
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
                    .into_any_element()
            })
            .collect()
    }
}

fn unpinned(corners: Corners<Pixels>, pinned: Pixels) -> Corners<Pixels> {
    Corners {
        top_left: (corners.top_left - pinned).max(Pixels::ZERO),
        top_right: (corners.top_right - pinned).max(Pixels::ZERO),
        ..Corners::default()
    }
}

fn radii(style: &StyleRefinement, rem: Pixels) -> Corners<Pixels> {
    let resolve = |length: Option<AbsoluteLength>| {
        length
            .map(|length| length.to_pixels(rem))
            .unwrap_or_default()
            .max(Pixels::ZERO)
    };

    Corners {
        top_left: resolve(style.corner_radii.top_left),
        top_right: resolve(style.corner_radii.top_right),
        bottom_right: resolve(style.corner_radii.bottom_right),
        bottom_left: resolve(style.corner_radii.bottom_left),
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
        let backdrop = cx.theme().background;
        let height = self.height();
        let pinned = self.viewport.top.clamp(Pixels::ZERO, height - ROW);
        let top = unpinned(self.corners, pinned);

        div()
            .relative()
            .w_full()
            .h(height)
            .children(self.rows(cx))
            .child(
                div()
                    .occlude()
                    .absolute()
                    .top(pinned)
                    .left_0()
                    .w_full()
                    .bg(backdrop)
                    .rounded_tl(top.top_left)
                    .rounded_tr(top.top_right)
                    .child(self.header(top, cx)),
            )
    }
}

#[derive(IntoElement)]
pub struct Grid<S: GridSource> {
    base: Div,
    state: Entity<GridState<S>>,
}

impl<S: GridSource> Styled for Grid<S> {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl<S: GridSource> InteractiveElement for Grid<S> {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl<S: GridSource> RenderOnce for Grid<S> {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let corners = radii(self.base.style(), window.rem_size());
        self.state.update(cx, |state, _| state.corners = corners);

        self.base.child(self.state)
    }
}

pub fn grid<S: GridSource>(state: &Entity<GridState<S>>) -> Grid<S> {
    Grid {
        base: div(),
        state: state.clone(),
    }
}
