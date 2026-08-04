use gpui::Entity;
use gpui::prelude::FluentBuilder as _;
use gpui::{App, Context, IntoElement, ParentElement, Pixels, Styled, Window, div, px};
use gpui_component::ActiveTheme as _;
use gpui_component::table::{Column as TableColumn, ColumnSort, TableDelegate, TableState};
use state::{Library, LibraryState};

use super::Section;
use super::columns::{CELL_PADDING, Field, cell, cover_art};

pub(super) struct LibraryTable {
    library: Entity<Library>,
    section: Section,
    width: Pixels,
    columns: Vec<TableColumn>,
    sort: Option<(Field, ColumnSort)>,
    order: Vec<usize>,
}

impl LibraryTable {
    pub(super) fn new(library: Entity<Library>, section: Section, width: Pixels) -> Self {
        Self {
            library,
            section,
            width,
            columns: section.columns(width, None),
            sort: None,
            order: Vec::new(),
        }
    }

    pub(super) fn set_section(&mut self, section: Section, cx: &App) {
        self.section = section;
        self.sort = None;
        self.rebuild(cx);
    }

    pub(super) fn set_width(&mut self, width: Pixels, cx: &App) {
        self.width = width;
        self.rebuild(cx);
    }

    pub(super) fn rebuild(&mut self, cx: &App) {
        self.columns = self.section.columns(self.width, self.sort);
        self.reorder(cx);
    }

    fn reorder(&mut self, cx: &App) {
        let LibraryState::Ready {
            tracks, playlists, ..
        } = self.library.read(cx).state()
        else {
            self.order = Vec::new();
            return;
        };

        let len = match self.section {
            Section::Tracks => tracks.len(),
            Section::Playlists => playlists.len(),
        };
        let mut order: Vec<usize> = (0..len).collect();

        if let Some((field, direction)) = self.sort {
            match direction {
                ColumnSort::Ascending => {
                    order.sort_by(|&a, &b| field.compare(a, b, tracks, playlists))
                }
                ColumnSort::Descending => {
                    order.sort_by(|&a, &b| field.compare(b, a, tracks, playlists))
                }
                ColumnSort::Default => (),
            }
        }

        self.order = order;
    }

    pub(super) fn row(&self, display_ix: usize) -> usize {
        self.order.get(display_ix).copied().unwrap_or(display_ix)
    }
}

impl TableDelegate for LibraryTable {
    fn columns_count(&self, _cx: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _cx: &App) -> usize {
        self.order.len()
    }

    fn perform_sort(
        &mut self,
        col_ix: usize,
        sort: ColumnSort,
        _window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) {
        let field = self.section.columns_for()[col_ix].field;
        self.sort = match sort {
            ColumnSort::Default => None,
            direction => Some((field, direction)),
        };
        self.reorder(cx);
    }

    fn column(&self, col_ix: usize, _cx: &App) -> &TableColumn {
        &self.columns[col_ix]
    }

    fn loading(&self, cx: &App) -> bool {
        self.library.read(cx).is_loading()
    }

    fn render_th(
        &mut self,
        col_ix: usize,
        _window: &mut Window,
        _cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let column = &self.columns[col_ix];
        let width = (column.width - CELL_PADDING * 2.).max(px(24.));
        cell(width, column.align).child(column.name.clone())
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        let section = self.section;
        let align = self.columns[col_ix].align;
        let cell_width = (self.columns[col_ix].width - CELL_PADDING * 2.).max(px(24.));
        let LibraryState::Ready {
            tracks, playlists, ..
        } = self.library.read(cx).state()
        else {
            return div();
        };

        let field = self.section.columns_for()[col_ix].field;
        let data_ix = self.row(row_ix);

        if field == Field::Cover {
            let url = match section {
                Section::Tracks => tracks.get(data_ix).and_then(|track| track.cover.as_deref()),
                Section::Playlists => playlists
                    .get(data_ix)
                    .and_then(|playlist| playlist.cover.as_deref()),
            };
            return div()
                .w(cell_width)
                .h_full()
                .flex()
                .items_center()
                .child(cover_art(url, muted));
        }

        let Some((text, dimmed)) = field.text(row_ix, data_ix, tracks, playlists) else {
            return div();
        };

        cell(cell_width, align)
            .when(dimmed, |this| this.text_color(muted))
            .child(text)
    }
}
