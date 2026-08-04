use std::time::Duration;

use gpui::{
    AnyElement, Div, Edges, Hsla, IntoElement, ParentElement, Pixels, SharedString, SharedUri,
    Styled, StyledImage as _, TextAlign, div, img, px,
};
use gpui_component::Icon;
use gpui_component::skeleton::Skeleton;
use gpui_component::table::{Column as TableColumn, ColumnSort};
use spotify::{Playlist, Track};
use std::cmp::Ordering;

pub(super) const CELL_PADDING: Pixels = px(8.);
const ROUNDED: Pixels = px(4.);

pub(super) fn cover_art(url: Option<&str>, muted: Hsla) -> AnyElement {
    let Some(url) = url else {
        return blank(muted).into_any_element();
    };

    img(SharedUri::from(url.to_owned()))
        .size(ARTWORK)
        .rounded(ROUNDED)
        .with_loading(|| {
            Skeleton::new()
                .size(ARTWORK)
                .rounded(ROUNDED)
                .into_any_element()
        })
        .with_fallback(move || blank(muted).into_any_element())
        .into_any_element()
}

fn blank(muted: Hsla) -> Div {
    div()
        .size(ARTWORK)
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

impl Centered for TableColumn {
    fn text_center(mut self) -> Self {
        self.align = TextAlign::Center;
        self
    }
}

fn flush_paddings() -> Edges<Pixels> {
    Edges {
        top: px(0.),
        bottom: px(0.),
        left: CELL_PADDING,
        right: CELL_PADDING,
    }
}

pub(super) fn cell(width: Pixels, align: TextAlign) -> Div {
    let cell = div().w(width);
    match align {
        TextAlign::Left => cell.truncate(),
        TextAlign::Center => cell.flex().justify_center(),
        TextAlign::Right => cell.flex().justify_end(),
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Field {
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
pub(super) enum Width {
    Fixed(Pixels),
    Fill(f32),
}

pub(super) struct Column {
    pub(super) field: Field,
    key: &'static str,
    header: &'static str,
    align: TextAlign,
    width: Width,
    flush: bool,
    sortable: bool,
}

const NUMBER: Pixels = px(44.);
const TRAILING: Pixels = px(72.);
const ARTWORK: Pixels = px(28.);
const ARTWORK_COLUMN: Pixels = px(28. + 8. * 2.);

pub(super) const TRACK_COLUMNS: &[Column] = &[
    Column {
        field: Field::Index,
        key: "index",
        header: "#",
        align: TextAlign::Center,
        width: Width::Fixed(NUMBER),
        flush: false,
        sortable: false,
    },
    Column {
        field: Field::Cover,
        key: "cover",
        header: "",
        align: TextAlign::Left,
        width: Width::Fixed(ARTWORK_COLUMN),
        flush: true,
        sortable: false,
    },
    Column {
        field: Field::Title,
        key: "title",
        header: "Title",
        align: TextAlign::Left,
        width: Width::Fill(0.42),
        flush: false,
        sortable: true,
    },
    Column {
        field: Field::Artists,
        key: "artists",
        header: "Artist",
        align: TextAlign::Left,
        width: Width::Fill(0.29),
        flush: false,
        sortable: true,
    },
    Column {
        field: Field::Album,
        key: "album",
        header: "Album",
        align: TextAlign::Left,
        width: Width::Fill(0.29),
        flush: false,
        sortable: true,
    },
    Column {
        field: Field::Duration,
        key: "duration",
        header: "Length",
        align: TextAlign::Right,
        width: Width::Fixed(TRAILING),
        flush: false,
        sortable: true,
    },
];

pub(super) const PLAYLIST_COLUMNS: &[Column] = &[
    Column {
        field: Field::Index,
        key: "index",
        header: "#",
        align: TextAlign::Center,
        width: Width::Fixed(NUMBER),
        flush: false,
        sortable: false,
    },
    Column {
        field: Field::Cover,
        key: "cover",
        header: "",
        align: TextAlign::Left,
        width: Width::Fixed(ARTWORK_COLUMN),
        flush: true,
        sortable: false,
    },
    Column {
        field: Field::Name,
        key: "name",
        header: "Name",
        align: TextAlign::Left,
        width: Width::Fill(0.55),
        flush: false,
        sortable: true,
    },
    Column {
        field: Field::Owner,
        key: "owner",
        header: "Owner",
        align: TextAlign::Left,
        width: Width::Fill(0.45),
        flush: false,
        sortable: true,
    },
    Column {
        field: Field::TrackCount,
        key: "tracks",
        header: "Tracks",
        align: TextAlign::Right,
        width: Width::Fixed(TRAILING),
        flush: false,
        sortable: true,
    },
];

impl Column {
    pub(super) fn is_fill(&self) -> bool {
        matches!(self.width, Width::Fill(_))
    }

    pub(super) fn resolve(&self, flexible: Pixels) -> Pixels {
        match self.width {
            Width::Fixed(width) => width,
            Width::Fill(share) => flexible * share,
        }
    }

    pub(super) fn build(&self, flexible: Pixels, sort: Option<(Field, ColumnSort)>) -> TableColumn {
        let column = TableColumn::new(self.key, self.header)
            .width(self.resolve(flexible))
            .resizable(false)
            .movable(false);

        let column = if self.flush {
            column.paddings(flush_paddings())
        } else {
            column
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
    pub(super) fn compare(
        self,
        a: usize,
        b: usize,
        tracks: &[Track],
        playlists: &[Playlist],
    ) -> Ordering {
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

    pub(super) fn text(
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
fn format_duration(duration: Duration) -> String {
    let total = duration.as_secs();
    format!("{}:{:02}", total / 60, total % 60)
}
