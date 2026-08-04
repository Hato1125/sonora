use std::time::Duration;

use gpui::{
    AnyElement, App, AppContext as _, Context, Div, Entity, Hsla, IntoElement, ParentElement,
    Pixels, Render, SharedString, SharedUri, Styled, StyledImage as _, TextAlign, Window, div, img,
    prelude::FluentBuilder as _, px,
};
use gpui_component::button::Button;
use gpui_component::skeleton::Skeleton;
use gpui_component::table::{Column, Table, TableDelegate, TableEvent, TableState};
use gpui_component::{ActiveTheme as _, Disableable as _, Icon, Selectable as _, Sizable as _};
use state::{Library, LibraryState, Playback};
use workspace::Sidebar;

const CELL_PADDING: Pixels = px(8.);
const COVER: Pixels = px(28.);
const ROUNDED: Pixels = px(4.);

fn cover_art(url: Option<&str>, muted: Hsla) -> AnyElement {
    let Some(url) = url else {
        return blank(muted).into_any_element();
    };

    img(SharedUri::from(url.to_owned()))
        .size(COVER)
        .rounded(ROUNDED)
        .with_loading(|| {
            Skeleton::new()
                .size(COVER)
                .rounded(ROUNDED)
                .into_any_element()
        })
        .with_fallback(move || blank(muted).into_any_element())
        .into_any_element()
}

fn blank(muted: Hsla) -> Div {
    div()
        .size(COVER)
        .rounded(ROUNDED)
        .bg(muted.opacity(0.12))
        .flex()
        .items_center()
        .justify_center()
        .child(
            Icon::default()
                .path("icons/music.svg")
                .size(px(13.))
                .text_color(muted.opacity(0.5)),
        )
}

trait Centered {
    fn text_center(self) -> Self;
}

impl Centered for Column {
    fn text_center(mut self) -> Self {
        self.align = TextAlign::Center;
        self
    }
}

fn cell(width: Pixels, align: TextAlign) -> Div {
    let cell = div().w(width);
    match align {
        TextAlign::Left => cell.truncate(),
        TextAlign::Center => cell.flex().justify_center(),
        TextAlign::Right => cell.flex().justify_end(),
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    Tracks,
    Playlists,
}

impl Section {
    fn label(self) -> &'static str {
        match self {
            Section::Tracks => "Songs",
            Section::Playlists => "Playlists",
        }
    }

    fn columns(self, available: Pixels) -> Vec<Column> {
        let index = px(44.);
        let trailing = px(72.);
        let cover = COVER + CELL_PADDING * 2.;
        let flexible = (available - index - trailing - cover).max(px(240.));

        match self {
            Section::Tracks => vec![
                Column::new("index", "#").width(index).text_center(),
                Column::new("cover", "")
                    .width(cover)
                    .resizable(false)
                    .movable(false),
                Column::new("title", "Title")
                    .width(flexible * 0.42)
                    .sortable(),
                Column::new("artists", "Artist")
                    .width(flexible * 0.29)
                    .sortable(),
                Column::new("album", "Album")
                    .width(flexible * 0.29)
                    .sortable(),
                Column::new("duration", "Length")
                    .width(trailing)
                    .text_right(),
            ],
            Section::Playlists => vec![
                Column::new("index", "#").width(index).text_center(),
                Column::new("cover", "")
                    .width(cover)
                    .resizable(false)
                    .movable(false),
                Column::new("name", "Name")
                    .width(flexible * 0.55)
                    .sortable(),
                Column::new("owner", "Owner")
                    .width(flexible * 0.45)
                    .sortable(),
                Column::new("tracks", "Tracks").width(trailing).text_right(),
            ],
        }
    }
}

struct LibraryTable {
    library: Entity<Library>,
    section: Section,
    width: Pixels,
    columns: Vec<Column>,
}

impl LibraryTable {
    fn new(library: Entity<Library>, section: Section, width: Pixels) -> Self {
        Self {
            library,
            section,
            width,
            columns: section.columns(width),
        }
    }

    fn set_section(&mut self, section: Section) {
        self.section = section;
        self.columns = section.columns(self.width);
    }

    fn set_width(&mut self, width: Pixels) {
        self.width = width;
        self.columns = self.section.columns(width);
    }
}

impl TableDelegate for LibraryTable {
    fn columns_count(&self, _cx: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, cx: &App) -> usize {
        match self.library.read(cx).state() {
            LibraryState::Ready {
                tracks, playlists, ..
            } => match self.section {
                Section::Tracks => tracks.len(),
                Section::Playlists => playlists.len(),
            },
            _ => 0,
        }
    }

    fn column(&self, col_ix: usize, _cx: &App) -> &Column {
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

        if col_ix == 1 {
            let url = match section {
                Section::Tracks => tracks.get(row_ix).and_then(|track| track.cover.as_deref()),
                Section::Playlists => playlists
                    .get(row_ix)
                    .and_then(|playlist| playlist.cover.as_deref()),
            };
            return cell(cell_width, align).child(cover_art(url, muted));
        }

        let content = match section {
            Section::Tracks => tracks.get(row_ix).map(|track| match col_ix {
                0 => (SharedString::from(format!("{}", row_ix + 1)), true),
                2 => (SharedString::from(track.name.clone()), false),
                3 => (SharedString::from(track.artists.clone()), true),
                4 => (SharedString::from(track.album.clone()), true),
                _ => (SharedString::from(format_duration(track.duration)), true),
            }),
            Section::Playlists => playlists.get(row_ix).map(|playlist| match col_ix {
                0 => (SharedString::from(format!("{}", row_ix + 1)), true),
                2 => (SharedString::from(playlist.name.clone()), false),
                3 => (SharedString::from(playlist.owner.clone()), true),
                _ => (
                    SharedString::from(format!("{}", playlist.track_count)),
                    true,
                ),
            }),
        };

        let Some((text, dimmed)) = content else {
            return div();
        };

        cell(cell_width, align)
            .when(dimmed, |this| this.text_color(muted))
            .child(text)
    }
}

pub struct LibraryView {
    library: Entity<Library>,
    playback: Entity<Playback>,
    sidebar: Entity<Sidebar>,
    section: Section,
    width: Pixels,
    table: Entity<TableState<LibraryTable>>,
}

impl LibraryView {
    pub fn new(
        library: Entity<Library>,
        playback: Entity<Playback>,
        sidebar: Entity<Sidebar>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&library, |this, _, cx| {
            this.table.update(cx, |table, cx| table.refresh(cx));
            cx.notify();
        })
        .detach();
        cx.observe(&sidebar, |_, _, cx| cx.notify()).detach();

        let section = Section::Tracks;
        let width = content_width(window, &sidebar, cx);
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
            sidebar,
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
        let Some(track) = tracks.get(row_ix).cloned() else {
            return;
        };
        self.playback
            .update(cx, |playback, cx| playback.play(&track, cx));
    }

    fn select(&mut self, section: Section, cx: &mut Context<Self>) {
        self.section = section;
        self.table.update(cx, |table, cx| {
            table.delegate_mut().set_section(section);
            table.refresh(cx);
        });
        cx.notify();
    }

    fn tabs(&self, cx: &mut Context<Self>) -> AnyElement {
        let (tracks, playlists) = match self.library.read(cx).state() {
            LibraryState::Ready {
                tracks, playlists, ..
            } => (tracks.len(), playlists.len()),
            _ => (0, 0),
        };

        let library = self.library.clone();
        let loading = self.library.read(cx).is_loading();

        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .px_5()
            .py_3()
            .child(
                div()
                    .flex()
                    .gap_1()
                    .child(self.tab(Section::Tracks, tracks, cx))
                    .child(self.tab(Section::Playlists, playlists, cx)),
            )
            .child(
                Button::new("refresh")
                    .label("Refresh")
                    .small()
                    .icon(Icon::default().path("icons/refresh-cw.svg"))
                    .disabled(loading)
                    .on_click(move |_, _, cx| {
                        library.update(cx, |library, cx| library.refresh(cx));
                    }),
            )
            .into_any_element()
    }

    fn tab(&self, section: Section, count: usize, cx: &mut Context<Self>) -> Button {
        let label = if count == 0 {
            section.label().to_owned()
        } else {
            format!("{} ({count})", section.label())
        };

        Button::new(SharedString::from(section.label()))
            .label(label)
            .small()
            .selected(self.section == section)
            .disabled(count == 0)
            .on_click(cx.listener(move |this, _, _, cx| this.select(section, cx)))
    }
}

impl Render for LibraryView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let width = content_width(window, &self.sidebar, cx);
        if (width - self.width).abs() > px(1.) {
            self.width = width;
            self.table.update(cx, |table, cx| {
                table.delegate_mut().set_width(width);
                table.refresh(cx);
            });
        }

        div()
            .flex()
            .flex_col()
            .size_full()
            .child(self.tabs(cx))
            .child(div().flex_1().min_h_0().child(Table::new(&self.table)))
    }
}

fn format_duration(duration: Duration) -> String {
    let total = duration.as_secs();
    format!("{}:{:02}", total / 60, total % 60)
}

fn content_width(window: &Window, sidebar: &Entity<Sidebar>, cx: &App) -> Pixels {
    const CONTENT_CHROME: Pixels = px(20.);
    (window.viewport_size().width - sidebar.read(cx).occupied_width() - CONTENT_CHROME)
        .max(px(320.))
}
