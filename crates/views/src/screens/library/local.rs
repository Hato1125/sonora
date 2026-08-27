use gpui::prelude::*;
use gpui::{
    AnyElement, App, Context, Entity, Pixels, Point, Render, ScrollHandle, SharedString,
    WeakEntity, Window, div, px,
};
use i18n::t;
use music::{Album, Track};
use state::{AppSettings, Library, LibraryPart, LibraryState, Origin, Playback, Sonora};
use ui::{
    ActiveTheme as _, Button, FlagAxis, GridDelegate, GridEvent, GridState, Popovers, Popup,
    RangeAxis, Scrollbar, Scroller, SortAxis, Table as _, Unit, grid, vacant,
};

use crate::chrome::Chrome;
use crate::chrome::tools::{self, Sift, Sliders};
use crate::chrome::{Searchable, Toolbar, Tooled};
use crate::shared::album_grid::AlbumGrid;
use crate::shared::cells;
use crate::shared::menu::album_menu;
use crate::shared::page;
use crate::shared::tracks::{
    LIBRARY_COLUMNS, PlaybackStatus, TrackSieve, TrackSource, Tracks, playback_status,
};

use super::albums::AlbumSource;

const PINNED: [&str; 3] = ["cover", "title", "name"];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    Tracks,
    Albums,
}

impl Section {
    const ALL: [Self; 2] = [Self::Tracks, Self::Albums];

    fn key(self) -> &'static str {
        match self {
            Section::Tracks => "local-songs",
            Section::Albums => "local-albums",
        }
    }

    fn slot(self) -> usize {
        match self {
            Section::Tracks => 0,
            Section::Albums => 1,
        }
    }

    fn vacancy(self) -> &'static str {
        match self {
            Section::Tracks => "library-no-local-songs",
            Section::Albums => "library-no-local-albums",
        }
    }

    fn part(self) -> LibraryPart {
        match self {
            Section::Tracks => LibraryPart::Tracks,
            Section::Albums => LibraryPart::Albums,
        }
    }
}

struct LocalTracks(Entity<Library>);

impl Tracks for LocalTracks {
    fn tracks<'a>(&self, cx: &'a App) -> &'a [Track] {
        match self.0.read(cx).local_state() {
            LibraryState::Ready { tracks, .. } => tracks.as_slice(),
            _ => &[],
        }
    }

    fn is_loading(&self, cx: &App) -> bool {
        matches!(self.0.read(cx).local_state(), LibraryState::Loading)
    }
}

pub(crate) struct LocalView {
    library: Entity<Library>,
    playback: Entity<Playback>,
    settings: Entity<AppSettings>,
    playback_status: PlaybackStatus,
    section: Section,
    width: Pixels,
    scrollbar: Entity<Scrollbar>,
    tracks: Entity<GridState<TrackSource>>,
    albums: Entity<GridState<AlbumSource>>,
    context_menu: Option<(Album, Point<Pixels>)>,
    toolbar: Entity<Toolbar>,
    popovers: Popovers,
    sliders: [Sliders; 2],
    me: WeakEntity<Self>,
}

impl LocalView {
    pub(crate) fn new(
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

        let id = cx.entity_id();
        let scrollbar = cx.new(|_| Scrollbar::new(ScrollHandle::new()).watching(id));
        let scroll = scrollbar.read(cx).scroll().clone();

        let tracks = cx.new(|cx| {
            let playlist_scrollbar = cx.new(|_| Scrollbar::inset().watching(id));
            let source = TrackSource::new(
                LIBRARY_COLUMNS,
                LocalTracks(library.clone()),
                playback.clone(),
                playlist_scrollbar,
            )
            .from(|_| Some(Origin::local()));
            let source = source.table(cx.weak_entity());
            let mut delegate = GridDelegate::new(source, width, cx);
            let (layout, sorting) = stored(Section::Tracks, cx);
            delegate.set_layout(layout, cx);
            if let Some(sorting) = sorting {
                delegate.set_sorting(sorting, cx);
            }
            GridState::new(delegate, cx).follow(scroll.clone())
        });

        let albums = cx.new(|cx| {
            let source = AlbumSource::local(library.clone(), playback.clone());
            let mut delegate = GridDelegate::new(source, width, cx);
            let (layout, sorting) = stored(Section::Albums, cx);
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
            cx.notify();
        })
        .detach();

        cx.subscribe(&tracks, |this, _, event, cx| match event {
            GridEvent::DoubleClicked(display) => {
                page::play(&this.tracks, &this.playback, *display, cx)
            }
            _ => this.persist(Section::Tracks, cx),
        })
        .detach();

        cx.subscribe(&albums, |this, _, event, cx| match event {
            GridEvent::DoubleClicked(_) => {}
            _ => this.persist(Section::Albums, cx),
        })
        .detach();

        let me = cx.entity();
        let toolbar = Toolbar::searchable(&me, cx);

        Self {
            library,
            playback,
            settings,
            playback_status: current_playback,
            section: Section::Tracks,
            width,
            scrollbar,
            tracks,
            albums,
            context_menu: None,
            toolbar,
            popovers: Popovers::default(),
            sliders: Section::ALL.map(|_| Sliders::default()),
            me: me.downgrade(),
        }
    }

    fn table(&self, section: Section) -> &dyn ui::Table {
        match section {
            Section::Tracks => &self.tracks,
            Section::Albums => &self.albums,
        }
    }

    fn tables(&self) -> [&dyn ui::Table; 2] {
        [&self.tracks, &self.albums]
    }

    fn rebuild(&mut self, cx: &mut Context<Self>) {
        for table in self.tables() {
            table.rebuild(cx);
        }
    }

    fn column_toggles(&self, cx: &App) -> Vec<ui::Toggle> {
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

    fn select(&mut self, section: Section, cx: &mut Context<Self>) {
        if self.section == section {
            return;
        }
        self.section = section;
        self.scrollbar
            .read(cx)
            .scroll()
            .set_offset(gpui::Point::default());
        if self.mode_is_table() {
            self.table(section).set_width(self.width, cx);
        }
        cx.notify();
    }

    fn mode_is_table(&self) -> bool {
        self.section == Section::Tracks
    }

    fn note(&self, cx: &App) -> Option<SharedString> {
        let library = self.library.read(cx);
        let table = self.table(self.section);
        match library.local_state() {
            LibraryState::Loading => return None,
            LibraryState::Failed(_) => return Some(t!("library-not-loaded")),
            _ if table.row_count(cx) > 0 => return None,
            _ => {}
        }

        Some(
            match (
                table.filtering(cx),
                library.local_part_failed(self.section.part()),
            ) {
                (true, _) => t!("library-no-matches"),
                (false, true) => t!("library-part-not-loaded"),
                (false, false) => i18n::lookup(self.section.vacancy(), None),
            },
        )
    }

    fn albums(&self, window: &Window, cx: &App) -> AnyElement {
        let inset = cx.theme().metrics.inset;
        let room = cells::content_width(window, page::reserved(inset), cx);
        let state = self.albums.read(cx);
        let delegate = state.delegate();
        let albums: Vec<(usize, Album)> = (0..delegate.row_count())
            .filter_map(|display| {
                let row = delegate.row(display);
                delegate.source().at(row, cx).map(|album| (display, album))
            })
            .collect();
        let view = self.me.clone();

        AlbumGrid::new("local-album", room, albums, self.playback.clone())
            .years()
            .on_context(move |album, position, cx| {
                let Some(view) = view.upgrade() else {
                    return;
                };
                view.update(cx, |this, cx| {
                    this.context_menu = Some((album.clone(), position));
                    cx.notify();
                });
            })
            .into_any_element()
    }

    fn toggle(&self, cx: &Context<Self>) -> AnyElement {
        div()
            .flex()
            .gap_1()
            .child(
                Button::new("local-section-tracks")
                    .label(t!("nav-songs"))
                    .small()
                    .outline()
                    .selected(self.section == Section::Tracks)
                    .on_click(cx.listener(|this, _, _, cx| this.select(Section::Tracks, cx))),
            )
            .child(
                Button::new("local-section-albums")
                    .label(t!("nav-albums"))
                    .small()
                    .outline()
                    .selected(self.section == Section::Albums)
                    .on_click(cx.listener(|this, _, _, cx| this.select(Section::Albums, cx))),
            )
            .into_any_element()
    }
}

impl Render for LocalView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.theme();
        let inset = theme.metrics.inset;
        let width = cells::content_width(window, Pixels::ZERO, cx);
        if (width - self.width).abs() >= px(0.5) {
            self.width = width;
            self.tracks.set_width(width, cx);
        }

        let scroll = self.scrollbar.read(cx).scroll().clone();
        if self.section == Section::Tracks {
            let viewport = page::viewport(&scroll, inset, window);
            self.tracks
                .update(cx, |table, _| table.set_viewport(viewport));
        }

        let note = self.note(cx);
        let content = match self.section {
            Section::Tracks => grid(&self.tracks).into_any_element(),
            Section::Albums => div()
                .px(inset)
                .child(self.albums(window, cx))
                .into_any_element(),
        };
        let context_menu = self.context_menu.clone().map(|(album, position)| {
            let menu = album_menu(album, self.playback.clone(), false, cx);
            Popup::new(position, menu).on_close(cx.listener(|this, _, _, cx| {
                this.context_menu = None;
                cx.notify();
            }))
        });

        let page = Scroller::new("local-page", &self.scrollbar)
            .pt(inset)
            .pb(inset)
            .child(div().px(inset).pb_3().child(self.toggle(cx)))
            .child(content)
            .when_some(note, |this, note| this.child(vacant(note, cx)));

        div()
            .relative()
            .size_full()
            .child(page)
            .when_some(context_menu, |this, menu| this.child(menu))
            .into_any_element()
    }
}

impl Searchable for LocalView {
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

impl Tooled for LocalView {
    fn toolbar(&self) -> Entity<Toolbar> {
        self.toolbar.clone()
    }

    fn tools(&self, cx: &App) -> Vec<AnyElement> {
        let columned = self.me.clone();
        let sifted = self.me.clone();
        let sorted = self.me.clone();

        let columns = (self.section == Section::Tracks).then(|| {
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
        tools
    }
}

impl LocalView {
    fn sorts(&self, cx: &App) -> Vec<SortAxis> {
        self.table(self.section).sortables(cx)
    }

    fn set_sort(&mut self, key: &'static str, cx: &mut Context<Self>) {
        self.table(self.section).cycle_sort(key, cx);
        cx.notify();
    }

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
