use std::ops::Range;

use gpui::prelude::*;

use gpui::{
    Context, DragMoveEvent, Entity, FontWeight, MouseButton, MouseDownEvent, Pixels, Point, Render,
    ScrollHandle, ScrollStrategy, ScrollWheelEvent, SharedString, UniformListScrollHandle, Window,
    div, px, uniform_list,
};
use i18n::t;
use music::Track;
use state::{Lyrics, LyricsState, Playback, PlaybackState, Queue, SideTab, Sonora};
use ui::{
    ActiveTheme as _, Button, Card, DraggedPin, Edge, Pin, PinKind, Pinnable as _, Popup,
    Scrollbar, Scroller, Spot, Text, drop_gap, drop_marker, eyebrow, snapped, vacant,
};

use crate::chrome::{Chrome, section_label};
use crate::shared::menu::ItemMenu;

const QUEUE: &str = "queue";
const PINNED_SHARE: f32 = 0.25;
const PIN: f32 = 0.3;
const SETTLE: std::time::Duration = std::time::Duration::from_secs(4);

fn track(queue: &Queue, position: QueuePosition) -> Option<Track> {
    match position {
        QueuePosition::Past(index) => queue.past().nth(index).cloned(),
        QueuePosition::Current => queue.current().cloned(),
        QueuePosition::Upcoming(index) => queue.upcoming().nth(index).cloned(),
        QueuePosition::Similar(index) => queue.similar().nth(index).cloned(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum QueuePosition {
    Past(usize),
    Current,
    Upcoming(usize),
    Similar(usize),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Slot {
    Header(&'static str),
    Track(QueuePosition),
}

#[derive(Clone, Copy)]
struct Sections {
    past: usize,
    current: bool,
    upcoming: usize,
    similar: usize,
}

impl Sections {
    fn past_end(self) -> usize {
        match self.past {
            0 => 0,
            count => count + 1,
        }
    }

    fn current_end(self) -> usize {
        self.past_end() + 2 * usize::from(self.current)
    }

    fn upcoming_end(self) -> usize {
        self.current_end()
            + match self.upcoming {
                0 => 0,
                count => count + 1,
            }
    }

    fn len(self) -> usize {
        self.upcoming_end()
            + match self.similar {
                0 => 0,
                count => count + 1,
            }
    }

    fn current_index(self) -> Option<usize> {
        self.current.then(|| self.past_end() + 1)
    }

    fn slot(self, index: usize) -> Slot {
        if index < self.past_end() {
            return match index {
                0 => Slot::Header("queue-history"),
                _ => Slot::Track(QueuePosition::Past(index - 1)),
            };
        }
        if index < self.current_end() {
            return match index == self.past_end() {
                true => Slot::Header("queue-now-playing"),
                false => Slot::Track(QueuePosition::Current),
            };
        }
        if index < self.upcoming_end() {
            return match index == self.current_end() {
                true => Slot::Header("queue-up-next"),
                false => Slot::Track(QueuePosition::Upcoming(index - self.current_end() - 1)),
            };
        }
        match index == self.upcoming_end() {
            true => Slot::Header("queue-similar"),
            false => Slot::Track(QueuePosition::Similar(index - self.upcoming_end() - 1)),
        }
    }
}

#[derive(Clone)]
struct ContextMenuState {
    track: Track,
    revision: u64,
    position: Point<Pixels>,
}

impl QueuePosition {
    fn past(self) -> Option<usize> {
        match self {
            Self::Past(index) => Some(index),
            _ => None,
        }
    }

    fn upcoming(self) -> Option<usize> {
        match self {
            Self::Upcoming(index) => Some(index),
            _ => None,
        }
    }

    fn similar(self) -> Option<usize> {
        match self {
            Self::Similar(index) => Some(index),
            _ => None,
        }
    }
}

pub(crate) struct Aside {
    queue: Entity<Queue>,
    playback: Entity<Playback>,
    lyrics: Entity<Lyrics>,
    tab: SideTab,
    verse_bar: Entity<Scrollbar>,
    followed: Option<usize>,
    goal: Option<Pixels>,
    pinned: bool,
    nudged: Option<std::time::Instant>,
    verse_of: Option<String>,
    context_menu: Option<ContextMenuState>,
    track_menu: ItemMenu,
    drop_gap: Option<usize>,
    scroll: UniformListScrollHandle,
    scrollbar: Entity<Scrollbar>,
    past_len: usize,
    anchor: bool,
    titled: bool,
}

impl Aside {
    pub(crate) fn new(
        queue: Entity<Queue>,
        playback: Entity<Playback>,
        tab: SideTab,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&queue, |this, queue, cx| {
            let revision = queue.read(cx).revision();
            if this
                .context_menu
                .as_ref()
                .is_some_and(|menu| menu.revision != revision)
            {
                this.track_menu.reset(cx);
                this.context_menu = None;
            }
            cx.notify();
        })
        .detach();
        cx.observe(&playback, |_, _, cx| cx.notify()).detach();
        let chrome = Chrome::entity(cx);
        cx.observe(&chrome, |_, _, cx| cx.notify()).detach();

        let scroll = UniformListScrollHandle::new();
        let scrollbar = cx.new(|_| Scrollbar::new(scroll.0.borrow().base_handle.clone()));
        let playlist_scrollbar = cx.new(|_| {
            Scrollbar::new(ScrollHandle::new())
                .always_visible()
                .track_inset(px(4.))
        });
        let lyrics = Sonora::global(cx).lyrics.clone();
        cx.observe(&lyrics, |_, _, cx| cx.notify()).detach();
        let verse_bar = cx.new(|_| Scrollbar::new(ScrollHandle::new()));

        Self {
            queue,
            playback,
            lyrics,
            tab,
            verse_bar,
            followed: None,
            goal: None,
            pinned: true,
            nudged: None,
            verse_of: None,
            context_menu: None,
            track_menu: ItemMenu::new(playlist_scrollbar),
            drop_gap: None,
            scroll,
            scrollbar,
            past_len: 0,
            anchor: true,
            titled: true,
        }
    }

    pub(crate) fn strip(&mut self) {
        self.titled = false;
    }

    pub(crate) fn tab(&self) -> SideTab {
        self.tab
    }

    pub(crate) fn show(&mut self, tab: SideTab, cx: &mut Context<Self>) {
        if self.tab != tab {
            self.tab = tab;
            self.anchor_verse();
        }
        self.anchor = true;
        cx.notify();
    }

    pub(crate) fn dismiss(&mut self, cx: &mut Context<Self>) {
        self.track_menu.reset(cx);
        self.context_menu = None;
        cx.notify();
    }

    fn dismiss_menu(&mut self, cx: &mut Context<Self>) {
        self.track_menu.reset(cx);
        self.context_menu = None;
        cx.notify();
    }

    fn row(
        track: Track,
        index: usize,
        position: QueuePosition,
        queue_revision: u64,
        drop_line: Option<Edge>,
        playing: bool,
        cx: &Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let theme = *cx.theme();
        let past_index = position.past();
        let queue_index = position.upcoming();
        let similar_index = position.similar();
        let title = match position {
            QueuePosition::Past(_) => theme.muted_foreground,
            QueuePosition::Current => theme.primary,
            QueuePosition::Upcoming(_) | QueuePosition::Similar(_) => theme.foreground,
        };
        let pin = track
            .id
            .clone()
            .map(|id| Pin::new(PinKind::Song, id, track.name.clone()).cover(track.cover.clone()));
        let menu_track = track.clone();

        let card = Card::new(
            ("queue-track", index),
            SharedString::from(track.name.clone()),
        )
        .cover(track.cover.clone())
        .bare_meta(
            crate::shared::cells::artist_links(
                SharedString::from(format!("queue-track-artist-{index}")),
                track.artist_refs.clone(),
                track.artists.clone(),
                theme.muted_foreground,
            )
            .text_size(theme.text(Text::Small))
            .truncate(),
        )
        .tint(title)
        .when(track.explicit, Card::explicit)
        .play(
            playing,
            cx.listener(move |this, _, _, cx| {
                let stale = this.queue.read(cx).revision() != queue_revision;
                this.playback.update(cx, |playback, cx| match position {
                    QueuePosition::Current => playback.toggle_play(cx),
                    QueuePosition::Past(index) if !stale => playback.play_past(index, cx),
                    QueuePosition::Upcoming(index) if !stale => playback.play_upcoming(index, cx),
                    QueuePosition::Similar(index) if !stale => playback.play_similar(index, cx),
                    _ => {}
                });
            }),
        )
        .on_mouse_down(
            MouseButton::Right,
            cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                window.prevent_default();
                this.track_menu.reset(cx);
                this.context_menu = Some(ContextMenuState {
                    track: menu_track.clone(),
                    revision: queue_revision,
                    position: event.position,
                });
                cx.stop_propagation();
                cx.notify();
            }),
        )
        .when_some(past_index, |this, index| {
            this.press(cx.listener(move |this, _, _, cx| {
                if this.queue.read(cx).revision() == queue_revision {
                    this.playback
                        .update(cx, |playback, cx| playback.play_past(index, cx));
                }
            }))
        })
        .when_some(queue_index, |this, target| {
            this.press(cx.listener(move |this, _, _, cx| {
                if this.queue.read(cx).revision() == queue_revision {
                    this.playback
                        .update(cx, |playback, cx| playback.play_upcoming(target, cx));
                }
            }))
            .action(
                Button::new(("remove-queued-track", index))
                    .ghost()
                    .small()
                    .icon("icons/x.svg")
                    .tooltip("menu-remove-from-queue")
                    .tint(theme.muted_foreground)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.queue.update(cx, |queue, cx| {
                            if queue.revision() == queue_revision {
                                queue.remove_upcoming(target, cx);
                            }
                        });
                    })),
            )
            .on_drag_move(
                cx.listener(move |this, event: &DragMoveEvent<DraggedPin>, _, cx| {
                    let Some(gap) = drop_gap(event.bounds, event.event.position, target) else {
                        return;
                    };
                    let Some(held) = event.drag(cx).spot(QUEUE) else {
                        return;
                    };
                    let gap = (gap != held.index && gap != held.index + 1).then_some(gap);
                    if this.drop_gap != gap {
                        this.drop_gap = gap;
                        cx.notify();
                    }
                }),
            )
            .on_drop(cx.listener(move |this, dragged: &DraggedPin, _, cx| {
                let Some(held) = dragged.spot(QUEUE) else {
                    return;
                };
                if let Some(gap) = this.drop_gap.take() {
                    this.queue.update(cx, |queue, cx| {
                        if queue.revision() == held.revision {
                            queue.move_upcoming_to_gap(held.index, gap, cx);
                        }
                    });
                }
            }))
        })
        .when_some(similar_index, |this, target| {
            this.press(cx.listener(move |this, _, _, cx| {
                if this.queue.read(cx).revision() == queue_revision {
                    this.playback
                        .update(cx, |playback, cx| playback.play_similar(target, cx));
                }
            }))
            .action(
                Button::new(("remove-similar-track", index))
                    .ghost()
                    .small()
                    .icon("icons/x.svg")
                    .tooltip("menu-remove-from-queue")
                    .tint(theme.muted_foreground)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.queue.update(cx, |queue, cx| {
                            if queue.revision() == queue_revision {
                                queue.remove_similar(target, cx);
                            }
                        });
                    })),
            )
        })
        .when_some(pin, |this, pin| match queue_index {
            Some(index) => this.pin_from(pin, Spot::new(QUEUE, index).revision(queue_revision)),
            None => this.pin(pin),
        });

        div()
            .id(("queue-track-container", index))
            .relative()
            .min_w_0()
            .child(card)
            .when_some(drop_line, |this, edge| this.child(drop_marker(edge, cx)))
    }

    fn menu(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        let ContextMenuState {
            track, position, ..
        } = self.context_menu.clone()?;

        Some(
            Popup::new(position, self.track_menu.for_track(&track, cx))
                .on_close(cx.listener(|this, _, _, cx| this.dismiss_menu(cx))),
        )
    }

    fn header(&self, sections: Sections, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.theme();

        div()
            .flex()
            .flex_none()
            .items_center()
            .justify_between()
            .gap_2()
            .h(theme.metrics.header)
            .px_2()
            .when(self.titled, |this| {
                this.border_b_1().border_color(theme.border).child(eyebrow(
                    match self.tab {
                        SideTab::Queue => t!("queue-title"),
                        SideTab::Lyrics => t!("lyrics-title"),
                    },
                    cx,
                ))
            })
            .when(!self.titled, |this| this.justify_end())
            .when(self.tab == SideTab::Queue, |this| {
                this.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(
                            Button::new("toggle-radio")
                                .ghost()
                                .small()
                                .icon("icons/radio.svg")
                                .tooltip("queue-radio")
                                .tint(match self.playback.read(cx).radio() {
                                    true => theme.primary,
                                    false => theme.muted_foreground,
                                })
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.playback
                                        .update(cx, |playback, cx| playback.toggle_radio(cx));
                                })),
                        )
                        .child(
                            Button::new("reset-queue")
                                .ghost()
                                .small()
                                .label(t!("queue-reset"))
                                .tint(theme.muted_foreground)
                                .disabled(!self.queue.read(cx).reordered())
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.queue.update(cx, |queue, cx| queue.reset(cx));
                                })),
                        )
                        .child(
                            Button::new("clear-queue")
                                .ghost()
                                .small()
                                .label(t!("queue-clear"))
                                .tint(theme.muted_foreground)
                                .disabled(sections.upcoming == 0)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.queue.update(cx, |queue, cx| queue.clear_upcoming(cx));
                                })),
                        ),
                )
            })
    }

    fn follow(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        let theme = *cx.theme();
        if self.tab != SideTab::Lyrics || self.pinned {
            return None;
        }

        Some(
            div()
                .absolute()
                .bottom_3()
                .w_full()
                .flex()
                .justify_center()
                .child(
                    div().flex().flex_none().block_mouse_except_scroll().child(
                        Button::new("resume-pin")
                            .ghost()
                            .small()
                            .icon("icons/undo-2.svg")
                            .tooltip("lyrics-follow")
                            .rounded_full()
                            .border_1()
                            .border_color(theme.border)
                            .bg(theme.popover)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.anchor_verse();
                                cx.notify();
                            })),
                    ),
                ),
        )
    }

    fn verses(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.theme();
        let at = self.playback.read(cx).position();
        let lyrics = self.lyrics.read(cx);
        let state = lyrics.state().clone();
        let shown = lyrics.current().map(|hit| hit.lyrics.clone());
        let following = lyrics.following().map(str::to_owned);

        let empty = |key: &'static str, cx: &mut Context<Self>| {
            vacant(i18n::lookup(key, None), cx)
                .flex_1()
                .into_any_element()
        };
        let lines = match (&state, &shown) {
            (LyricsState::Ready, Some(music::Lyrics::Synced { lines })) => Some(lines.clone()),
            _ => None,
        };

        let body: Vec<gpui::AnyElement> = match (&lines, &state) {
            (Some(lines), _) => {
                let sung = music::lyrics::active(lines, at);
                lines
                    .iter()
                    .enumerate()
                    .map(|(index, line)| {
                        let seek = line.start;
                        div()
                            .id(("verse", index))
                            .px_2()
                            .py_1()
                            .rounded(theme.radius)
                            .cursor_pointer()
                            .hover(|style| style.bg(theme.table_hover))
                            .text_size(theme.text(Text::Large))
                            .when_else(
                                Some(index) == sung,
                                |this| {
                                    this.text_color(theme.foreground)
                                        .font_weight(FontWeight::SEMIBOLD)
                                },
                                |this| this.text_color(theme.muted_foreground),
                            )
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.playback
                                    .update(cx, |playback, cx| playback.seek(seek, cx));
                            }))
                            .child(SharedString::from(line.text.clone()))
                            .into_any_element()
                    })
                    .collect()
            }
            (None, LyricsState::Ready) => match &shown {
                Some(music::Lyrics::Plain(text)) => vec![
                    div()
                        .px_2()
                        .text_size(theme.text(Text::Body))
                        .text_color(theme.muted_foreground)
                        .child(SharedString::from(text.clone()))
                        .into_any_element(),
                ],
                _ => vec![empty("lyrics-missing", cx)],
            },
            (None, LyricsState::Idle) => vec![empty("lyrics-idle", cx)],
            (None, LyricsState::Loading) => vec![empty("lyrics-loading", cx)],
            (None, LyricsState::Missing) => vec![empty("lyrics-missing", cx)],
            (None, LyricsState::Failed(_)) => vec![empty("lyrics-failed", cx)],
        };

        if self.verse_of != following {
            self.verse_of = following;
            self.anchor_verse();
            let scroll = self.verse_bar.read(cx).scroll().clone();
            scroll.set_offset(gpui::point(scroll.offset().x, px(0.)));
            self.verse_bar
                .update(cx, |bar, _| bar.settle(scroll.offset().y));
        }
        if let Some(lines) = &lines {
            self.pin_verse(music::lyrics::active(lines, at), window, cx);
        }

        Scroller::new("lyrics", &self.verse_bar)
            .flex()
            .flex_col()
            .gap_1()
            .flex_1()
            .min_h_0()
            .px_1()
            .pb_12()
            .children(body)
    }

    fn anchor_verse(&mut self) {
        self.pinned = true;
        self.followed = None;
        self.goal = None;
        self.nudged = None;
    }

    fn pin_verse(&mut self, sung: Option<usize>, window: &mut Window, cx: &mut Context<Self>) {
        let scroll = self.verse_bar.read(cx).scroll().clone();
        let aimed = self.verse_bar.read(cx).goal();
        if let Some(goal) = self.goal
            && (aimed - goal).abs() > px(1.)
        {
            self.pinned = false;
            self.goal = None;
            self.nudged = Some(std::time::Instant::now());
        }
        if !self.pinned {
            self.followed = sung;
            if self.nudged.is_some_and(|at| at.elapsed() >= SETTLE) {
                self.anchor_verse();
            } else {
                return;
            }
        }
        let Some(index) = sung else {
            return;
        };
        if self.followed == sung {
            return;
        }
        let Some(item) = scroll.bounds_for_item(index) else {
            return;
        };
        self.followed = sung;
        let view = scroll.bounds();
        let rest = view.origin.y - item.origin.y + view.size.height * PIN;
        let goal = rest.clamp(-scroll.max_offset().y, px(0.));
        self.verse_bar.update(cx, |bar, _| bar.aim(goal, window));
        self.goal = Some(self.verse_bar.read(cx).goal());
    }

    fn pin(&mut self, sections: Sections, window: &Window, cx: &Context<Self>) {
        let Some(index) = sections.current_index() else {
            self.anchor = false;
            return;
        };

        let viewport = self.scroll.0.borrow().base_handle.bounds().size.height;
        if viewport <= px(0.) {
            window.request_animation_frame();
            return;
        }

        let row = snapped(cx.theme().metrics.list_row, window);
        let above = (viewport * PINNED_SHARE / row).round() as usize;
        self.scroll
            .scroll_to_item_strict_with_offset(index, ScrollStrategy::Top, above);
        self.anchor = false;
    }

    fn rows(&self, sections: Sections, cx: &mut Context<Self>) -> gpui::UniformList {
        let queue = self.queue.clone();
        let drop_gap = self.drop_gap;
        let upcoming = sections.upcoming;
        let audible = matches!(self.playback.read(cx).state(), PlaybackState::Playing);

        uniform_list(
            "queue-rows",
            sections.len(),
            cx.processor(move |_, range: Range<usize>, window, cx| {
                let (revision, slots) = {
                    let queue = queue.read(cx);
                    let slots = range
                        .clone()
                        .map(|index| {
                            let slot = sections.slot(index);
                            let found = match slot {
                                Slot::Header(_) => None,
                                Slot::Track(position) => track(queue, position),
                            };
                            (index, slot, found)
                        })
                        .collect::<Vec<_>>();
                    (queue.revision(), slots)
                };

                slots
                    .into_iter()
                    .map(|(index, slot, found)| match (slot, found) {
                        (Slot::Header(key), _) => section_label(key, window, cx).into_any_element(),
                        (Slot::Track(position), Some(found)) => {
                            let drop_line = match (position.upcoming(), drop_gap) {
                                (Some(queued), Some(gap)) if gap == queued => Some(Edge::Above),
                                (Some(queued), Some(gap))
                                    if gap == upcoming && queued + 1 == upcoming =>
                                {
                                    Some(Edge::Below)
                                }
                                _ => None,
                            };
                            let playing = audible && position == QueuePosition::Current;
                            Self::row(found, index, position, revision, drop_line, playing, cx)
                                .into_any_element()
                        }
                        (Slot::Track(_), None) => div().into_any_element(),
                    })
                    .collect()
            }),
        )
    }
}

impl Render for Aside {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.scrollbar.read(cx).sync();
        let queue = self.queue.read(cx);
        let sections = Sections {
            past: queue.past().len(),
            current: queue.current().is_some(),
            upcoming: queue.upcoming().len(),
            similar: queue.similar().len(),
        };
        let empty = sections.len() == 0;
        if !cx.has_active_drag() {
            self.drop_gap = None;
        }

        if self.past_len != sections.past {
            self.past_len = sections.past;
            self.anchor = true;
        }
        if self.anchor && self.tab == SideTab::Queue {
            self.pin(sections, window, cx);
        }

        div()
            .id("aside")
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .on_drag_move(cx.listener(|this, _: &DragMoveEvent<DraggedPin>, _, cx| {
                if this.drop_gap.take().is_some() {
                    cx.notify();
                }
            }))
            .child(self.header(sections, cx))
            .child(
                div()
                    .relative()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h_0()
                    .when(self.tab == SideTab::Lyrics, |this| {
                        this.child(self.verses(window, cx))
                    })
                    .when(self.tab == SideTab::Queue && empty, |this| {
                        this.child(vacant(t!("queue-empty"), cx).flex_1())
                    })
                    .when(self.tab == SideTab::Queue && !empty, |this| {
                        let gliding = self.scrollbar.clone();

                        this.child(
                            div()
                                .relative()
                                .flex_1()
                                .min_h_0()
                                .child(
                                    self.rows(sections, cx)
                                        .px_2()
                                        .pb_2()
                                        .track_scroll(&self.scroll)
                                        .size_full()
                                        .on_scroll_wheel(
                                            move |event: &ScrollWheelEvent, window, cx| {
                                                if event.delta.precise() {
                                                    return;
                                                }
                                                gliding.update(cx, |bar, _| bar.nudge(window));
                                            },
                                        ),
                                )
                                .child(self.scrollbar.clone()),
                        )
                    })
                    .children(self.follow(cx)),
            )
            .children(self.menu(cx))
    }
}

#[cfg(test)]
mod tests {
    use super::{QueuePosition, Sections, Slot};

    fn slots(sections: Sections) -> Vec<Slot> {
        (0..sections.len()).map(|i| sections.slot(i)).collect()
    }

    #[test]
    fn lays_out_every_section() {
        let sections = Sections {
            past: 2,
            current: true,
            upcoming: 2,
            similar: 2,
        };

        assert_eq!(sections.current_index(), Some(4));
        assert_eq!(
            slots(sections),
            [
                Slot::Header("queue-history"),
                Slot::Track(QueuePosition::Past(0)),
                Slot::Track(QueuePosition::Past(1)),
                Slot::Header("queue-now-playing"),
                Slot::Track(QueuePosition::Current),
                Slot::Header("queue-up-next"),
                Slot::Track(QueuePosition::Upcoming(0)),
                Slot::Track(QueuePosition::Upcoming(1)),
                Slot::Header("queue-similar"),
                Slot::Track(QueuePosition::Similar(0)),
                Slot::Track(QueuePosition::Similar(1)),
            ]
        );
    }

    #[test]
    fn suggests_similar_tracks_without_anything_up_next() {
        let sections = Sections {
            past: 0,
            current: true,
            upcoming: 0,
            similar: 1,
        };

        assert_eq!(
            slots(sections),
            [
                Slot::Header("queue-now-playing"),
                Slot::Track(QueuePosition::Current),
                Slot::Header("queue-similar"),
                Slot::Track(QueuePosition::Similar(0)),
            ]
        );
    }

    #[test]
    fn drops_headers_for_empty_sections() {
        let sections = Sections {
            past: 0,
            current: true,
            upcoming: 1,
            similar: 0,
        };

        assert_eq!(sections.current_index(), Some(1));
        assert_eq!(
            slots(sections),
            [
                Slot::Header("queue-now-playing"),
                Slot::Track(QueuePosition::Current),
                Slot::Header("queue-up-next"),
                Slot::Track(QueuePosition::Upcoming(0)),
            ]
        );
    }

    #[test]
    fn lays_out_history_without_a_current_track() {
        let sections = Sections {
            past: 1,
            current: false,
            upcoming: 0,
            similar: 0,
        };

        assert_eq!(sections.current_index(), None);
        assert_eq!(
            slots(sections),
            [
                Slot::Header("queue-history"),
                Slot::Track(QueuePosition::Past(0))
            ]
        );
    }

    #[test]
    fn an_empty_queue_has_no_rows() {
        let sections = Sections {
            past: 0,
            current: false,
            upcoming: 0,
            similar: 0,
        };

        assert_eq!(sections.len(), 0);
        assert_eq!(sections.current_index(), None);
    }
}
