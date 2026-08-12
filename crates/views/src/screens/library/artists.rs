use std::cmp::Ordering;
use ui::ActiveTheme as _;

use gpui::{AnyElement, App, Entity, SharedString, TextAlign};
use music::SavedArtist;
use router::Destination;
use state::{Library, LibraryState, Origin, Playback};
use ui::rank::{ESSENTIAL, HANDY};
use ui::{Cell, ColumnSpec, GridSource, Menu, Pin, PinKind, Width};

use crate::shared::cells::{self, DATE, NUMBER};
use crate::shared::menu::artist_menu;
use crate::shared::tracks::initial;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ArtistField {
    Index,
    Cover,
    Name,
    AddedAt,
}

const COLUMN: ColumnSpec<ArtistField> = ColumnSpec {
    field: ArtistField::Index,
    key: "",
    header: "",
    align: TextAlign::Left,
    width: Width::Fill(1.),
    anchored: false,
    sortable: true,
    rank: ESSENTIAL,
};

const INDEX: ColumnSpec<ArtistField> = ColumnSpec {
    field: ArtistField::Index,
    key: "index",
    header: "column-index",
    align: TextAlign::Center,
    width: Width::Fixed(NUMBER),
    anchored: true,
    sortable: false,
    ..COLUMN
};

const COVER: ColumnSpec<ArtistField> = ColumnSpec {
    field: ArtistField::Cover,
    key: "cover",
    width: Width::Thumb,
    anchored: true,
    sortable: false,
    ..COLUMN
};

const NAME: ColumnSpec<ArtistField> = ColumnSpec {
    field: ArtistField::Name,
    key: "name",
    header: "column-name",
    rank: ESSENTIAL,
    ..COLUMN
};

const ADDED_AT: ColumnSpec<ArtistField> = ColumnSpec {
    field: ArtistField::AddedAt,
    key: "added-at",
    header: "column-date-added",
    width: Width::Fixed(DATE),
    rank: HANDY,
    ..COLUMN
};

pub(super) const COLUMNS: &[ColumnSpec<ArtistField>] = &[INDEX, COVER, NAME, ADDED_AT];

pub(super) struct ArtistSource {
    library: Entity<Library>,
    playback: Entity<Playback>,
}

impl ArtistSource {
    pub(super) fn new(library: Entity<Library>, playback: Entity<Playback>) -> Self {
        Self { library, playback }
    }

    fn index_cell(&self, cell: &Cell<ArtistField>, artist: &SavedArtist, cx: &App) -> AnyElement {
        let origin = Origin::Artist(artist.id.clone());
        let state = self.playback.read(cx).playing_from(&origin);
        let id = artist.id.clone();
        let press = cells::toggle(&self.playback, state.clone(), move |playback, cx| {
            playback.play_artist(&id, cx)
        });

        cells::index(cell, state, true, None, press, cx)
    }

    pub(super) fn at(&self, row: usize, cx: &App) -> Option<SavedArtist> {
        self.artists(cx).get(row).cloned()
    }

    fn artists<'a>(&self, cx: &'a App) -> &'a [SavedArtist] {
        match self.library.read(cx).state() {
            LibraryState::Ready { artists, .. } => artists.as_slice(),
            _ => &[],
        }
    }
}

impl GridSource for ArtistSource {
    type Field = ArtistField;

    fn columns(&self) -> &'static [ColumnSpec<ArtistField>] {
        COLUMNS
    }

    fn rows(&self, cx: &App) -> usize {
        self.artists(cx).len()
    }

    fn matches(&self, row: usize, query: &str, cx: &App) -> bool {
        self.at(row, cx)
            .is_some_and(|artist| artist.name.to_lowercase().contains(query))
    }

    fn playing(&self, row: usize, cx: &App) -> bool {
        self.artists(cx).get(row).is_some_and(|artist| {
            let origin = Origin::Artist(artist.id.clone());
            self.playback.read(cx).playing_from(&origin).is_some()
        })
    }

    fn is_loading(&self, cx: &App) -> bool {
        self.library.read(cx).is_loading()
    }

    fn pin(&self, row: usize, cx: &App) -> Option<Pin> {
        let artist = self.at(row, cx)?;

        Some(Pin::new(PinKind::Artist, artist.id, artist.name).cover(artist.cover))
    }

    fn context_menu(&self, row: usize, _visible: &[ArtistField], cx: &App) -> Option<Menu> {
        Some(artist_menu(self.at(row, cx)?.id))
    }

    fn cell(&self, cell: Cell<ArtistField>, cx: &mut App) -> AnyElement {
        let theme = *cx.theme();
        let muted = theme.muted_foreground;

        let Some(artist) = self.artists(cx).get(cell.row) else {
            return cells::blank(&cell);
        };

        if cell.field == ArtistField::Index {
            return self.index_cell(&cell, artist, cx);
        }

        match cell.field {
            ArtistField::Cover => cells::avatar(&cell, artist.cover.clone()),
            ArtistField::Name => cells::link(
                &cell,
                "artist-name",
                artist.name.clone(),
                theme.foreground,
                Destination::Artist(artist.id.clone().into()),
            ),
            ArtistField::AddedAt => cells::dim(&cell, cells::stamp(artist.added_at), muted),
            ArtistField::Index => cells::blank(&cell),
        }
    }

    fn compare(&self, field: ArtistField, a: usize, b: usize, cx: &App) -> Ordering {
        let artists = self.artists(cx);
        let at = |index: usize| artists.get(index);

        match field {
            ArtistField::Name => at(a)
                .map(|artist| artist.name.to_lowercase())
                .cmp(&at(b).map(|artist| artist.name.to_lowercase())),
            ArtistField::AddedAt => at(a)
                .map(|artist| artist.added_at)
                .cmp(&at(b).map(|artist| artist.added_at)),
            ArtistField::Index | ArtistField::Cover => a.cmp(&b),
        }
    }

    fn group(&self, field: ArtistField, row: usize, cx: &App) -> Option<SharedString> {
        let artist = self.artists(cx).get(row)?;

        match field {
            ArtistField::Name => Some(initial(&artist.name)),
            _ => None,
        }
    }
}
