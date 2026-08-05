use std::cmp::Ordering;
use ui::ActiveTheme as _;

use gpui::prelude::*;
use gpui::{AnyElement, App, Entity, Hsla, TextAlign, div, svg};
use spotify::Track;
use state::{Playback, PlaybackState};
use ui::{Cell, ColumnSpec, GridSource, Width, clock};
use workspace::{Destination, Navigation};

use crate::cells::{self, ALWAYS, ARTWORK_COLUMN, GLYPH, NUMBER, ROOMY, SNUG, TRAILING, WIDE};

const PLAY: &str = "icons/play.svg";
const PLAYING: &str = "icons/music-2.svg";
const PAUSE: &str = "icons/pause.svg";
const UNAVAILABLE: &str = "icons/play-off.svg";

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
    playback: Entity<Playback>,
    navigation: Entity<Navigation>,
}

impl TrackSource {
    pub(crate) fn new(
        columns: &'static [ColumnSpec<TrackField>],
        provider: impl Tracks,
        playback: Entity<Playback>,
        navigation: Entity<Navigation>,
    ) -> Self {
        Self {
            columns,
            provider: Box::new(provider),
            playback,
            navigation,
        }
    }

    fn album_cell(&self, cell: &Cell<TrackField>, track: &Track, color: Hsla) -> AnyElement {
        let Some(album) = track.album_id.clone() else {
            return cells::dim(cell, track.album.clone(), color);
        };

        let navigation = self.navigation.clone();
        cells::link(cell, "album", track.album.clone(), color, move |_, cx| {
            let destination = Destination::Album(album.clone().into());
            navigation.update(cx, |navigation, cx| navigation.go(destination, cx));
        })
    }

    fn index_cell(&self, cell: &Cell<TrackField>, track: &Track, cx: &App) -> AnyElement {
        let theme = *cx.theme();
        let state = self.now_playing(track, cx);
        let playing = matches!(state, Some(PlaybackState::Playing));

        let resting = match &state {
            Some(PlaybackState::Playing) => svg()
                .path(PLAYING)
                .size(GLYPH)
                .text_color(theme.foreground)
                .into_any_element(),
            Some(_) => svg()
                .path(PAUSE)
                .size(GLYPH)
                .text_color(theme.muted_foreground)
                .into_any_element(),
            None => {
                let color = match track.playable {
                    true => theme.muted_foreground,
                    false => theme.muted_foreground.opacity(0.5),
                };
                div()
                    .text_color(color)
                    .child(format!("{}", cell.display + 1))
                    .into_any_element()
            }
        };

        let icon = match playing {
            true => PAUSE,
            false => match track.playable {
                true => PLAY,
                false => UNAVAILABLE,
            },
        };
        let color = match track.playable {
            true => theme.foreground,
            false => theme.muted_foreground,
        };

        let press: Option<Box<dyn Fn(&mut App)>> = match track.playable {
            false => None,
            true => {
                let playback = self.playback.clone();
                let queued = self.provider.tracks(cx).to_vec();
                let row = cell.row;
                Some(Box::new(move |cx: &mut App| {
                    playback.update(cx, |playback, cx| match state {
                        Some(PlaybackState::Playing) => playback.pause(cx),
                        Some(PlaybackState::Paused) => playback.resume(cx),
                        _ => playback.start(queued.clone(), row, cx),
                    });
                }))
            }
        };

        cells::transport(cell, resting, cells::Transport { icon, color, press })
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
            TrackField::Title => match title {
                Some(color) => cells::dim(&cell, track.name.clone(), color),
                None => cells::text(&cell, track.name.clone()),
            },
            TrackField::Artists => cells::dim(&cell, track.artists.clone(), detail),
            TrackField::Album => self.album_cell(&cell, track, detail),
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
            TrackField::Duration => tracks
                .get(a)
                .map(|track| track.duration)
                .cmp(&tracks.get(b).map(|track| track.duration)),
            TrackField::Index | TrackField::Cover => a.cmp(&b),
        }
    }
}
