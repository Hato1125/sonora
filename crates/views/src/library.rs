use std::cmp::Ordering;
use std::time::Duration;

use gpui::{
    AnyElement, App, AppContext as _, Context, Div, Edges, Entity, Hsla, IntoElement,
    ParentElement, Pixels, Render, SharedString, SharedUri, Styled, StyledImage as _, TextAlign,
    Window, div, img, prelude::FluentBuilder as _, px,
};
use gpui_component::skeleton::Skeleton;
use gpui_component::table::{Column, ColumnSort, Table, TableDelegate, TableEvent, TableState};
use gpui_component::{ActiveTheme as _, Icon};
use spotify::{Playlist, Track};
use state::{Library, LibraryState, Playback};

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

fn cover_paddings() -> Edges<Pixels> {
    Edges {
        top: px(0.),
        bottom: px(0.),
        left: CELL_PADDING,
        right: CELL_PADDING,
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
enum Field {
    Index,
    Cover,
    Title,
    Artists,
    Album,
    Duration,
    Name,
    Owner,
    TrackCount,
}

#[derive(Clone, Copy)]
enum Span {
    Index,
    Cover,
    Trailing,
    Share(f32),
}

struct Spec {
    field: Field,
    key: &'static str,
    header: &'static str,
    align: TextAlign,
    span: Span,
    sortable: bool,
}

const INDEX: Pixels = px(44.);
const TRAILING: Pixels = px(72.);

const TRACK_COLUMNS: &[Spec] = &[
    Spec {
        field: Field::Index,
        key: "index",
        header: "#",
        align: TextAlign::Center,
        span: Span::Index,
        sortable: false,
    },
    Spec {
        field: Field::Cover,
        key: "cover",
        header: "",
        align: TextAlign::Left,
        span: Span::Cover,
        sortable: false,
    },
    Spec {
        field: Field::Title,
        key: "title",
        header: "Title",
        align: TextAlign::Left,
        span: Span::Share(0.42),
        sortable: true,
    },
    Spec {
        field: Field::Artists,
        key: "artists",
        header: "Artist",
        align: TextAlign::Left,
        span: Span::Share(0.29),
        sortable: true,
    },
    Spec {
        field: Field::Album,
        key: "album",
        header: "Album",
        align: TextAlign::Left,
        span: Span::Share(0.29),
        sortable: true,
    },
    Spec {
        field: Field::Duration,
        key: "duration",
        header: "Length",
        align: TextAlign::Right,
        span: Span::Trailing,
        sortable: true,
    },
];

const PLAYLIST_COLUMNS: &[Spec] = &[
    Spec {
        field: Field::Index,
        key: "index",
        header: "#",
        align: TextAlign::Center,
        span: Span::Index,
        sortable: false,
    },
    Spec {
        field: Field::Cover,
        key: "cover",
        header: "",
        align: TextAlign::Left,
        span: Span::Cover,
        sortable: false,
    },
    Spec {
        field: Field::Name,
        key: "name",
        header: "Name",
        align: TextAlign::Left,
        span: Span::Share(0.55),
        sortable: true,
    },
    Spec {
        field: Field::Owner,
        key: "owner",
        header: "Owner",
        align: TextAlign::Left,
        span: Span::Share(0.45),
        sortable: true,
    },
    Spec {
        field: Field::TrackCount,
        key: "tracks",
        header: "Tracks",
        align: TextAlign::Right,
        span: Span::Trailing,
        sortable: true,
    },
];

impl Spec {
    fn width(&self, flexible: Pixels) -> Pixels {
        match self.span {
            Span::Index => INDEX,
            Span::Cover => COVER + CELL_PADDING * 2.,
            Span::Trailing => TRAILING,
            Span::Share(share) => flexible * share,
        }
    }

    fn column(&self, flexible: Pixels, sort: Option<(Field, ColumnSort)>) -> Column {
        let column = Column::new(self.key, self.header)
            .width(self.width(flexible))
            .resizable(false)
            .movable(false);

        let column = match self.span {
            Span::Cover => column.paddings(cover_paddings()),
            _ => column,
        };

        let column = match self.align {
            TextAlign::Center => column.text_center(),
            TextAlign::Right => column.text_right(),
            TextAlign::Left => column,
        };

        if !self.sortable {
            return column;
        }

        let direction = match sort {
            Some((field, direction)) if field == self.field => direction,
            _ => ColumnSort::Default,
        };
        column.sort(direction)
    }
}

impl Field {
    fn compare(self, a: usize, b: usize, tracks: &[Track], playlists: &[Playlist]) -> Ordering {
        let track_text = |index: usize, pick: fn(&Track) -> &String| {
            tracks
                .get(index)
                .map(|track| pick(track).to_lowercase())
                .unwrap_or_default()
        };
        let playlist_text = |index: usize, pick: fn(&Playlist) -> &String| {
            playlists
                .get(index)
                .map(|playlist| pick(playlist).to_lowercase())
                .unwrap_or_default()
        };

        match self {
            Field::Title => {
                track_text(a, |track| &track.name).cmp(&track_text(b, |track| &track.name))
            }
            Field::Artists => {
                track_text(a, |track| &track.artists).cmp(&track_text(b, |track| &track.artists))
            }
            Field::Album => {
                track_text(a, |track| &track.album).cmp(&track_text(b, |track| &track.album))
            }
            Field::Duration => tracks
                .get(a)
                .map(|track| track.duration)
                .cmp(&tracks.get(b).map(|track| track.duration)),
            Field::Name => playlist_text(a, |playlist| &playlist.name)
                .cmp(&playlist_text(b, |playlist| &playlist.name)),
            Field::Owner => playlist_text(a, |playlist| &playlist.owner)
                .cmp(&playlist_text(b, |playlist| &playlist.owner)),
            Field::TrackCount => playlists
                .get(a)
                .map(|playlist| playlist.track_count)
                .cmp(&playlists.get(b).map(|playlist| playlist.track_count)),
            Field::Index | Field::Cover => a.cmp(&b),
        }
    }

    fn text(
        self,
        display_ix: usize,
        row_ix: usize,
        tracks: &[Track],
        playlists: &[Playlist],
    ) -> Option<(SharedString, bool)> {
        match self {
            Field::Cover => None,
            Field::Index => Some((SharedString::from(format!("{}", display_ix + 1)), true)),
            Field::Title => tracks
                .get(row_ix)
                .map(|track| (SharedString::from(track.name.clone()), false)),
            Field::Artists => tracks
                .get(row_ix)
                .map(|track| (SharedString::from(track.artists.clone()), true)),
            Field::Album => tracks
                .get(row_ix)
                .map(|track| (SharedString::from(track.album.clone()), true)),
            Field::Duration => tracks
                .get(row_ix)
                .map(|track| (SharedString::from(format_duration(track.duration)), true)),
            Field::Name => playlists
                .get(row_ix)
                .map(|playlist| (SharedString::from(playlist.name.clone()), false)),
            Field::Owner => playlists
                .get(row_ix)
                .map(|playlist| (SharedString::from(playlist.owner.clone()), true)),
            Field::TrackCount => playlists.get(row_ix).map(|playlist| {
                (
                    SharedString::from(format!("{}", playlist.track_count)),
                    true,
                )
            }),
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

    fn specs(self) -> &'static [Spec] {
        match self {
            Section::Tracks => TRACK_COLUMNS,
            Section::Playlists => PLAYLIST_COLUMNS,
        }
    }

    fn columns(self, available: Pixels, sort: Option<(Field, ColumnSort)>) -> Vec<Column> {
        let specs = self.specs();
        let fixed: Pixels = specs
            .iter()
            .filter(|spec| !matches!(spec.span, Span::Share(_)))
            .map(|spec| spec.width(Pixels::ZERO))
            .fold(Pixels::ZERO, |total, width| total + width);
        let flexible = (available - fixed).max(px(240.));

        specs
            .iter()
            .map(|spec| spec.column(flexible, sort))
            .collect()
    }
}

struct LibraryTable {
    library: Entity<Library>,
    section: Section,
    width: Pixels,
    columns: Vec<Column>,
    sort: Option<(Field, ColumnSort)>,
    order: Vec<usize>,
}

impl LibraryTable {
    fn new(library: Entity<Library>, section: Section, width: Pixels) -> Self {
        Self {
            library,
            section,
            width,
            columns: section.columns(width, None),
            sort: None,
            order: Vec::new(),
        }
    }

    fn set_section(&mut self, section: Section, cx: &App) {
        self.section = section;
        self.sort = None;
        self.rebuild(cx);
    }

    fn set_width(&mut self, width: Pixels, cx: &App) {
        self.width = width;
        self.rebuild(cx);
    }

    fn rebuild(&mut self, cx: &App) {
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

    fn row(&self, display_ix: usize) -> usize {
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
        let field = self.section.specs()[col_ix].field;
        self.sort = match sort {
            ColumnSort::Default => None,
            direction => Some((field, direction)),
        };
        self.reorder(cx);
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

        let field = self.section.specs()[col_ix].field;
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

        div()
            .flex()
            .flex_col()
            .size_full()
            .child(div().flex_1().min_h_0().child(Table::new(&self.table)))
    }
}

fn format_duration(duration: Duration) -> String {
    let total = duration.as_secs();
    format!("{}:{:02}", total / 60, total % 60)
}

fn content_width(window: &Window) -> Pixels {
    const CHROME: f32 = 240.;
    (window.viewport_size().width - px(CHROME)).max(px(320.))
}
