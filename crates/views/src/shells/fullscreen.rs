use std::time::{Duration, Instant};

use gpui::prelude::*;
use gpui::{
    AnyView, App, Context, Entity, FocusHandle, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, Pixels, Point, Render, SharedString, Task,
};
use gpui::{Window, div, px};
use i18n::t;
use input::{ToggleFullscreen, WORKSPACE_CONTEXT};
use router::{Destination, navigate};
use state::{Cover, Playback, Queue, SideTab, Sonora};
use ui::{
    ActiveTheme as _, Artwork, Button, InlineLink, InlineLinks, Motion, Motioned as _, Popup, Room,
    Scrollbar, Scrubber, ScrubberState, Text, clock, snapped,
};

use crate::chrome::{Aside, TitleBarOptions};
use crate::shared::menu::ItemMenu;
use crate::shared::transport::{like, transport};
use crate::shells::Shell;

const COVER_TALL: f32 = 0.46;
const COVER_WIDE: f32 = 0.34;
const COVER_TALL_TIGHT: f32 = 0.6;
const COVER_WIDE_TIGHT: f32 = 0.86;
const COVER_MIN: f32 = 96.;
const COVER_MAX: f32 = 520.;
const RESERVE: f32 = 2.9;
const PANEL: f32 = 460.;
const PILL_GAP: f32 = 2.;
const FROST: f32 = 16.;
const FROSTED: f32 = 0.5;
const SEEK_MAX: f32 = 420.;
const CLOCK_SHORT: f32 = 3.4;
const CLOCK_LONG: f32 = 5.4;
const REST: Duration = Duration::from_secs(3);
const WAKE_DEBOUNCE: Duration = Duration::from_millis(400);

pub struct FullscreenView {
    playback: Entity<Playback>,
    queue: Entity<Queue>,
    cover: Entity<Cover>,
    aside: Entity<Aside>,
    panel: Option<SideTab>,
    seek: ScrubberState,
    pending: Option<f32>,
    large: Option<SharedString>,
    revision: usize,
    track_menu: ItemMenu,
    context_menu: Option<(music::Track, Point<Pixels>)>,
    last_moved: Instant,
    awake: bool,
    rest: Option<Task<()>>,
    focus: FocusHandle,
}

impl FullscreenView {
    pub fn new(playback: Entity<Playback>, queue: Entity<Queue>, cx: &mut Context<Self>) -> Self {
        cx.observe(&playback, |_, _, cx| cx.notify()).detach();
        let cover = Sonora::global(cx).cover.clone();
        cx.observe(&cover, |_, _, cx| cx.notify()).detach();
        let library = Sonora::global(cx).library.clone();
        cx.observe(&library, |_, _, cx| cx.notify()).detach();
        let aside = cx.new(|cx| Aside::new(queue.clone(), playback.clone(), SideTab::Lyrics, cx));
        aside.update(cx, |aside, _| aside.strip());
        cx.observe(&aside, |_, _, cx| cx.notify()).detach();
        let playlist_scrollbar = cx.new(|_| {
            Scrollbar::new(gpui::ScrollHandle::new())
                .always_visible()
                .track_inset(px(4.))
        });

        Self {
            playback,
            queue,
            cover,
            aside,
            panel: Some(SideTab::Lyrics),
            seek: ScrubberState::new("fullscreen-seek"),
            pending: None,
            large: None,
            revision: 0,
            track_menu: ItemMenu::new(playlist_scrollbar),
            context_menu: None,
            last_moved: Instant::now(),
            awake: true,
            rest: None,
            focus: cx.focus_handle(),
        }
    }

    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        window.focus(&self.focus, cx);
    }

    fn show(&mut self, panel: Option<SideTab>, cx: &mut Context<Self>) {
        self.panel = panel;
        if let Some(tab) = panel {
            self.aside.update(cx, |aside, cx| aside.show(tab, cx));
        }
        self.stir(cx);
    }

    fn stir(&mut self, cx: &mut Context<Self>) {
        if !self.awake {
            self.awake = true;
            cx.notify();
        }
        self.last_moved = Instant::now();
        self.rest = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(REST).await;
            this.update(cx, |this, cx| {
                if this.last_moved.elapsed() < REST {
                    return;
                }
                this.awake = false;
                cx.notify();
            })
            .ok();
        }));
    }

    fn on_mouse_move(&mut self, _: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.awake && self.last_moved.elapsed() < WAKE_DEBOUNCE {
            return;
        }
        self.stir(cx);
    }

    fn commit_seek(&mut self, cx: &mut Context<Self>) {
        let Some(fraction) = self.pending.take() else {
            return;
        };
        self.playback
            .update(cx, |playback, cx| playback.seek_fraction(fraction, cx));
    }

    fn artwork(&mut self, side: Pixels, cx: &mut Context<Self>) -> impl IntoElement {
        let radius = cx.theme().radius * 2.;
        let track = self.playback.read(cx).track().cloned();
        let album = track.as_ref().and_then(|track| track.album_id.clone());
        let small = track.as_ref().and_then(|track| track.cover.clone());
        let large = self
            .cover
            .read(cx)
            .large()
            .filter(|url| Some(*url) != small.as_deref())
            .map(SharedString::from);

        if self.large != large {
            self.large = large.clone();
            self.revision += 1;
        }
        let revision = self.revision;
        let waiting = large.is_none();

        div()
            .id("fullscreen-artwork")
            .relative()
            .size(side)
            .flex_none()
            .when_some(album, |this, album| {
                this.cursor_pointer()
                    .on_click(move |_, _, cx| open_album(&album, cx))
            })
            .child(
                Artwork::new(small)
                    .size(side)
                    .corner_radius(radius)
                    .soft(waiting),
            )
            .when_some(large, |this, url| {
                this.child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .child(Artwork::new(Some(url)).size(side).corner_radius(radius))
                        .motion(("cover-large", revision), Motion::Slow, |art, t| {
                            art.opacity(t)
                        }),
                )
            })
    }

    fn meta(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.theme();
        let track = self.playback.read(cx).track().cloned();
        let title = match &track {
            Some(track) => SharedString::from(track.name.clone()),
            None => t!("player-nothing-playing"),
        };
        let album = track.as_ref().and_then(|track| track.album_id.clone());
        let held = track.clone();

        div()
            .flex()
            .flex_col()
            .items_center()
            .gap_1()
            .w_full()
            .min_w_0()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap_2()
                    .w_full()
                    .min_w_0()
                    .child(div().w(theme.metrics.control_small).flex_none())
                    .child(
                        div()
                            .id("fullscreen-title")
                            .min_w_0()
                            .truncate()
                            .text_size(theme.text(Text::Title))
                            .when_some(album, |this, album| {
                                this.hover(|style| style.underline())
                                    .on_click(move |_, _, cx| open_album(&album, cx))
                            })
                            .on_mouse_down(
                                MouseButton::Right,
                                cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                                    let Some(track) = held.clone() else {
                                        return;
                                    };
                                    window.prevent_default();
                                    this.track_menu.reset(cx);
                                    this.context_menu = Some((track, event.position));
                                    cx.stop_propagation();
                                    cx.notify();
                                }),
                            )
                            .child(title),
                    )
                    .child(like(track.clone(), cx)),
            )
            .when_some(track, |this, track| {
                this.child(
                    div().flex().w_full().min_w_0().justify_center().child(
                        InlineLinks::new(
                            "fullscreen-artists",
                            track.artist_refs.into_iter().map(|artist| {
                                InlineLink::new(artist.name, artist.id.map(Into::into))
                            }),
                            track.artists,
                            theme.muted_foreground,
                        )
                        .text_size(theme.text(Text::Body))
                        .truncate()
                        .on_click(|id, cx| navigate(Destination::Artist(id), cx)),
                    ),
                )
            })
    }

    fn strip(&self, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.theme();
        let cover = ui::snapped(theme.metrics.row, window);
        let track = self.playback.read(cx).track().cloned();
        let title = match &track {
            Some(track) => SharedString::from(track.name.clone()),
            None => t!("player-nothing-playing"),
        };
        let album = track.as_ref().and_then(|track| track.album_id.clone());

        div()
            .flex()
            .items_center()
            .gap_3()
            .w_full()
            .min_w_0()
            .child(
                div()
                    .id("strip-artwork")
                    .when_some(album.clone(), |this, album| {
                        this.cursor_pointer()
                            .on_click(move |_, _, cx| open_album(&album, cx))
                    })
                    .child(Artwork::new(track.as_ref().and_then(|t| t.cover.clone())).size(cover)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .justify_center()
                    .flex_1()
                    .min_w_0()
                    .child(div().min_w_0().truncate().child(title))
                    .when_some(track.clone(), |this, track| {
                        this.child(
                            InlineLinks::new(
                                "strip-artists",
                                track.artist_refs.into_iter().map(|artist| {
                                    InlineLink::new(artist.name, artist.id.map(Into::into))
                                }),
                                track.artists,
                                theme.muted_foreground,
                            )
                            .text_size(theme.text(Text::Small))
                            .truncate()
                            .on_click(|id, cx| navigate(Destination::Artist(id), cx)),
                        )
                    }),
            )
            .child(like(track, cx))
    }

    fn seek(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.theme();
        let muted = theme.muted_foreground;
        let empty = muted.opacity(0.3);
        let text = theme.text(Text::Tiny);
        let playback = self.playback.read(cx);
        let seekable = playback.track().is_some();
        let progress = self.pending.unwrap_or_else(|| playback.progress());
        let elapsed = playback.position();
        let total = playback
            .track()
            .map(|track| track.duration)
            .unwrap_or(Duration::ZERO);
        let width = text
            * match total.as_secs() >= 3600 {
                true => CLOCK_LONG,
                false => CLOCK_SHORT,
            };

        let label = move |value: Duration, align_end: bool| {
            div()
                .child(clock(value))
                .w(width)
                .flex_none()
                .whitespace_nowrap()
                .text_size(text)
                .text_color(muted)
                .when_else(align_end, |this| this.text_right(), |this| this.text_left())
        };

        div()
            .flex()
            .items_center()
            .gap_2()
            .w_full()
            .child(label(elapsed, true))
            .child(
                div().flex_1().min_w_0().child(
                    Scrubber::new(&self.seek, progress)
                        .colors(theme.progress_bar, empty, theme.foreground)
                        .enabled(seekable)
                        .on_move(cx.listener(|this, fraction: &f32, _, cx| {
                            this.pending = Some(*fraction);
                            cx.notify();
                        }))
                        .on_release(
                            cx.listener(|this, _: &MouseUpEvent, _, cx| this.commit_seek(cx)),
                        ),
                ),
            )
            .child(label(total, false))
    }

    fn controls(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let inline = self.panel.is_none();

        div()
            .flex()
            .flex_col()
            .items_center()
            .gap_3()
            .w_full()
            .max_w(px(SEEK_MAX))
            .flex_none()
            .when(inline, |this| this.child(self.pill(false, cx)))
            .child(self.seek(cx))
            .child(transport(&self.playback, &self.queue, true, cx))
    }

    fn pill(&self, fading: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.theme();
        let gap = px(PILL_GAP);
        let (from, to) = match self.awake || !fading {
            true => (0., 1.),
            false => (1., 0.),
        };

        let tab = move |id: &'static str, icon: &'static str, hint: &'static str, panel| {
            let showing = self.panel == panel;

            Button::new(id)
                .ghost()
                .small()
                .icon(icon)
                .tooltip_above(hint)
                .selected(showing)
                .rounded(theme.radius)
                .tint(match showing {
                    true => theme.foreground,
                    false => theme.muted_foreground,
                })
                .on_click(cx.listener(move |this, _, _, cx| this.show(panel, cx)))
        };

        div()
            .flex()
            .flex_none()
            .items_center()
            .gap(gap)
            .p(gap)
            .rounded(theme.radius + gap)
            .border_1()
            .border_color(theme.border)
            .backdrop_blur(px(FROST))
            .bg(theme.popover.opacity(FROSTED))
            .child(tab(
                "fullscreen-artwork-tab",
                "icons/disc-3.svg",
                "fullscreen-artwork",
                None,
            ))
            .child(tab(
                "fullscreen-lyrics",
                "icons/mic-vocal.svg",
                "lyrics-title",
                Some(SideTab::Lyrics),
            ))
            .child(tab(
                "fullscreen-queue",
                "icons/list-music.svg",
                "queue-title",
                Some(SideTab::Queue),
            ))
            .motion(
                ("pill", usize::from(self.awake || !fading)),
                Motion::Base,
                move |pill, t| pill.opacity(from + (to - from) * t),
            )
    }

    fn floating(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .absolute()
            .bottom_3()
            .w_full()
            .flex()
            .justify_center()
            .child(
                div()
                    .flex()
                    .flex_none()
                    .block_mouse_except_scroll()
                    .child(self.pill(true, cx)),
            )
    }

    fn menu(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        let (track, position) = self.context_menu.clone()?;

        Some(
            Popup::new(position, self.track_menu.for_track(&track, cx)).on_close(cx.listener(
                |this, _, _, cx| {
                    this.context_menu = None;
                    cx.notify();
                },
            )),
        )
    }

    fn leave(&self) -> Button {
        Button::new("leave-fullscreen")
            .ghost()
            .icon("icons/chevron-down.svg")
            .tooltip("player-fullscreen-leave")
            .on_click(|_, window, cx| window.dispatch_action(Box::new(ToggleFullscreen), cx))
    }
}

fn open_album(album: &str, cx: &mut App) {
    navigate(Destination::Album(album.into()), cx);
}

impl Shell for FullscreenView {
    fn title_bar(&self, _content: Option<AnyView>, _cx: &App) -> TitleBarOptions {
        TitleBarOptions {
            navigation: false,
            sidebar_open: false,
            sidebar_right: None,
            offset: Pixels::ZERO,
            border: false,
            content: None,
        }
    }
}

impl Render for FullscreenView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.theme();
        let viewport = window.viewport_size();
        let room = Room::of(viewport.width);
        let split = room.fits(Room::Wide) && self.panel.is_some();
        let reserve = theme.metrics.title_bar + theme.metrics.player_bar * RESERVE;
        let (tall, wide, ceiling) = match room.fits(Room::Wide) {
            true => (COVER_TALL, COVER_WIDE, px(COVER_MAX)),
            false => (COVER_TALL_TIGHT, COVER_WIDE_TIGHT, viewport.width),
        };
        let side = snapped(
            (viewport.height * tall)
                .min(viewport.height - reserve)
                .min(viewport.width * wide)
                .min(ceiling)
                .max(px(COVER_MIN)),
            window,
        );
        let staged = self.panel.is_none() || split;

        div()
            .id("fullscreen")
            .key_context(WORKSPACE_CONTEXT)
            .track_focus(&self.focus)
            .relative()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .w_full()
            .gap_5()
            .px_8()
            .pb_6()
            .bg(theme.background)
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .child(
                div()
                    .relative()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .items_center()
                    .justify_center()
                    .gap_8()
                    .when(staged, |this| {
                        this.child(
                            div()
                                .flex()
                                .flex_col()
                                .items_center()
                                .justify_center()
                                .gap_5()
                                .min_w_0()
                                .when_else(
                                    split,
                                    |this| this.flex_1().h_full(),
                                    |this| this.w_full(),
                                )
                                .child(self.artwork(side, cx))
                                .child(self.meta(cx))
                                .when(split, |this| this.child(self.controls(cx))),
                        )
                    })
                    .when(self.panel.is_some(), |this| {
                        this.child(
                            div()
                                .relative()
                                .flex()
                                .flex_col()
                                .flex_1()
                                .min_w_0()
                                .min_h_0()
                                .h_full()
                                .when(split, |this| this.max_w(px(PANEL)))
                                .child(self.aside.clone())
                                .child(self.floating(cx)),
                        )
                    }),
            )
            .when(!split && self.panel.is_some(), |this| {
                this.child(self.strip(window, cx))
            })
            .when(!split, |this| {
                this.child(
                    div()
                        .flex()
                        .w_full()
                        .justify_center()
                        .child(self.controls(cx)),
                )
            })
            .child(div().absolute().top_0().right_3().child(self.leave()))
            .children(self.menu(cx))
    }
}
