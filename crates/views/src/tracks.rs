use std::cmp::Ordering;
use std::rc::Rc;
use ui::ActiveTheme as _;

use gpui::{AnyElement, App, Entity, Hsla, TextAlign};
use jiff::Timestamp;
use router::Destination;
use spotify::Track;
use state::{Playback, PlaybackState};
use ui::{Cell, ColumnSpec, GridSource, Width, clock};

use crate::cells::{self, ALWAYS, DATE, NUMBER, ROOMY, SNUG, TRAILING, WIDE};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum TrackField {
    Index,
    Cover,
    Title,
    Artists,
    Album,
    AddedAt,
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
        width: Width::Thumb,
        flush: true,
        sortable: false,
        hide_below: ALWAYS,
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
        field: TrackField::AddedAt,
        key: "added-at",
        header: "Date added",
        align: TextAlign::Left,
        width: Width::Fixed(DATE),
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

pub(crate) type PlaybackStatus = (Option<String>, PlaybackState);

pub(crate) fn playback_status(playback: &Entity<Playback>, cx: &App) -> PlaybackStatus {
    let playback = playback.read(cx);
    let track = playback.track().and_then(|track| track.id.clone());
    (track, playback.state().clone())
}

pub(crate) trait Tracks: 'static {
    fn tracks<'a>(&self, cx: &'a App) -> &'a [Track];
    fn is_loading(&self, cx: &App) -> bool;
}

pub(crate) struct TrackSource {
    columns: &'static [ColumnSpec<TrackField>],
    provider: Rc<dyn Tracks>,
    playback: Entity<Playback>,
}

impl TrackSource {
    pub(crate) fn new(
        columns: &'static [ColumnSpec<TrackField>],
        provider: impl Tracks,
        playback: Entity<Playback>,
    ) -> Self {
        Self {
            columns,
            provider: Rc::new(provider),
            playback,
        }
    }

    fn artist_cell(&self, cell: &Cell<TrackField>, track: &Track, color: Hsla) -> AnyElement {
        cells::artists(
            cell,
            track.artist_refs.clone(),
            track.artists.clone(),
            color,
        )
    }

    fn album_cell(&self, cell: &Cell<TrackField>, track: &Track, color: Hsla) -> AnyElement {
        let Some(album) = track.album_id.clone() else {
            return cells::dim(cell, track.album.clone(), color);
        };

        cells::link(
            cell,
            "album",
            track.album.clone(),
            color,
            Destination::Album(album.into()),
        )
    }

    fn index_cell(&self, cell: &Cell<TrackField>, track: &Track, cx: &App) -> AnyElement {
        let state = self.now_playing(track, cx);
        let (preload, press) = match track.playable {
            false => (None, None),
            true => {
                let playback = self.playback.clone();
                let preload_track = track.clone();
                let preload: Option<Box<dyn Fn(&mut App)>> = Some(Box::new(move |cx| {
                    playback.update(cx, |playback, _| playback.preload(&preload_track));
                }));
                let provider = self.provider.clone();
                let row = cell.row;
                let press = cells::toggle(&self.playback, state.clone(), move |playback, cx| {
                    let queued = provider.tracks(cx).to_vec();
                    playback.start(queued, row, cx)
                });
                (preload, press)
            }
        };

        cells::index(cell, state, track.playable, preload, press, cx)
    }

    fn now_playing(&self, track: &Track, cx: &App) -> Option<PlaybackState> {
        let playback = self.playback.read(cx);
        let current = playback.track()?;
        (current.id.is_some() && current.id == track.id).then(|| playback.state().clone())
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

    fn matches(&self, row: usize, query: &str, cx: &App) -> bool {
        self.at(row, cx).is_some_and(|track| {
            let haystack = format!("{} {} {}", track.name, track.artists, track.album);
            haystack.to_lowercase().contains(query)
        })
    }

    fn is_loading(&self, cx: &App) -> bool {
        self.provider.is_loading(cx)
    }

    fn cell(&self, cell: Cell<TrackField>, cx: &mut App) -> AnyElement {
        let muted = cx.theme().muted_foreground;

        let Some(track) = self.provider.tracks(cx).get(cell.row) else {
            return cells::blank(&cell);
        };

        if cell.field == TrackField::Index {
            return self.index_cell(&cell, track, cx);
        }
        let faded = muted.opacity(0.5);
        let (title, detail) = match track.playable {
            true => (None, muted),
            false => (Some(faded), faded),
        };

        match cell.field {
            TrackField::Cover => cells::artwork(&cell, track.cover.clone()),
            TrackField::Title => cells::title(&cell, track.name.clone(), title, track.explicit),
            TrackField::Artists => self.artist_cell(&cell, track, detail),
            TrackField::Album => self.album_cell(&cell, track, detail),
            TrackField::AddedAt => cells::dim(
                &cell,
                track
                    .added_at
                    .and_then(|seconds| Timestamp::new(seconds, 0).ok())
                    .map(|timestamp| timestamp.strftime("%b %-d, %Y").to_string())
                    .unwrap_or_default(),
                detail,
            ),
            TrackField::Duration => cells::dim(&cell, clock(track.duration), detail),
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
            TrackField::AddedAt => tracks
                .get(a)
                .and_then(|track| track.added_at)
                .cmp(&tracks.get(b).and_then(|track| track.added_at)),
            TrackField::Duration => tracks
                .get(a)
                .map(|track| track.duration)
                .cmp(&tracks.get(b).map(|track| track.duration)),
            TrackField::Index | TrackField::Cover => a.cmp(&b),
        }
    }
}
