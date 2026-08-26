use std::cmp::Ordering;
use ui::ActiveTheme as _;

use gpui::{AnyElement, App, Entity, TextAlign};
use music::Playlist;
use router::Destination;
use state::{Library, LibraryState, Origin, Playback};
use ui::rank::{ESSENTIAL, HANDY, NICE, SPARE};
use ui::{Cell, ColumnSpec, GridSource, Menu, Pin, Width};

use crate::shared::cells::{self, DATE, NUMBER, TRAILING};
use crate::shared::menu::playlist_menu;
use crate::shared::pins::Pinned as _;
use crate::shared::text::{folded, holds};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum PlaylistField {
    Index,
    Cover,
    Name,
    Owner,
    TrackCount,
    Modified,
}

const COLUMN: ColumnSpec<PlaylistField> = ColumnSpec::filling(PlaylistField::Index);

const INDEX: ColumnSpec<PlaylistField> = ColumnSpec::numbering(PlaylistField::Index, NUMBER);

const COVER: ColumnSpec<PlaylistField> = ColumnSpec::artwork(PlaylistField::Cover);

const NAME: ColumnSpec<PlaylistField> = ColumnSpec {
    field: PlaylistField::Name,
    key: "name",
    header: "column-name",
    width: Width::Fill(0.55),
    rank: ESSENTIAL,
    ..COLUMN
};

const OWNER: ColumnSpec<PlaylistField> = ColumnSpec {
    field: PlaylistField::Owner,
    key: "owner",
    header: "column-owner",
    width: Width::Fill(0.45),
    rank: NICE,
    ..COLUMN
};

const TRACK_COUNT: ColumnSpec<PlaylistField> = ColumnSpec {
    field: PlaylistField::TrackCount,
    key: "tracks",
    header: "column-tracks",
    align: TextAlign::Right,
    width: Width::Fixed(TRAILING),
    rank: SPARE,
    ..COLUMN
};

const MODIFIED: ColumnSpec<PlaylistField> = ColumnSpec {
    field: PlaylistField::Modified,
    key: "modified",
    header: "column-modified",
    width: Width::Fixed(DATE),
    rank: HANDY,
    ..COLUMN
};

pub(super) const COLUMNS: &[ColumnSpec<PlaylistField>] =
    &[INDEX, COVER, NAME, OWNER, MODIFIED, TRACK_COUNT];

pub(super) struct PlaylistSource {
    library: Entity<Library>,
    playback: Entity<Playback>,
}

impl PlaylistSource {
    pub(super) fn new(library: Entity<Library>, playback: Entity<Playback>) -> Self {
        Self { library, playback }
    }

    fn index_cell(&self, cell: &Cell<PlaylistField>, playlist: &Playlist, cx: &App) -> AnyElement {
        let origin = Origin::Playlist(playlist.id.clone());
        let state = self.playback.read(cx).playing_from(&origin);
        let id = playlist.id.clone();
        let press = cells::toggle(&self.playback, state.clone(), move |playback, cx| {
            playback.play_playlist(&id, cx)
        });

        cells::index(cell, state, true, None, press, cx)
    }

    pub(super) fn at(&self, row: usize, cx: &App) -> Option<Playlist> {
        self.playlists(cx).get(row).cloned()
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

    fn matches(&self, row: usize, query: &str, cx: &App) -> bool {
        self.playlists(cx)
            .get(row)
            .is_some_and(|playlist| holds(&playlist.name, query) || holds(&playlist.owner, query))
    }

    fn playing(&self, row: usize, cx: &App) -> bool {
        self.playlists(cx).get(row).is_some_and(|playlist| {
            let origin = Origin::Playlist(playlist.id.clone());
            self.playback.read(cx).playing_from(&origin).is_some()
        })
    }

    fn is_loading(&self, cx: &App) -> bool {
        self.library.read(cx).is_loading()
    }

    fn pin(&self, row: usize, cx: &App) -> Option<Pin> {
        self.playlists(cx).get(row)?.pin()
    }

    fn context_menu(&self, row: usize, _visible: &[PlaylistField], cx: &App) -> Option<Menu> {
        Some(playlist_menu(
            self.at(row, cx)?,
            self.playback.clone(),
            false,
            cx,
        ))
    }

    fn cell(&self, cell: Cell<PlaylistField>, cx: &mut App) -> AnyElement {
        let theme = *cx.theme();
        let muted = theme.muted_foreground;

        let Some(playlist) = self.playlists(cx).get(cell.row) else {
            return cells::blank(&cell);
        };

        if cell.field == PlaylistField::Index {
            return self.index_cell(&cell, playlist, cx);
        }

        match cell.field {
            PlaylistField::Cover => cells::artwork(&cell, playlist.cover.clone()),
            PlaylistField::Name => cells::link(
                &cell,
                "playlist-name",
                playlist.name.clone(),
                theme.foreground,
                Destination::Playlist(playlist.id.clone().into()),
            ),
            PlaylistField::Owner => cells::dim(&cell, playlist.owner.clone(), muted),
            PlaylistField::TrackCount => {
                cells::dim(&cell, format!("{}", playlist.track_count), muted)
            }
            PlaylistField::Modified => cells::dim(&cell, cells::stamp(playlist.modified_at), muted),
            PlaylistField::Index => cells::blank(&cell),
        }
    }

    fn compare(&self, field: PlaylistField, a: usize, b: usize, cx: &App) -> Ordering {
        let playlists = self.playlists(cx);
        let text = |index: usize, pick: fn(&Playlist) -> &str| {
            playlists.get(index).map(pick).unwrap_or_default()
        };

        match field {
            PlaylistField::Name => folded(
                text(a, |playlist| &playlist.name),
                text(b, |playlist| &playlist.name),
            ),
            PlaylistField::Owner => folded(
                text(a, |playlist| &playlist.owner),
                text(b, |playlist| &playlist.owner),
            ),
            PlaylistField::TrackCount => playlists
                .get(a)
                .map(|playlist| playlist.track_count)
                .cmp(&playlists.get(b).map(|playlist| playlist.track_count)),
            PlaylistField::Modified => playlists
                .get(a)
                .map(|playlist| playlist.modified_at)
                .cmp(&playlists.get(b).map(|playlist| playlist.modified_at)),
            PlaylistField::Index | PlaylistField::Cover => a.cmp(&b),
        }
    }
}
