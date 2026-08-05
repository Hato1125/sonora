use std::cmp::Ordering;
use ui::ActiveTheme as _;

use gpui::prelude::*;
use gpui::{AnyElement, App, Entity, TextAlign, div};
use spotify::Album;
use state::{Library, LibraryState, Playback};
use ui::{Cell, ColumnSpec, GridSource, Width};

use crate::cells::{self, ALWAYS, NUMBER, ROOMY, TRAILING, WIDE, YEAR};

const PLAY: &str = "icons/play.svg";

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
        hide_below: ALWAYS,
    },
    ColumnSpec {
        field: AlbumField::Cover,
        key: "cover",
        header: "",
        align: TextAlign::Left,
        width: Width::Thumb,
        flush: true,
        sortable: false,
        hide_below: ALWAYS,
    },
    ColumnSpec {
        field: AlbumField::Name,
        key: "name",
        header: "Album",
        align: TextAlign::Left,
        width: Width::Fill(0.55),
        flush: false,
        sortable: true,
        hide_below: ALWAYS,
    },
    ColumnSpec {
        field: AlbumField::Artists,
        key: "artists",
        header: "Artist",
        align: TextAlign::Left,
        width: Width::Fill(0.45),
        flush: false,
        sortable: true,
        hide_below: ALWAYS,
    },
    ColumnSpec {
        field: AlbumField::Year,
        key: "year",
        header: "Year",
        align: TextAlign::Right,
        width: Width::Fixed(YEAR),
        flush: false,
        sortable: true,
        hide_below: ROOMY,
    },
    ColumnSpec {
        field: AlbumField::TrackCount,
        key: "tracks",
        header: "Tracks",
        align: TextAlign::Right,
        width: Width::Fixed(TRAILING),
        flush: false,
        sortable: true,
        hide_below: WIDE,
    },
];

pub(super) struct AlbumSource {
    library: Entity<Library>,
    playback: Entity<Playback>,
}

impl AlbumSource {
    pub(super) fn new(library: Entity<Library>, playback: Entity<Playback>) -> Self {
        Self { library, playback }
    }

    fn index_cell(&self, cell: &Cell<AlbumField>, album: &Album, cx: &App) -> AnyElement {
        let theme = *cx.theme();
        let resting = div()
            .text_color(theme.muted_foreground)
            .child(format!("{}", cell.display + 1))
            .into_any_element();

        let playback = self.playback.clone();
        let id = album.id.clone();
        let press: Option<Box<dyn Fn(&mut App)>> = Some(Box::new(move |cx: &mut App| {
            playback.update(cx, |playback, cx| playback.play_album(&id, cx));
        }));

        cells::transport(
            cell,
            resting,
            cells::Transport {
                icon: PLAY,
                color: theme.foreground,
                press,
            },
        )
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

    fn matches(&self, row: usize, query: &str, cx: &App) -> bool {
        self.at(row, cx).is_some_and(|album| {
            let haystack = format!("{} {} {}", album.name, album.artists, album.year);
            haystack.to_lowercase().contains(query)
        })
    }

    fn is_loading(&self, cx: &App) -> bool {
        self.library.read(cx).is_loading()
    }

    fn cell(&self, cell: Cell<AlbumField>, cx: &mut App) -> AnyElement {
        let muted = cx.theme().muted_foreground;

        let Some(album) = self.albums(cx).get(cell.row) else {
            return cells::blank(&cell);
        };

        if cell.field == AlbumField::Index {
            return self.index_cell(&cell, album, cx);
        }

        match cell.field {
            AlbumField::Cover => cells::artwork(&cell, album.cover.clone()),
            AlbumField::Name => cells::text(&cell, album.name.clone()),
            AlbumField::Artists => cells::dim(&cell, album.artists.clone(), muted),
            AlbumField::Year => cells::dim(&cell, year(album), muted),
            AlbumField::TrackCount => cells::dim(&cell, format!("{}", album.track_count), muted),
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
