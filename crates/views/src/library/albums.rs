use std::cmp::Ordering;

use gpui::{AnyElement, App, Entity, TextAlign};
use gpui_component::ActiveTheme as _;
use spotify::Album;
use state::{Library, LibraryState};
use ui::{Cell, ColumnSpec, GridSource, Width};

use crate::cells::{self, ARTWORK_COLUMN, NUMBER, TRAILING, YEAR};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum AlbumField {
    Index,
    Cover,
    Name,
    Artists,
    Year,
    TrackCount,
}

pub(super) const COLUMNS: &[ColumnSpec<AlbumField>] = &[
    ColumnSpec {
        field: AlbumField::Index,
        key: "index",
        header: "#",
        align: TextAlign::Center,
        width: Width::Fixed(NUMBER),
        flush: false,
        sortable: false,
    },
    ColumnSpec {
        field: AlbumField::Cover,
        key: "cover",
        header: "",
        align: TextAlign::Left,
        width: Width::Fixed(ARTWORK_COLUMN),
        flush: true,
        sortable: false,
    },
    ColumnSpec {
        field: AlbumField::Name,
        key: "name",
        header: "Album",
        align: TextAlign::Left,
        width: Width::Fill(0.55),
        flush: false,
        sortable: true,
    },
    ColumnSpec {
        field: AlbumField::Artists,
        key: "artists",
        header: "Artist",
        align: TextAlign::Left,
        width: Width::Fill(0.45),
        flush: false,
        sortable: true,
    },
    ColumnSpec {
        field: AlbumField::Year,
        key: "year",
        header: "Year",
        align: TextAlign::Right,
        width: Width::Fixed(YEAR),
        flush: false,
        sortable: true,
    },
    ColumnSpec {
        field: AlbumField::TrackCount,
        key: "tracks",
        header: "Tracks",
        align: TextAlign::Right,
        width: Width::Fixed(TRAILING),
        flush: false,
        sortable: true,
    },
];

pub(super) struct AlbumSource {
    library: Entity<Library>,
}

impl AlbumSource {
    pub(super) fn new(library: Entity<Library>) -> Self {
        Self { library }
    }

    pub(super) fn at(&self, row: usize, cx: &App) -> Option<Album> {
        self.albums(cx).get(row).cloned()
    }

    fn albums<'a>(&self, cx: &'a App) -> &'a [Album] {
        match self.library.read(cx).state() {
            LibraryState::Ready { albums, .. } => albums.as_slice(),
            _ => &[],
        }
    }
}

impl GridSource for AlbumSource {
    type Field = AlbumField;

    fn columns(&self) -> &'static [ColumnSpec<AlbumField>] {
        COLUMNS
    }

    fn rows(&self, cx: &App) -> usize {
        self.albums(cx).len()
    }

    fn is_loading(&self, cx: &App) -> bool {
        self.library.read(cx).is_loading()
    }

    fn cell(&self, cell: Cell<AlbumField>, cx: &mut App) -> AnyElement {
        let muted = cx.theme().muted_foreground;

        if cell.field == AlbumField::Index {
            return cells::dim(&cell, format!("{}", cell.display + 1), muted);
        }

        let Some(album) = self.albums(cx).get(cell.row) else {
            return cells::blank(&cell);
        };

        match cell.field {
            AlbumField::Cover => cells::artwork(&cell, album.cover.clone()),
            AlbumField::Name => cells::text(&cell, album.name.clone()),
            AlbumField::Artists => cells::dim(&cell, album.artists.clone(), muted),
            AlbumField::Year => cells::dim(&cell, year(album), muted),
            AlbumField::TrackCount => {
                cells::dim(&cell, format!("{}", album.track_count), muted)
            }
            AlbumField::Index => cells::blank(&cell),
        }
    }

    fn compare(&self, field: AlbumField, a: usize, b: usize, cx: &App) -> Ordering {
        let albums = self.albums(cx);
        let text = |index: usize, pick: fn(&Album) -> &String| {
            albums
                .get(index)
                .map(|album| pick(album).to_lowercase())
                .unwrap_or_default()
        };

        match field {
            AlbumField::Name => text(a, |album| &album.name).cmp(&text(b, |album| &album.name)),
            AlbumField::Artists => {
                text(a, |album| &album.artists).cmp(&text(b, |album| &album.artists))
            }
            AlbumField::Year => albums
                .get(a)
                .map(|album| album.year)
                .cmp(&albums.get(b).map(|album| album.year)),
            AlbumField::TrackCount => albums
                .get(a)
                .map(|album| album.track_count)
                .cmp(&albums.get(b).map(|album| album.track_count)),
            AlbumField::Index | AlbumField::Cover => a.cmp(&b),
        }
    }
}

fn year(album: &Album) -> String {
    if album.year > 0 {
        format!("{}", album.year)
    } else {
        String::new()
    }
}
