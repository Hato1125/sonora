mod albums;
mod playlists;

use gpui::prelude::*;
use gpui::{App, Context, Entity, Pixels, Point, Render, ScrollHandle, Window, div, px};
use router::{Destination, LibraryTab, navigate};
use spotify::Track;
use state::{Library, LibraryState, Playback};
use ui::{GridDelegate, GridEvent, GridState, Scrollbar, Viewport, grid, scrolled};
use workspace::{Searchable, Sidebar};

use crate::cells;
use crate::tracks::{LIBRARY_COLUMNS, TrackSource, Tracks};
use albums::AlbumSource;
use playlists::PlaylistSource;

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
    sidebar: Entity<Sidebar>,
    section: Section,
    width: Pixels,
    scrollbar: Entity<Scrollbar>,
    tracks: Entity<GridState<TrackSource>>,
    albums: Entity<GridState<AlbumSource>>,
    playlists: Entity<GridState<PlaylistSource>>,
}

impl LibraryView {
    pub fn new(
        library: Entity<Library>,
        playback: Entity<Playback>,
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
            );
            GridState::new(GridDelegate::new(source, width, cx))
        });
        let albums = cx.new(|cx| {
            let source = AlbumSource::new(library.clone(), playback.clone());
            GridState::new(GridDelegate::new(source, width, cx))
        });
        let playlists = cx.new(|cx| {
            let source = PlaylistSource::new(library.clone(), playback.clone());
            GridState::new(GridDelegate::new(source, width, cx))
        });

        cx.observe(&library, |this, _, cx| {
            this.rebuild(cx);
            cx.notify();
        })
        .detach();

        cx.observe(&sidebar, |_, _, cx| cx.notify()).detach();

        cx.observe(&playback, |this, _, cx| {
            this.tracks.update(cx, |table, cx| table.refresh(cx));
            this.albums.update(cx, |table, cx| table.refresh(cx));
            this.playlists.update(cx, |table, cx| table.refresh(cx));
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

        cx.subscribe(&playlists, |this, _, event, cx| {
            let GridEvent::DoubleClicked(display) = event;
            this.open_playlist(*display, cx);
        })
        .detach();

        let scrollbar = cx.new(|_| Scrollbar::new(ScrollHandle::new()));

        Self {
            library,
            playback,
            sidebar,
            section: Section::Tracks,
            width,
            scrollbar,
            tracks,
            albums,
            playlists,
        }
    }

    pub fn section(&self) -> Section {
        self.section
    }

    pub fn is_loading(&self, cx: &App) -> bool {
        self.library.read(cx).is_loading()
    }

    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.library.update(cx, |library, cx| library.refresh(cx));
    }

    pub fn select(&mut self, section: Section, cx: &mut Context<Self>) {
        if self.section != section {
            self.scrollbar
                .read(cx)
                .scroll()
                .set_offset(Point::default());
        }
        self.section = section;
        cx.notify();
    }

    fn viewport(scroll: &ScrollHandle, window: &Window) -> Viewport {
        let visible = scroll.bounds().size.height;

        Viewport {
            top: scrolled(scroll),
            height: match visible > Pixels::ZERO {
                true => visible,
                false => window.viewport_size().height,
            },
        }
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
        navigate(Destination::Album(album.id.into()), cx);
    }

    fn open_playlist(&mut self, display: usize, cx: &mut Context<Self>) {
        let playlist = {
            let state = self.playlists.read(cx);
            let row = state.delegate().row(display);
            state.delegate().source().at(row, cx)
        };
        let Some(playlist) = playlist else {
            return;
        };
        navigate(Destination::Playlist(playlist.id.into()), cx);
    }

    fn resize(&mut self, window: &Window, cx: &mut Context<Self>) {
        let sidebar = self.sidebar.read(cx).occupied_width();
        let width = cells::content_width(window, sidebar, Pixels::ZERO);
        if (width - self.width).abs() < px(0.5) {
            return;
        }
        self.width = width;

        self.tracks.update(cx, |table, cx| {
            table.delegate_mut().set_width(width, cx);
            table.refresh(cx);
        });
        self.albums.update(cx, |table, cx| {
            table.delegate_mut().set_width(width, cx);
            table.refresh(cx);
        });
        self.playlists.update(cx, |table, cx| {
            table.delegate_mut().set_width(width, cx);
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

        let scroll = self.scrollbar.read(cx).scroll().clone();
        let viewport = Self::viewport(&scroll, window);

        let table = match self.section {
            Section::Tracks => {
                self.tracks
                    .update(cx, |table, _| table.set_viewport(viewport));
                grid(&self.tracks).into_any_element()
            }
            Section::Albums => {
                self.albums
                    .update(cx, |table, _| table.set_viewport(viewport));
                grid(&self.albums).into_any_element()
            }
            Section::Playlists => {
                self.playlists
                    .update(cx, |table, _| table.set_viewport(viewport));
                grid(&self.playlists).into_any_element()
            }
        };

        div()
            .relative()
            .size_full()
            .child(
                div()
                    .id("library-page")
                    .size_full()
                    .overflow_y_scroll()
                    .track_scroll(&scroll)
                    .child(table),
            )
            .child(self.scrollbar.clone())
    }
}

impl Searchable for LibraryView {
    fn search(&mut self, query: &str, cx: &mut Context<Self>) {
        self.tracks.update(cx, |table, cx| {
            table.delegate_mut().set_filter(query, cx);
            table.refresh(cx);
        });
        self.albums.update(cx, |table, cx| {
            table.delegate_mut().set_filter(query, cx);
            table.refresh(cx);
        });
        self.playlists.update(cx, |table, cx| {
            table.delegate_mut().set_filter(query, cx);
            table.refresh(cx);
        });
        cx.notify();
    }

    fn hint() -> &'static str {
        "Filter your library"
    }
}
