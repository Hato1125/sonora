mod columns;
mod table;
mod toolbar;

use gpui::prelude::*;
use gpui::{App, Context, Entity, Pixels, Render, Window, div, px};
use gpui_component::table::{Table, TableEvent, TableState};
use state::{Library, LibraryState, Playback};
use workspace::LibraryTab;

use columns::{Column, Field, PLAYLIST_COLUMNS, TRACK_COLUMNS};
use gpui_component::table::Column as TableColumn;
use gpui_component::table::ColumnSort;
use table::LibraryTable;

pub use toolbar::LibraryToolbar;

impl From<LibraryTab> for Section {
    fn from(tab: LibraryTab) -> Self {
        match tab {
            LibraryTab::Songs => Section::Tracks,
            LibraryTab::Playlists => Section::Playlists,
        }
    }
}

impl From<Section> for LibraryTab {
    fn from(section: Section) -> Self {
        match section {
            Section::Tracks => LibraryTab::Songs,
            Section::Playlists => LibraryTab::Playlists,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Tracks,
    Playlists,
}

impl Section {
    pub fn label(self) -> &'static str {
        match self {
            Section::Tracks => "Songs",
            Section::Playlists => "Playlists",
        }
    }

    fn columns_for(self) -> &'static [Column] {
        match self {
            Section::Tracks => TRACK_COLUMNS,
            Section::Playlists => PLAYLIST_COLUMNS,
        }
    }

    fn columns(self, available: Pixels, sort: Option<(Field, ColumnSort)>) -> Vec<TableColumn> {
        let columns = self.columns_for();
        let fixed: Pixels = columns
            .iter()
            .filter(|column| !column.is_fill())
            .map(|column| column.resolve(Pixels::ZERO))
            .fold(Pixels::ZERO, |total, width| total + width);
        let flexible = (available - fixed).max(px(240.));

        columns
            .iter()
            .map(|column| column.build(flexible, sort))
            .collect()
    }
}

pub struct LibraryView {
    library: Entity<Library>,
    playback: Entity<Playback>,
    section: Section,
    width: Pixels,
    table: Entity<TableState<LibraryTable>>,
}

impl LibraryView {
    pub fn new(
        library: Entity<Library>,
        playback: Entity<Playback>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&library, |this, _, cx| {
            this.table.update(cx, |table, cx| {
                table.delegate_mut().rebuild(cx);
                table.refresh(cx);
            });
            cx.notify();
        })
        .detach();

        let section = Section::Tracks;
        let width = content_width(window);
        let delegate = LibraryTable::new(library.clone(), section, width);
        let table = cx.new(|cx| TableState::new(delegate, window, cx).col_selectable(false));

        cx.subscribe(&table, |this, _, event, cx| {
            if let TableEvent::DoubleClickedRow(row_ix) = event {
                this.activate(*row_ix, cx);
            }
        })
        .detach();

        Self {
            library,
            playback,
            section,
            width,
            table,
        }
    }

    fn activate(&mut self, row_ix: usize, cx: &mut Context<Self>) {
        if self.section != Section::Tracks {
            return;
        }
        let LibraryState::Ready { tracks, .. } = self.library.read(cx).state() else {
            return;
        };
        let data_ix = self.table.read(cx).delegate().row(row_ix);
        let Some(track) = tracks.get(data_ix).cloned() else {
            return;
        };
        self.playback
            .update(cx, |playback, cx| playback.play(&track, cx));
    }

    pub fn section(&self) -> Section {
        self.section
    }

    pub fn counts(&self, cx: &App) -> (usize, usize) {
        match self.library.read(cx).state() {
            LibraryState::Ready {
                tracks, playlists, ..
            } => (tracks.len(), playlists.len()),
            _ => (0, 0),
        }
    }

    pub fn is_loading(&self, cx: &App) -> bool {
        self.library.read(cx).is_loading()
    }

    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.library.update(cx, |library, cx| library.refresh(cx));
    }

    pub fn select(&mut self, section: Section, cx: &mut Context<Self>) {
        self.section = section;
        self.table.update(cx, |table, cx| {
            table.delegate_mut().set_section(section, cx);
            table.refresh(cx);
        });
        cx.notify();
    }
}

impl Render for LibraryView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let width = content_width(window);
        if (width - self.width).abs() > px(1.) {
            self.width = width;
            self.table.update(cx, |table, cx| {
                table.delegate_mut().set_width(width, cx);
                table.refresh(cx);
            });
        }

        div().flex().flex_col().size_full().child(
            div()
                .flex_1()
                .min_h_0()
                .child(Table::new(&self.table).bordered(false)),
        )
    }
}

fn content_width(window: &Window) -> Pixels {
    const CHROME: f32 = 240.;
    (window.viewport_size().width - px(CHROME)).max(px(320.))
}
