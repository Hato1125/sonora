use ui::ActiveTheme as _;
use std::cmp::Ordering;

use gpui::{AnyElement, App, TextAlign};
use spotify::Track;
use ui::{Cell, ColumnSpec, GridSource, Width, clock};

use crate::cells::{self, ALWAYS, ARTWORK_COLUMN, NUMBER, ROOMY, SNUG, TRAILING, WIDE};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum TrackField {
    Index,
    Cover,
    Title,
    Artists,
    Album,
    Duration,
}

pub(crate) const LIBRARY_COLUMNS: &[ColumnSpec<TrackField>] = &[
    ColumnSpec {
        field: TrackField::Index,
        key: "index",
        header: "#",
        align: TextAlign::Center,
        width: Width::Fixed(NUMBER),
        flush: false,
        sortable: false,
        hide_below: ALWAYS,
    },
    ColumnSpec {
        field: TrackField::Cover,
        key: "cover",
        header: "",
        align: TextAlign::Left,
        width: Width::Fixed(ARTWORK_COLUMN),
        flush: true,
        sortable: false,
        hide_below: SNUG,
    },
    ColumnSpec {
        field: TrackField::Title,
        key: "title",
        header: "Title",
        align: TextAlign::Left,
        width: Width::Fill(0.42),
        flush: false,
        sortable: true,
        hide_below: ALWAYS,
    },
    ColumnSpec {
        field: TrackField::Artists,
        key: "artists",
        header: "Artist",
        align: TextAlign::Left,
        width: Width::Fill(0.29),
        flush: false,
        sortable: true,
        hide_below: ROOMY,
    },
    ColumnSpec {
        field: TrackField::Album,
        key: "album",
        header: "Album",
        align: TextAlign::Left,
        width: Width::Fill(0.29),
        flush: false,
        sortable: true,
        hide_below: WIDE,
    },
    ColumnSpec {
        field: TrackField::Duration,
        key: "duration",
        header: "Length",
        align: TextAlign::Right,
        width: Width::Fixed(TRAILING),
        flush: false,
        sortable: true,
        hide_below: SNUG,
    },
];

pub(crate) const ALBUM_COLUMNS: &[ColumnSpec<TrackField>] = &[
    ColumnSpec {
        field: TrackField::Index,
        key: "index",
        header: "#",
        align: TextAlign::Center,
        width: Width::Fixed(NUMBER),
        flush: false,
        sortable: false,
        hide_below: ALWAYS,
    },
    ColumnSpec {
        field: TrackField::Title,
        key: "title",
        header: "Title",
        align: TextAlign::Left,
        width: Width::Fill(0.62),
        flush: false,
        sortable: true,
        hide_below: ALWAYS,
    },
    ColumnSpec {
        field: TrackField::Artists,
        key: "artists",
        header: "Artist",
        align: TextAlign::Left,
        width: Width::Fill(0.38),
        flush: false,
        sortable: true,
        hide_below: ROOMY,
    },
    ColumnSpec {
        field: TrackField::Duration,
        key: "duration",
        header: "Length",
        align: TextAlign::Right,
        width: Width::Fixed(TRAILING),
        flush: false,
        sortable: true,
        hide_below: SNUG,
    },
];

pub(crate) trait Tracks: 'static {
    fn tracks<'a>(&self, cx: &'a App) -> &'a [Track];
    fn is_loading(&self, cx: &App) -> bool;
}

pub(crate) struct TrackSource {
    columns: &'static [ColumnSpec<TrackField>],
    provider: Box<dyn Tracks>,
}

impl TrackSource {
    pub(crate) fn new(columns: &'static [ColumnSpec<TrackField>], provider: impl Tracks) -> Self {
        Self {
            columns,
            provider: Box::new(provider),
        }
    }

    pub(crate) fn at(&self, row: usize, cx: &App) -> Option<Track> {
        self.provider.tracks(cx).get(row).cloned()
    }
}

impl GridSource for TrackSource {
    type Field = TrackField;

    fn columns(&self) -> &'static [ColumnSpec<TrackField>] {
        self.columns
    }

    fn rows(&self, cx: &App) -> usize {
        self.provider.tracks(cx).len()
    }

    fn is_loading(&self, cx: &App) -> bool {
        self.provider.is_loading(cx)
    }

    fn cell(&self, cell: Cell<TrackField>, cx: &mut App) -> AnyElement {
        let muted = cx.theme().muted_foreground;

        if cell.field == TrackField::Index {
            return cells::dim(&cell, format!("{}", cell.display + 1), muted);
        }

        let Some(track) = self.provider.tracks(cx).get(cell.row) else {
            return cells::blank(&cell);
        };

        match cell.field {
            TrackField::Cover => cells::artwork(&cell, track.cover.clone()),
            TrackField::Title => cells::text(&cell, track.name.clone()),
            TrackField::Artists => cells::dim(&cell, track.artists.clone(), muted),
            TrackField::Album => cells::dim(&cell, track.album.clone(), muted),
            TrackField::Duration => cells::dim(&cell, clock(track.duration), muted),
            TrackField::Index => cells::blank(&cell),
        }
    }

    fn compare(&self, field: TrackField, a: usize, b: usize, cx: &App) -> Ordering {
        let tracks = self.provider.tracks(cx);
        let text = |index: usize, pick: fn(&Track) -> &String| {
            tracks
                .get(index)
                .map(|track| pick(track).to_lowercase())
                .unwrap_or_default()
        };

        match field {
            TrackField::Title => text(a, |track| &track.name).cmp(&text(b, |track| &track.name)),
            TrackField::Artists => {
                text(a, |track| &track.artists).cmp(&text(b, |track| &track.artists))
            }
            TrackField::Album => text(a, |track| &track.album).cmp(&text(b, |track| &track.album)),
            TrackField::Duration => tracks
                .get(a)
                .map(|track| track.duration)
                .cmp(&tracks.get(b).map(|track| track.duration)),
            TrackField::Index | TrackField::Cover => a.cmp(&b),
        }
    }
}
