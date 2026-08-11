// SPDX-License-Identifier: GPL-3.0-or-later

use gpui::prelude::*;
use gpui::{
    AnyElement, App, Context, Entity, Pixels, Point, Render, ScrollHandle, SharedString,
    WeakEntity, Window, div, px,
};
use i18n::t;
use music::{Album, Track};
use state::{Library, LibraryState, Playback};
use ui::{
    ActiveTheme as _, Button, GridDelegate, GridEvent, GridState, Popup, Scrollbar, Scroller, grid,
    scrolled, vacant,
};

use crate::chrome::Chrome;
use crate::shared::album_grid::AlbumGrid;
use crate::shared::cells;
use crate::shared::menu::album_menu;
use crate::shared::page;
use crate::shared::tracks::{
    LIBRARY_COLUMNS, PlaybackStatus, TrackSource, Tracks, playback_status,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    Tracks,
    Albums,
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
    playback_status: PlaybackStatus,
    section: Section,
    width: Pixels,
    scrollbar: Entity<Scrollbar>,
    tracks: Entity<GridState<TrackSource>>,
    context_menu: Option<(Album, Point<Pixels>)>,
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
                LocalTracks(library.clone()),
                playback.clone(),
                playlist_scrollbar,
            );
            let source = source.table(cx.weak_entity());
            let delegate = GridDelegate::new(source, width, cx);
            GridState::new(delegate, cx).follow(scroll)
        });

        cx.observe(&library, |this, _, cx| {
            this.tracks.update(cx, |table, cx| table.rebuild(cx));
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
            this.tracks.update(cx, |table, cx| table.refresh(cx));
        })
        .detach();

        cx.subscribe(&tracks, |this, _, event, cx| {
            if let GridEvent::DoubleClicked(display) = event {
                page::play(&this.tracks, &this.playback, *display, cx);
            }
        })
        .detach();

        Self {
            library,
            playback,
            playback_status: current_playback,
            section: Section::Tracks,
            width,
            scrollbar,
            tracks,
            context_menu: None,
            me: cx.weak_entity(),
        }
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
        cx.notify();
    }

    fn note(&self, cx: &App) -> Option<SharedString> {
        let state = self.library.read(cx).local_state();
        if matches!(state, LibraryState::Loading) {
            return None;
        }
        let empty = match (self.section, state) {
            (Section::Tracks, LibraryState::Ready { tracks, .. }) => tracks.is_empty(),
            (Section::Albums, LibraryState::Ready { albums, .. }) => albums.is_empty(),
            _ => !matches!(state, LibraryState::Ready { .. }),
        };
        if !empty {
            return None;
        }
        Some(match self.section {
            Section::Tracks => t!("library-no-local-songs"),
            Section::Albums => t!("library-no-local-albums"),
        })
    }

    fn albums(&self, cx: &App) -> AnyElement {
        let albums = match self.library.read(cx).local_state() {
            LibraryState::Ready { albums, .. } => albums.clone(),
            _ => Vec::new(),
        };
        let view = self.me.clone();
        AlbumGrid::new(
            "local-album",
            self.width,
            albums.into_iter().enumerate(),
            self.playback.clone(),
        )
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

fn viewport(scroll: &ScrollHandle, window: &Window) -> ui::Viewport {
    let visible = scroll.bounds().size.height;

    ui::Viewport {
        top: scrolled(scroll),
        height: match visible > Pixels::ZERO {
            true => visible,
            false => window.viewport_size().height,
        },
    }
}

impl Render for LocalView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.theme();
        let inset = theme.metrics.inset;
        page::resize(&self.tracks, &mut self.width, inset, window, cx);

        let scroll = self.scrollbar.read(cx).scroll().clone();
        if self.section == Section::Tracks {
            let viewport = viewport(&scroll, window);
            self.tracks
                .update(cx, |table, _| table.set_viewport(viewport));
        }

        let note = self.note(cx);
        let content = match self.section {
            Section::Tracks => grid(&self.tracks).into_any_element(),
            Section::Albums => self.albums(cx),
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
            .px(inset)
            .child(div().pb_3().child(self.toggle(cx)))
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
