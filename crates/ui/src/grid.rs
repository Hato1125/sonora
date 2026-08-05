use std::cmp::Ordering;

use gpui::prelude::*;
use gpui::{AnyElement, App, Context, Div, Edges, Pixels, TextAlign, Window, div, px};
use gpui_component::table::{Column as TableColumn, ColumnSort, Table, TableDelegate, TableState};

const PADDING: Pixels = px(8.);
const TRAIL: Pixels = px(12.);
const MIN_CELL: Pixels = px(24.);
const MIN_FLEXIBLE: Pixels = px(120.);

#[derive(Clone, Copy)]
pub enum Width {
    Fixed(Pixels),
    Fill(f32),
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
    let frame = div().w(width);
    match align {
        TextAlign::Left => frame.truncate(),
        TextAlign::Center => frame.flex().justify_center(),
        TextAlign::Right => frame.flex().justify_end(),
    }
}

fn flush_paddings() -> Edges<Pixels> {
    Edges {
        top: px(0.),
        bottom: px(0.),
        left: PADDING,
        right: PADDING,
    }
}

pub struct GridDelegate<S: GridSource> {
    source: S,
    specs: Vec<&'static ColumnSpec<S::Field>>,
    columns: Vec<TableColumn>,
    width: Pixels,
    sort: Option<(S::Field, ColumnSort)>,
    order: Vec<usize>,
}

impl<S: GridSource> GridDelegate<S> {
    pub fn new(source: S, width: Pixels, cx: &App) -> Self {
        let (specs, columns) = build(source.columns(), width, None);
        let mut delegate = Self {
            source,
            specs,
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

    fn relayout(&mut self) {
        let (specs, columns) = build(self.source.columns(), self.width, self.sort);
        self.specs = specs;
        self.columns = columns;
    }

    pub fn row(&self, display: usize) -> usize {
        self.order.get(display).copied().unwrap_or(display)
    }

    fn reorder(&mut self, cx: &App) {
        let mut order: Vec<usize> = (0..self.source.rows(cx)).collect();

        if let Some((field, direction)) = self.sort {
            match direction {
                ColumnSort::Ascending => {
                    order.sort_by(|&a, &b| self.source.compare(field, a, b, cx))
                }
                ColumnSort::Descending => {
                    order.sort_by(|&a, &b| self.source.compare(field, b, a, cx))
                }
                ColumnSort::Default => (),
            }
        }

        self.order = order;
    }

    fn inner_width(&self, col_ix: usize) -> Pixels {
        let trailing = col_ix + 1 == self.columns.len();
        let gutter = if trailing { TRAIL } else { Pixels::ZERO };
        (self.columns[col_ix].width - PADDING * 2. - gutter).max(MIN_CELL)
    }
}

type Layout<F> = (Vec<&'static ColumnSpec<F>>, Vec<TableColumn>);

fn build<F: Copy + PartialEq + 'static>(
    specs: &'static [ColumnSpec<F>],
    available: Pixels,
    sort: Option<(F, ColumnSort)>,
) -> Layout<F> {
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

    let mut columns: Vec<TableColumn> = visible
        .iter()
        .map(|spec| {
            let mut column = TableColumn::new(spec.key, spec.header)
                .width(spec.resolve(flexible, shares))
                .resizable(false)
                .movable(false);

            if spec.flush {
                column = column.paddings(flush_paddings());
            }
            column.align = spec.align;

            if spec.sortable {
                let direction = match sort {
                    Some((field, direction)) if field == spec.field => direction,
                    _ => ColumnSort::Default,
                };
                column = column.sort(direction);
            }
            column
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

    (visible, columns)
}

impl<S: GridSource> TableDelegate for GridDelegate<S> {
    fn columns_count(&self, _cx: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _cx: &App) -> usize {
        self.order.len()
    }

    fn column(&self, col_ix: usize, _cx: &App) -> &TableColumn {
        &self.columns[col_ix]
    }

    fn loading(&self, cx: &App) -> bool {
        self.source.is_loading(cx)
    }

    fn perform_sort(
        &mut self,
        col_ix: usize,
        sort: ColumnSort,
        _window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) {
        let field = self.specs[col_ix].field;
        self.sort = match sort {
            ColumnSort::Default => None,
            direction => Some((field, direction)),
        };
        self.reorder(cx);
    }

    fn render_th(
        &mut self,
        col_ix: usize,
        _window: &mut Window,
        _cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let width = self.inner_width(col_ix);
        let column = &self.columns[col_ix];
        frame(width, column.align).child(column.name.clone())
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let spec = self.specs[col_ix];
        let cell = Cell {
            field: spec.field,
            width: self.inner_width(col_ix),
            align: spec.align,
            display: row_ix,
            row: self.row(row_ix),
        };
        self.source.cell(cell, cx)
    }
}

pub type GridState<S> = TableState<GridDelegate<S>>;

pub fn grid<S: GridSource>(state: &gpui::Entity<GridState<S>>) -> Table<GridDelegate<S>> {
    Table::new(state).bordered(false)
}
