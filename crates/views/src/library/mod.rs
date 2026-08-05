mod albums;
mod playlists;
mod toolbar;

use gpui::prelude::*;
use gpui::{App, Context, Entity, Pixels, Render, Window, div, px};
use spotify::Track;
use state::{Library, LibraryState, Playback};
use ui::{GridDelegate, GridEvent, GridState, grid};
use workspace::{Destination, LibraryTab, Navigation, Sidebar};

use crate::cells;
use crate::tracks::{LIBRARY_COLUMNS, TrackSource, Tracks};
use albums::AlbumSource;
use playlists::PlaylistSource;

pub use toolbar::LibraryToolbar;

impl From<LibraryTab> for Section {
    fn from(tab: LibraryTab) -> Self {
        match tab {
            LibraryTab::Songs => Section::Tracks,
            LibraryTab::Albums => Section::Albums,
            LibraryTab::Playlists => Section::Playlists,
        }
    }
}

impl From<Section> for LibraryTab {
    fn from(section: Section) -> Self {
        match section {
            Section::Tracks => LibraryTab::Songs,
            Section::Albums => LibraryTab::Albums,
            Section::Playlists => LibraryTab::Playlists,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Tracks,
    Albums,
    Playlists,
}

impl Section {
    pub fn label(self) -> &'static str {
        match self {
            Section::Tracks => "Songs",
            Section::Albums => "Albums",
            Section::Playlists => "Playlists",
        }
    }
}

pub struct Counts {
    pub tracks: usize,
    pub albums: usize,
    pub playlists: usize,
}

struct LibraryTracks(Entity<Library>);

impl Tracks for LibraryTracks {
    fn tracks<'a>(&self, cx: &'a App) -> &'a [Track] {
        match self.0.read(cx).state() {
            LibraryState::Ready { tracks, .. } => tracks.as_slice(),
            _ => &[],
        }
    }

    fn is_loading(&self, cx: &App) -> bool {
        self.0.read(cx).is_loading()
    }
}

pub struct LibraryView {
    library: Entity<Library>,
    playback: Entity<Playback>,
    navigation: Entity<Navigation>,
    sidebar: Entity<Sidebar>,
    section: Section,
    width: Pixels,
    tracks: Entity<GridState<TrackSource>>,
    albums: Entity<GridState<AlbumSource>>,
    playlists: Entity<GridState<PlaylistSource>>,
}

impl LibraryView {
    pub fn new(
        library: Entity<Library>,
        playback: Entity<Playback>,
        navigation: Entity<Navigation>,
        sidebar: Entity<Sidebar>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let width = cells::content_width(window, sidebar.read(cx).occupied_width(), Pixels::ZERO);

        let tracks = cx.new(|cx| {
            let source = TrackSource::new(
                LIBRARY_COLUMNS,
                LibraryTracks(library.clone()),
                playback.clone(),
                navigation.clone(),
            );
            GridState::new(GridDelegate::new(source, width, cx), window, cx)
        });
        let albums = cx.new(|cx| {
            let source = AlbumSource::new(library.clone(), playback.clone());
            GridState::new(GridDelegate::new(source, width, cx), window, cx)
        });
        let playlists = cx.new(|cx| {
            let source = PlaylistSource::new(library.clone());
            GridState::new(GridDelegate::new(source, width, cx), window, cx)
        });

        cx.observe(&library, |this, _, cx| {
            this.rebuild(cx);
            cx.notify();
        })
        .detach();

        cx.observe(&sidebar, |_, _, cx| cx.notify()).detach();

        cx.observe(&playback, |this, _, cx| {
            this.tracks.update(cx, |table, cx| table.refresh(cx));
        })
        .detach();

        cx.subscribe(&tracks, |this, _, event, cx| {
            let GridEvent::DoubleClicked(display) = event;
            this.play(*display, cx);
        })
        .detach();

        cx.subscribe(&albums, |this, _, event, cx| {
            let GridEvent::DoubleClicked(display) = event;
            this.open_album(*display, cx);
        })
        .detach();

        Self {
            library,
            playback,
            navigation,
            sidebar,
            section: Section::Tracks,
            width,
            tracks,
            albums,
            playlists,
        }
    }

    pub fn section(&self) -> Section {
        self.section
    }

    pub fn counts(&self, cx: &App) -> Counts {
        match self.library.read(cx).state() {
            LibraryState::Ready {
                tracks,
                albums,
                playlists,
                ..
            } => Counts {
                tracks: tracks.len(),
                albums: albums.len(),
                playlists: playlists.len(),
            },
            _ => Counts {
                tracks: 0,
                albums: 0,
                playlists: 0,
            },
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
        cx.notify();
    }

    fn play(&mut self, display: usize, cx: &mut Context<Self>) {
        let queued = {
            let state = self.tracks.read(cx);
            let delegate = state.delegate();
            (0..delegate.row_count())
                .filter_map(|row| delegate.source().at(delegate.row(row), cx))
                .collect::<Vec<_>>()
        };
        self.playback
            .update(cx, |playback, cx| playback.start(queued, display, cx));
    }

    fn open_album(&mut self, display: usize, cx: &mut Context<Self>) {
        let album = {
            let state = self.albums.read(cx);
            let row = state.delegate().row(display);
            state.delegate().source().at(row, cx)
        };
        let Some(album) = album else {
            return;
        };
        let destination = Destination::Album(album.id.into());
        self.navigation
            .update(cx, |navigation, cx| navigation.go(destination, cx));
    }

    fn resize(&mut self, window: &Window, cx: &mut Context<Self>) {
        let sidebar = self.sidebar.read(cx).occupied_width();
        let width = cells::content_width(window, sidebar, Pixels::ZERO);
        if (width - self.width).abs() < px(0.5) {
            return;
        }
        self.width = width;

        self.tracks.update(cx, |table, cx| {
            table.delegate_mut().set_width(width);
            table.refresh(cx);
        });
        self.albums.update(cx, |table, cx| {
            table.delegate_mut().set_width(width);
            table.refresh(cx);
        });
        self.playlists.update(cx, |table, cx| {
            table.delegate_mut().set_width(width);
            table.refresh(cx);
        });
    }

    fn rebuild(&mut self, cx: &mut Context<Self>) {
        self.tracks.update(cx, |table, cx| {
            table.delegate_mut().rebuild(cx);
            table.refresh(cx);
        });
        self.albums.update(cx, |table, cx| {
            table.delegate_mut().rebuild(cx);
            table.refresh(cx);
        });
        self.playlists.update(cx, |table, cx| {
            table.delegate_mut().rebuild(cx);
            table.refresh(cx);
        });
    }
}

impl Render for LibraryView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.resize(window, cx);

        let table = match self.section {
            Section::Tracks => grid(&self.tracks).into_any_element(),
            Section::Albums => grid(&self.albums).into_any_element(),
            Section::Playlists => grid(&self.playlists).into_any_element(),
        };

        div()
            .flex()
            .flex_col()
            .size_full()
            .child(div().flex_1().min_h_0().child(table))
    }
}
