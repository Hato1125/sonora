// SPDX-License-Identifier: GPL-3.0-or-later

mod albums;
mod playlists;

use gpui::prelude::*;
use gpui::{App, Context, Entity, Pixels, Point, Render, ScrollHandle, SharedString, Window, px};
use i18n::t;
use router::{Destination, LibraryTab, navigate};
use spotify::Track;
use state::{AppSettings, Library, LibraryState, Playback, Sonora};
use ui::{
    FlagAxis, GridDelegate, GridEvent, GridState, RangeAxis, Scrollbar, Scroller, Sort, Toggle,
    Unit, Viewport, scrolled,
};
use workspace::{Chrome, Columned, Filterable, Searchable, Toolbar, Tooled};

use crate::tracks::{
    self, LIBRARY_COLUMNS, PlaybackStatus, TrackField, TrackSieve, TrackSource, Tracks,
    playback_status,
};
use crate::{cells, page};
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

const PINNED: [&str; 3] = ["cover", "title", "name"];

impl Section {
    fn key(self) -> &'static str {
        match self {
            Section::Tracks => "songs",
            Section::Albums => "albums",
            Section::Playlists => "playlists",
        }
    }
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
    settings: Entity<AppSettings>,
    playback: Entity<Playback>,
    playback_status: PlaybackStatus,
    section: Section,
    width: Pixels,
    scrollbar: Entity<Scrollbar>,
    tracks: Entity<GridState<TrackSource>>,
    albums: Entity<GridState<AlbumSource>>,
    playlists: Entity<GridState<PlaylistSource>>,
    toolbar: Entity<Toolbar>,
}

impl LibraryView {
    pub fn new(
        library: Entity<Library>,
        playback: Entity<Playback>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let width = cells::content_width(window, Pixels::ZERO, cx);
        let settings = Sonora::global(cx).settings.clone();
        let stored = |section: Section, cx: &App| {
            let settings = settings.read(cx);
            (
                settings.table(section.key()),
                settings.sorting(section.key()),
            )
        };

        let scrollbar = cx.new(|_| Scrollbar::new(ScrollHandle::new()));
        let scroll = scrollbar.read(cx).scroll().clone();

        let tracks = cx.new(|cx| {
            let playlist_scrollbar = cx.new(|_| {
                Scrollbar::new(ScrollHandle::new())
                    .always_visible()
                    .track_inset(px(4.))
            });
            let source = TrackSource::new(
                LIBRARY_COLUMNS,
                LibraryTracks(library.clone()),
                playback.clone(),
                playlist_scrollbar,
            );
            let source = source.table(cx.weak_entity());
            let mut delegate = GridDelegate::new(source, width, cx).with_sort(
                TrackField::AddedAt,
                Sort::Descending,
                cx,
            );
            let (layout, sorting) = stored(Section::Tracks, cx);
            delegate.set_layout(layout, cx);
            if let Some(sorting) = sorting {
                delegate.set_sorting(sorting, cx);
            }
            GridState::new(delegate, cx).follow(scroll.clone())
        });
        let albums = cx.new(|cx| {
            let source = AlbumSource::new(library.clone(), playback.clone());
            let mut delegate = GridDelegate::new(source, width, cx);
            let (layout, sorting) = stored(Section::Albums, cx);
            delegate.set_layout(layout, cx);
            delegate.set_sorting(sorting.flatten(), cx);
            GridState::new(delegate, cx).follow(scroll.clone())
        });
        let playlists = cx.new(|cx| {
            let source = PlaylistSource::new(library.clone(), playback.clone());
            let mut delegate = GridDelegate::new(source, width, cx);
            let (layout, sorting) = stored(Section::Playlists, cx);
            delegate.set_layout(layout, cx);
            delegate.set_sorting(sorting.flatten(), cx);
            GridState::new(delegate, cx).follow(scroll)
        });

        cx.observe(&library, |this, _, cx| {
            this.rebuild(cx);
            cx.notify();
        })
        .detach();

        let chrome = Chrome::entity(cx);
        cx.observe(&chrome, |_, _, cx| cx.notify()).detach();

        let current_playback = playback_status(&playback, cx);
        cx.observe(&playback, |this, playback, cx| {
            let current = playback_status(&playback, cx);
            if this.playback_status == current {
                return;
            }
            this.playback_status = current;
            for table in this.tables() {
                table.refresh(cx);
            }
        })
        .detach();

        cx.subscribe(&tracks, |this, _, event, cx| match event {
            GridEvent::DoubleClicked(display) => this.play(*display, cx),
            _ => this.persist(Section::Tracks, cx),
        })
        .detach();

        cx.subscribe(&albums, |this, _, event, cx| match event {
            GridEvent::DoubleClicked(display) => this.open_album(*display, cx),
            _ => this.persist(Section::Albums, cx),
        })
        .detach();

        cx.subscribe(&playlists, |this, _, event, cx| match event {
            GridEvent::DoubleClicked(display) => this.open_playlist(*display, cx),
            _ => this.persist(Section::Playlists, cx),
        })
        .detach();

        let me = cx.entity();
        let toolbar = cx.new(|cx| {
            let mut toolbar = Toolbar::new(cx);
            toolbar.bind(&me, cx);
            toolbar.columns(&me, cx);
            toolbar.filters(&me, cx);
            toolbar
        });

        Self {
            library,
            settings,
            playback,
            playback_status: current_playback,
            section: Section::Tracks,
            width,
            scrollbar,
            tracks,
            albums,
            playlists,
            toolbar,
        }
    }

    pub fn section(&self) -> Section {
        self.section
    }

    fn table(&self, section: Section) -> &dyn ui::Table {
        match section {
            Section::Tracks => &self.tracks,
            Section::Albums => &self.albums,
            Section::Playlists => &self.playlists,
        }
    }

    fn tables(&self) -> [&dyn ui::Table; 3] {
        [&self.tracks, &self.albums, &self.playlists]
    }

    fn column_toggles(&self, cx: &App) -> Vec<Toggle> {
        self.table(self.section)
            .toggles(cx)
            .into_iter()
            .filter(|toggle| !PINNED.contains(&toggle.key))
            .collect()
    }

    fn switch_column(&mut self, key: &str, cx: &mut Context<Self>) {
        if PINNED.contains(&key) {
            return;
        }

        let mut layout = self.table(self.section).layout(cx);
        layout.toggle(key);
        self.table(self.section).set_layout(layout, cx);
        self.persist(self.section, cx);
        cx.notify();
    }

    fn persist(&mut self, section: Section, cx: &mut Context<Self>) {
        page::store(
            &self.settings.clone(),
            self.table(section),
            section.key(),
            section.key(),
            cx,
        );
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
        let queued = tracks::ordered(&self.tracks, cx);
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
        let width = cells::content_width(window, Pixels::ZERO, cx);
        if (width - self.width).abs() < px(0.5) {
            return;
        }
        self.width = width;

        for table in self.tables() {
            table.set_width(width, cx);
        }
    }

    fn rebuild(&mut self, cx: &mut Context<Self>) {
        for table in self.tables() {
            table.rebuild(cx);
        }
    }
}

impl Render for LibraryView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.resize(window, cx);

        let scroll = self.scrollbar.read(cx).scroll().clone();
        let viewport = Self::viewport(&scroll, window);
        let table = self.table(self.section);
        table.set_viewport(viewport, cx);

        Scroller::new("library-page", &self.scrollbar).child(table.element())
    }
}

impl Searchable for LibraryView {
    fn search(&mut self, query: &str, cx: &mut Context<Self>) {
        for table in self.tables() {
            table.set_filter(query, cx);
        }
        cx.notify();
    }

    fn hint() -> SharedString {
        "filter-library".into()
    }
}

impl Columned for LibraryView {
    fn toggles(&self, cx: &App) -> Vec<Toggle> {
        self.column_toggles(cx)
    }

    fn toggle_column(&mut self, key: &'static str, cx: &mut Context<Self>) {
        self.switch_column(key, cx);
    }
}

impl Tooled for LibraryView {
    fn toolbar(&self) -> Entity<Toolbar> {
        self.toolbar.clone()
    }
}

impl LibraryView {
    fn sieve(&self, cx: &App) -> TrackSieve {
        self.tracks.read(cx).delegate().source().sieve()
    }

    fn sift(&mut self, sieve: TrackSieve, cx: &mut Context<Self>) {
        self.tracks.update(cx, |table, cx| {
            table.delegate_mut().source_mut().set_sieve(sieve);
            table.delegate_mut().resift(cx);
            table.refresh(cx);
        });
        cx.notify();
    }

    fn span(&self, cx: &App) -> Option<(f32, f32)> {
        self.albums.read(cx).delegate().source().span()
    }

    fn set_span(&mut self, span: Option<(f32, f32)>, cx: &mut Context<Self>) {
        self.albums.update(cx, |table, cx| {
            table.delegate_mut().source_mut().set_span(span);
            table.delegate_mut().resift(cx);
            table.refresh(cx);
        });
        cx.notify();
    }
}

impl Filterable for LibraryView {
    fn ranges(&self, cx: &App) -> Vec<RangeAxis> {
        match self.section {
            Section::Tracks => {
                let table = self.tracks.read(cx);
                let Some(bounds) = table
                    .delegate()
                    .source()
                    .extent(table.delegate().query(), cx)
                else {
                    return Vec::new();
                };
                let value = self.sieve(cx).duration.unwrap_or(bounds);
                vec![
                    RangeAxis {
                        key: "filter-duration",
                        label: t!("filter-duration"),
                        bounds,
                        value,
                        unit: Unit::Clock,
                        values: None,
                    }
                    .clamped(),
                ]
            }
            Section::Albums => {
                let table = self.albums.read(cx);
                let years = table
                    .delegate()
                    .source()
                    .years(table.delegate().query(), cx);
                let (Some(first), Some(last)) = (years.first(), years.last()) else {
                    return Vec::new();
                };
                let bounds = (*first, *last);
                let value = self.span(cx).unwrap_or(bounds);
                vec![
                    RangeAxis {
                        key: "filter-year",
                        label: t!("filter-year"),
                        bounds,
                        value,
                        unit: Unit::Plain,
                        values: Some(years),
                    }
                    .clamped(),
                ]
            }
            Section::Playlists => Vec::new(),
        }
    }

    fn flags(&self, cx: &App) -> Vec<FlagAxis> {
        if self.section != Section::Tracks {
            return Vec::new();
        }

        let sieve = self.sieve(cx);
        vec![
            FlagAxis {
                key: "filter-explicit",
                label: t!("filter-explicit"),
                on: sieve.explicit,
            },
            FlagAxis {
                key: "filter-playable",
                label: t!("filter-playable"),
                on: sieve.playable,
            },
        ]
    }

    fn set_range(&mut self, key: &'static str, value: (f32, f32), cx: &mut Context<Self>) {
        match key {
            "filter-year" => self.set_span(Some(value), cx),
            _ => {
                let mut sieve = self.sieve(cx);
                sieve.duration = Some(value);
                self.sift(sieve, cx);
            }
        }
    }

    fn set_flag(&mut self, key: &'static str, on: bool, cx: &mut Context<Self>) {
        let mut sieve = self.sieve(cx);
        match key {
            "filter-explicit" => sieve.explicit = on,
            "filter-playable" => sieve.playable = on,
            _ => return,
        }
        self.sift(sieve, cx);
    }

    fn reset_filters(&mut self, cx: &mut Context<Self>) {
        self.sift(TrackSieve::default(), cx);
        self.set_span(None, cx);
    }
}
