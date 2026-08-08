// SPDX-License-Identifier: GPL-3.0-or-later

mod albums;
mod playlists;

use crate::chrome::tools::{self, Sift, Sliders};
use crate::chrome::{Chrome, Searchable, Toolbar, Tooled};
use gpui::prelude::*;
use gpui::{
    AnyElement, App, Context, Entity, FontWeight, Pixels, Point, Render, ScrollHandle,
    SharedString, WeakEntity, Window, div, px,
};
use i18n::t;
use router::{Destination, LibraryTab, navigate};
use spotify::Track;
use state::{AppSettings, Library, LibraryState, Origin, Playback, PlaybackState, Sonora};
use ui::{
    ActiveTheme as _, Card, FlagAxis, GridDelegate, GridEvent, GridSource, GridState, Mode,
    Popovers, RangeAxis, Scrollbar, Scroller, Sort, SortAxis, Text, Toggle, Unit, Viewport,
    heading, scrolled,
};

use crate::shared::release_card::ReleaseCard;
use crate::shared::tracks::{
    self, LIBRARY_COLUMNS, PlaybackStatus, TrackField, TrackSieve, TrackSource, Tracks,
    playback_status,
};
use crate::shared::{cells, page};
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
    const ALL: [Self; 3] = [Self::Tracks, Self::Albums, Self::Playlists];

    fn key(self) -> &'static str {
        match self {
            Section::Tracks => "songs",
            Section::Albums => "albums",
            Section::Playlists => "playlists",
        }
    }

    fn slot(self) -> usize {
        match self {
            Section::Tracks => 0,
            Section::Albums => 1,
            Section::Playlists => 2,
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
    views: [Mode; 3],
    width: Pixels,
    scrollbar: Entity<Scrollbar>,
    tracks: Entity<GridState<TrackSource>>,
    albums: Entity<GridState<AlbumSource>>,
    playlists: Entity<GridState<PlaylistSource>>,
    toolbar: Entity<Toolbar>,
    popovers: Popovers,
    sliders: [Sliders; 3],
    me: WeakEntity<Self>,
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
        let viewed = |section: Section, cx: &App| settings.read(cx).view(section.key());
        let views = Section::ALL.map(|section| viewed(section, cx));

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
            toolbar.wire(&me, cx);
            toolbar
        });

        Self {
            library,
            settings,
            playback,
            playback_status: current_playback,
            section: Section::Tracks,
            views,
            width,
            scrollbar,
            tracks,
            albums,
            playlists,
            toolbar,
            popovers: Popovers::default(),
            sliders: Section::ALL.map(|_| Sliders::default()),
            me: me.downgrade(),
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

    fn cards(&self, cx: &App) -> AnyElement {
        let tiles = match self.section {
            Section::Tracks => deck(&self.tracks, cx, |display, row| {
                self.track_card(display, row, cx)
            }),
            Section::Albums => deck(&self.albums, cx, |display, row| {
                self.album_card(display, row, cx)
            }),
            Section::Playlists => deck(&self.playlists, cx, |display, row| {
                self.playlist_card(display, row, cx)
            }),
        };

        div()
            .flex()
            .flex_wrap()
            .gap_x_8()
            .gap_y_6()
            .px_8()
            .children(tiles)
            .into_any_element()
    }

    fn track_card(&self, display: usize, row: usize, cx: &App) -> Option<AnyElement> {
        let theme = *cx.theme();
        let track = self.tracks.read(cx).delegate().source().at(row, cx)?;
        let playable = track.playable;
        let pressed = (self.tracks.clone(), self.playback.clone());
        let played = pressed.clone();
        let current = self.playback.read(cx);
        let state = current
            .track()
            .filter(|playing| playing.id.is_some() && playing.id == track.id)
            .map(|_| current.state().clone());
        let playing = matches!(state, Some(PlaybackState::Playing));
        let artists = cells::artist_links(
            SharedString::from(format!("library-track-artist-{display}")),
            track.artist_refs.clone(),
            track.artists.clone(),
            theme.muted_foreground,
        )
        .text_size(theme.text(Text::Small))
        .truncate();

        Some(
            Card::new(("library-track", display), SharedString::from(track.name))
                .tile(theme.metrics.cover)
                .cover(track.cover)
                .weight(FontWeight::SEMIBOLD)
                .flat()
                .underline()
                .when(track.explicit, Card::explicit)
                .bare_meta(artists)
                .when(playable, move |card| {
                    card.play(playing, move |_, _, cx| match &state {
                        Some(PlaybackState::Playing) => {
                            played.1.update(cx, |playback, cx| playback.pause(cx))
                        }
                        Some(PlaybackState::Paused) => {
                            played.1.update(cx, |playback, cx| playback.resume(cx))
                        }
                        _ => page::play(&played.0, &played.1, display, cx),
                    })
                    .press(move |_, _, cx| page::play(&pressed.0, &pressed.1, display, cx))
                })
                .into_any_element(),
        )
    }

    fn album_card(&self, display: usize, row: usize, cx: &App) -> Option<AnyElement> {
        let album = self.albums.read(cx).delegate().source().at(row, cx)?;

        Some(ReleaseCard::new(display, album, self.playback.clone()).into_any_element())
    }

    fn playlist_card(&self, display: usize, row: usize, cx: &App) -> Option<AnyElement> {
        let theme = *cx.theme();
        let playlist = self.playlists.read(cx).delegate().source().at(row, cx)?;
        let playback = self.playback.clone();
        let origin = Origin::Playlist(playlist.id.clone());
        let state = self.playback.read(cx).playing_from(&origin);
        let playing = matches!(state, Some(PlaybackState::Playing));
        let played = playlist.id.clone();
        let opened = SharedString::from(playlist.id);

        Some(
            Card::new(
                ("library-playlist", display),
                SharedString::from(playlist.name),
            )
            .tile(theme.metrics.cover)
            .cover(playlist.cover)
            .weight(FontWeight::SEMIBOLD)
            .flat()
            .underline()
            .meta(SharedString::from(playlist.owner))
            .play(playing, move |_, _, cx| {
                playback.update(cx, |playback, cx| match &state {
                    Some(PlaybackState::Playing) => playback.pause(cx),
                    Some(PlaybackState::Paused) => playback.resume(cx),
                    _ => playback.play_playlist(&played, cx),
                });
            })
            .press(move |_, _, cx| navigate(Destination::Playlist(opened.clone()), cx))
            .into_any_element(),
        )
    }
}

impl Render for LibraryView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.resize(window, cx);

        let scroll = self.scrollbar.read(cx).scroll().clone();
        let viewport = Self::viewport(&scroll, window);
        let table = self.table(self.section);
        table.set_viewport(viewport, cx);

        Scroller::new("library-page", &self.scrollbar).child(match self.mode() {
            Mode::List => table.element(),
            Mode::Cards => self.cards(cx),
        })
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

impl LibraryView {
    fn sorts(&self, cx: &App) -> Vec<SortAxis> {
        self.table(self.section).sortables(cx)
    }

    fn set_sort(&mut self, key: &'static str, cx: &mut Context<Self>) {
        self.table(self.section).cycle_sort(key, cx);
        cx.notify();
    }

    fn mode(&self) -> Mode {
        self.views[self.section.slot()]
    }

    fn set_mode(&mut self, mode: Mode, cx: &mut Context<Self>) {
        let section = self.section;
        self.views[section.slot()] = mode;

        let settings = self.settings.clone();
        settings.update(cx, |settings, cx| {
            settings.set_view(section.key(), mode, cx)
        });
        cx.notify();
    }
}

impl Tooled for LibraryView {
    fn toolbar(&self) -> Entity<Toolbar> {
        self.toolbar.clone()
    }

    fn tools(&self, cx: &App) -> Vec<AnyElement> {
        let columned = self.me.clone();
        let sifted = self.me.clone();
        let sorted = self.me.clone();
        let viewed = self.me.clone();

        let columns = matches!(self.mode(), Mode::List).then(|| {
            tools::columns(&self.popovers, self.column_toggles(cx), move |key, cx| {
                columned
                    .update(cx, |view, cx| view.switch_column(key, cx))
                    .ok();
            })
        });

        let mut tools = Vec::new();
        tools.extend(columns);
        tools.push(tools::filters(
            &self.popovers,
            &self.sliders[self.section.slot()],
            self.ranges(cx),
            self.flags(cx),
            move |change, cx| {
                sifted.update(cx, |view, cx| view.narrow(change, cx)).ok();
            },
            cx,
        ));
        tools.push(tools::sorts(
            &self.popovers,
            self.sorts(cx),
            move |key, cx| {
                sorted.update(cx, |view, cx| view.set_sort(key, cx)).ok();
            },
            cx,
        ));
        tools.push(tools::views(
            &self.popovers,
            self.mode(),
            move |mode, cx| {
                viewed.update(cx, |view, cx| view.set_mode(mode, cx)).ok();
            },
        ));
        tools
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

    fn narrow(&mut self, change: Sift, cx: &mut Context<Self>) {
        match change {
            Sift::Range("filter-year", value) => self.set_span(Some(value), cx),
            Sift::Range(_, value) => {
                let mut sieve = self.sieve(cx);
                sieve.duration = Some(value);
                self.sift(sieve, cx);
            }
            Sift::Flag(key, on) => {
                let mut sieve = self.sieve(cx);
                match key {
                    "filter-explicit" => sieve.explicit = on,
                    "filter-playable" => sieve.playable = on,
                    _ => return,
                }
                self.sift(sieve, cx);
            }
            Sift::Reset => {
                self.sift(TrackSieve::default(), cx);
                self.set_span(None, cx);
            }
        }
    }
}

fn deck<S: GridSource>(
    state: &Entity<GridState<S>>,
    cx: &App,
    card: impl Fn(usize, usize) -> Option<AnyElement>,
) -> Vec<AnyElement> {
    let state = state.read(cx);
    let delegate = state.delegate();
    let mut tiles = Vec::new();
    let mut group: Option<SharedString> = None;

    for display in 0..delegate.row_count() {
        let Some(card) = card(display, delegate.row(display)) else {
            continue;
        };
        let label = delegate.group(display, cx);
        match &label {
            Some(text) if group.as_ref() != Some(text) => tiles.push(head(text.clone(), cx)),
            _ => {}
        }
        group = label;
        tiles.push(card);
    }

    tiles
}

fn head(label: SharedString, cx: &App) -> AnyElement {
    heading(label, cx).w_full().pt_2().into_any_element()
}
