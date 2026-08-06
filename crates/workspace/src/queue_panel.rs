use gpui::prelude::*;
use gpui::{
    Context, DragMoveEvent, Entity, FontWeight, MouseButton, MouseDownEvent, Pixels, Point, Render,
    ScrollStrategy, SharedString, UniformListScrollHandle, Window, div, px, uniform_list,
};
use spotify::Track;
use state::{Playback, Queue};
use ui::{ActiveTheme as _, Card, Menu, MenuItem, snapped};

use crate::Sidebar;

const PANEL_WIDTH: f32 = 380.;
const FULLSCREEN_BREAKPOINT: f32 = 680.;

fn fills_content(width: Pixels) -> bool {
    width < px(FULLSCREEN_BREAKPOINT)
}

#[derive(Clone)]
struct DraggedTrack {
    index: usize,
    revision: u64,
    name: SharedString,
    position: Point<Pixels>,
}

impl DraggedTrack {
    fn at(mut self, position: Point<Pixels>) -> Self {
        self.position = position;
        self
    }
}

#[derive(Clone, Copy)]
enum QueuePosition {
    Past(usize),
    Current,
    Upcoming(usize),
}

/// Which edge of a row the drop indicator line is drawn at.
#[derive(Clone, Copy)]
enum DropLine {
    Above,
    Below,
}

#[derive(Clone, Copy)]
enum MenuPlacement {
    Above,
    Below,
}

#[derive(Clone, Copy)]
struct ContextMenuState {
    index: usize,
    revision: u64,
    placement: MenuPlacement,
}

/// Transient per-row decorations.
#[derive(Clone, Copy, Default)]
struct RowState {
    menu: Option<MenuPlacement>,
    drop_line: Option<DropLine>,
}

impl QueuePosition {
    fn past(self) -> Option<usize> {
        match self {
            Self::Past(index) => Some(index),
            Self::Current | Self::Upcoming(_) => None,
        }
    }

    fn upcoming(self) -> Option<usize> {
        match self {
            Self::Upcoming(index) => Some(index),
            Self::Past(_) | Self::Current => None,
        }
    }

    fn label(self) -> Option<&'static str> {
        match self {
            Self::Past(_) => None,
            Self::Current => Some("Playing"),
            Self::Upcoming(_) => None,
        }
    }
}

impl Render for DraggedTrack {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.theme();

        div()
            .pl(self.position.x + px(8.))
            .pt(self.position.y + px(8.))
            .child(
                div()
                    .max_w(px(240.))
                    .px_2()
                    .py_1()
                    .rounded(theme.radius)
                    .bg(theme.secondary)
                    .text_color(theme.foreground)
                    .truncate()
                    .child(self.name.clone()),
            )
    }
}

pub(crate) struct QueuePanel {
    queue: Entity<Queue>,
    playback: Entity<Playback>,
    sidebar: Entity<Sidebar>,
    context_menu: Option<ContextMenuState>,
    drop_gap: Option<usize>,
    scroll: UniformListScrollHandle,
    scroll_to_playing: bool,
    open: bool,
}

impl QueuePanel {
    pub(crate) fn new(
        queue: Entity<Queue>,
        playback: Entity<Playback>,
        sidebar: Entity<Sidebar>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&queue, |this, queue, cx| {
            let revision = queue.read(cx).revision();
            if this
                .context_menu
                .is_some_and(|menu| menu.revision != revision)
            {
                this.context_menu = None;
            }
            cx.notify();
        })
        .detach();
        cx.observe(&sidebar, |_, _, cx| cx.notify()).detach();

        Self {
            queue,
            playback,
            sidebar,
            context_menu: None,
            drop_gap: None,
            scroll: UniformListScrollHandle::new(),
            scroll_to_playing: false,
            open: false,
        }
    }

    pub(crate) fn is_open(&self) -> bool {
        self.open
    }

    pub(crate) fn toggle(&mut self, cx: &mut Context<Self>) {
        self.open = !self.open;
        self.scroll_to_playing = self.open;
        cx.notify();
    }

    pub(crate) fn close(&mut self, cx: &mut Context<Self>) {
        self.context_menu = None;
        self.open = false;
        cx.notify();
    }

    fn dismiss_menu(&mut self, cx: &mut Context<Self>) {
        self.context_menu = None;
        cx.notify();
    }

    fn row(
        track: Track,
        index: usize,
        position: QueuePosition,
        queue_revision: u64,
        state: RowState,
        window: &Window,
        cx: &Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let theme = *cx.theme();
        let height = snapped(theme.metrics.list_row, window);
        let past_index = position.past();
        let queue_index = position.upcoming();
        let title = match position {
            QueuePosition::Past(_) => theme.muted_foreground,
            QueuePosition::Current => theme.primary,
            QueuePosition::Upcoming(_) => theme.foreground,
        };
        let dragged = queue_index.map(|index| DraggedTrack {
            index,
            revision: queue_revision,
            name: SharedString::from(track.name.clone()),
            position: Point::default(),
        });

        let card = Card::new(("queue-track", index), SharedString::from(track.name))
            .cover(track.cover)
            .meta(SharedString::from(track.artists))
            .weight(FontWeight::SEMIBOLD)
            .tint(title)
            .when(track.explicit, Card::explicit)
            .when_some(position.label(), |this, label| {
                this.trailing(
                    div()
                        .flex_none()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(label),
                )
            })
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
                .on_drag_move(cx.listener(
                    move |this, event: &DragMoveEvent<DraggedTrack>, _, cx| {
                        let position = event.event.position;
                        if !event.bounds.contains(&position) {
                            return;
                        }
                        let gap = if position.y < event.bounds.center().y {
                            target
                        } else {
                            target + 1
                        };
                        let dragged = event.drag(cx).index;
                        let gap = (gap != dragged && gap != dragged + 1).then_some(gap);
                        if this.drop_gap != gap {
                            this.drop_gap = gap;
                            cx.notify();
                        }
                    },
                ))
                .on_drop(cx.listener(move |this, dragged: &DraggedTrack, _, cx| {
                    if let Some(gap) = this.drop_gap.take() {
                        this.queue.update(cx, |queue, cx| {
                            if queue.revision() == dragged.revision {
                                queue.move_upcoming_to_gap(dragged.index, gap, cx);
                            }
                        });
                    }
                }))
                .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                        let placement = if event.position.y > window.viewport_size().height / 2. {
                            MenuPlacement::Above
                        } else {
                            MenuPlacement::Below
                        };
                        this.context_menu = Some(ContextMenuState {
                            index: target,
                            revision: queue_revision,
                            placement,
                        });
                        cx.stop_propagation();
                        cx.notify();
                    }),
                )
            })
            .when_some(dragged, |this, dragged| {
                this.on_drag(dragged, |dragged, position, _, cx| {
                    cx.new(|_| dragged.clone().at(position))
                })
            });

        div()
            .id(("queue-track-container", index))
            .relative()
            .min_w_0()
            .child(card)
            .when_some(state.drop_line, |this, edge| {
                let line = div()
                    .absolute()
                    .left_2()
                    .right_2()
                    .h(px(2.))
                    .rounded_full()
                    .bg(theme.primary);
                this.child(match edge {
                    DropLine::Above => line.top_0(),
                    DropLine::Below => line.bottom_0(),
                })
            })
            .when_some(state.menu.zip(queue_index), |this, (placement, index)| {
                this.child(
                    Menu::new(("queue-track-menu", index))
                        .right_2()
                        .w(px(180.))
                        .when_else(
                            matches!(placement, MenuPlacement::Above),
                            |menu| menu.bottom(height - px(4.)),
                            |menu| menu.top(height - px(4.)),
                        )
                        .on_dismiss(cx.listener(|this, _, _, cx| this.dismiss_menu(cx)))
                        .item(
                            MenuItem::new(("remove-queued-track", index), "Remove from queue")
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.queue.update(cx, |queue, cx| {
                                        if queue.revision() == queue_revision {
                                            queue.remove_upcoming(index, cx);
                                        }
                                    });
                                    this.dismiss_menu(cx);
                                })),
                        ),
                )
            })
    }
}

impl Render for QueuePanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.open {
            return div().into_any_element();
        }

        let theme = *cx.theme();
        let content_width = window.viewport_size().width - self.sidebar.read(cx).occupied_width();
        let fullscreen = fills_content(content_width);
        let queue = self.queue.read(cx);
        let past_count = queue.past().len();
        let has_current = queue.current().is_some();
        let current_count = usize::from(has_current);
        let upcoming_count = queue.upcoming().len();
        let total_count = past_count + current_count + upcoming_count;
        let context_menu = self.context_menu;
        let empty = total_count == 0;
        if !cx.has_active_drag() {
            self.drop_gap = None;
        }
        let drop_gap = self.drop_gap;

        if self.scroll_to_playing && has_current {
            self.scroll
                .scroll_to_item(past_count, ScrollStrategy::Center);
            self.scroll_to_playing = false;
        }

        div()
            .id("queue-panel")
            .occlude()
            .on_drag_move(cx.listener(|this, _: &DragMoveEvent<DraggedTrack>, _, cx| {
                if this.drop_gap.take().is_some() {
                    cx.notify();
                }
            }))
            .absolute()
            .top_0()
            .right_0()
            .bottom_0()
            .flex()
            .flex_col()
            .bg(theme.background)
            .border_l_1()
            .border_color(theme.border)
            .when(fullscreen, |this| this.left_0())
            .when(!fullscreen, |this| this.w(px(PANEL_WIDTH)))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h_0()
                    .p_2()
                    .when(!empty, |this| {
                        let queue = self.queue.clone();
                        this.child(
                            uniform_list(
                                "queue-tracks",
                                total_count,
                                cx.processor(
                                    move |_, range: std::ops::Range<usize>, window, cx| {
                                        let (revision, visible) = {
                                            let queue = queue.read(cx);
                                            let visible = queue
                                                .past()
                                                .cloned()
                                                .enumerate()
                                                .map(|(index, track)| {
                                                    (track, QueuePosition::Past(index))
                                                })
                                                .chain(
                                                    queue.current().cloned().map(|track| {
                                                        (track, QueuePosition::Current)
                                                    }),
                                                )
                                                .chain(queue.upcoming().cloned().enumerate().map(
                                                    |(index, track)| {
                                                        (track, QueuePosition::Upcoming(index))
                                                    },
                                                ))
                                                .skip(range.start)
                                                .take(range.len())
                                                .collect::<Vec<_>>();
                                            (queue.revision(), visible)
                                        };
                                        visible
                                            .into_iter()
                                            .enumerate()
                                            .map(|(offset, (track, position))| {
                                                let row_state = RowState {
                                                    menu: position.upcoming().and_then(|index| {
                                                        context_menu
                                                            .filter(|menu| {
                                                                menu.index == index
                                                                    && menu.revision == revision
                                                            })
                                                            .map(|menu| menu.placement)
                                                    }),
                                                    drop_line: match (position.upcoming(), drop_gap)
                                                    {
                                                        (Some(index), Some(gap))
                                                            if gap == index =>
                                                        {
                                                            Some(DropLine::Above)
                                                        }
                                                        (Some(index), Some(gap))
                                                            if gap == upcoming_count
                                                                && index + 1 == upcoming_count =>
                                                        {
                                                            Some(DropLine::Below)
                                                        }
                                                        _ => None,
                                                    },
                                                };
                                                Self::row(
                                                    track,
                                                    range.start + offset,
                                                    position,
                                                    revision,
                                                    row_state,
                                                    window,
                                                    cx,
                                                )
                                            })
                                            .collect()
                                    },
                                ),
                            )
                            .track_scroll(&self.scroll)
                            .flex_1()
                            .min_h_0(),
                        )
                    })
                    .when(empty, |this| {
                        this.child(
                            div()
                                .flex()
                                .flex_1()
                                .items_center()
                                .justify_center()
                                .text_color(theme.muted_foreground)
                                .child("Your queue is empty"),
                        )
                    }),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use gpui::px;

    use super::fills_content;

    #[test]
    fn fills_narrow_content_area() {
        assert!(fills_content(px(679.)));
        assert!(!fills_content(px(680.)));
    }
}
