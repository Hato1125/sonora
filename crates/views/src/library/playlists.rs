use std::cmp::Ordering;
use ui::ActiveTheme as _;

use gpui::{AnyElement, App, Entity, TextAlign};
use spotify::Playlist;
use state::{Library, LibraryState};
use ui::{Cell, ColumnSpec, GridSource, Width};

use crate::cells::{self, ALWAYS, ARTWORK_COLUMN, NUMBER, ROOMY, SNUG, TRAILING};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum PlaylistField {
    Index,
    Cover,
    Name,
    Owner,
    TrackCount,
}

pub(super) const COLUMNS: &[ColumnSpec<PlaylistField>] = &[
    ColumnSpec {
        field: PlaylistField::Index,
        key: "index",
        header: "#",
        align: TextAlign::Center,
        width: Width::Fixed(NUMBER),
        flush: false,
        sortable: false,
        hide_below: ALWAYS,
    },
    ColumnSpec {
        field: PlaylistField::Cover,
        key: "cover",
        header: "",
        align: TextAlign::Left,
        width: Width::Fixed(ARTWORK_COLUMN),
        flush: true,
        sortable: false,
        hide_below: ALWAYS,
    },
    ColumnSpec {
        field: PlaylistField::Name,
        key: "name",
        header: "Name",
        align: TextAlign::Left,
        width: Width::Fill(0.55),
        flush: false,
        sortable: true,
        hide_below: ALWAYS,
    },
    ColumnSpec {
        field: PlaylistField::Owner,
        key: "owner",
        header: "Owner",
        align: TextAlign::Left,
        width: Width::Fill(0.45),
        flush: false,
        sortable: true,
        hide_below: ROOMY,
    },
    ColumnSpec {
        field: PlaylistField::TrackCount,
        key: "tracks",
        header: "Tracks",
        align: TextAlign::Right,
        width: Width::Fixed(TRAILING),
        flush: false,
        sortable: true,
        hide_below: SNUG,
    },
];

pub(super) struct PlaylistSource {
    library: Entity<Library>,
}

impl PlaylistSource {
    pub(super) fn new(library: Entity<Library>) -> Self {
        Self { library }
    }

    fn playlists<'a>(&self, cx: &'a App) -> &'a [Playlist] {
        match self.library.read(cx).state() {
            LibraryState::Ready { playlists, .. } => playlists.as_slice(),
            _ => &[],
        }
    }
}

impl GridSource for PlaylistSource {
    type Field = PlaylistField;

    fn columns(&self) -> &'static [ColumnSpec<PlaylistField>] {
        COLUMNS
    }

    fn rows(&self, cx: &App) -> usize {
        self.playlists(cx).len()
    }

    fn is_loading(&self, cx: &App) -> bool {
        self.library.read(cx).is_loading()
    }

    fn cell(&self, cell: Cell<PlaylistField>, cx: &mut App) -> AnyElement {
        let muted = cx.theme().muted_foreground;

        if cell.field == PlaylistField::Index {
            return cells::dim(&cell, format!("{}", cell.display + 1), muted);
        }

        let Some(playlist) = self.playlists(cx).get(cell.row) else {
            return cells::blank(&cell);
        };

        match cell.field {
            PlaylistField::Cover => cells::artwork(&cell, playlist.cover.clone()),
            PlaylistField::Name => cells::text(&cell, playlist.name.clone()),
            PlaylistField::Owner => cells::dim(&cell, playlist.owner.clone(), muted),
            PlaylistField::TrackCount => {
                cells::dim(&cell, format!("{}", playlist.track_count), muted)
            }
            PlaylistField::Index => cells::blank(&cell),
        }
    }

    fn compare(&self, field: PlaylistField, a: usize, b: usize, cx: &App) -> Ordering {
        let playlists = self.playlists(cx);
        let text = |index: usize, pick: fn(&Playlist) -> &String| {
            playlists
                .get(index)
                .map(|playlist| pick(playlist).to_lowercase())
                .unwrap_or_default()
        };

        match field {
            PlaylistField::Name => {
                text(a, |playlist| &playlist.name).cmp(&text(b, |playlist| &playlist.name))
            }
            PlaylistField::Owner => {
                text(a, |playlist| &playlist.owner).cmp(&text(b, |playlist| &playlist.owner))
            }
            PlaylistField::TrackCount => playlists
                .get(a)
                .map(|playlist| playlist.track_count)
                .cmp(&playlists.get(b).map(|playlist| playlist.track_count)),
            PlaylistField::Index | PlaylistField::Cover => a.cmp(&b),
        }
    }
}
