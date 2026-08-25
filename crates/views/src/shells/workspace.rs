use std::time::{Duration, Instant};

use gpui::prelude::*;
use gpui::{AnyView, App, Context, Entity, FocusHandle, Render, px};
use gpui::{Window, div};
use input::WORKSPACE_CONTEXT;
use state::{Playback, Queue, SideTab};
use ui::{Motion, ease_out_expo};

use crate::chrome::{Chrome, PlayerBar, SidebarLeft, SidebarRight, TitleBarOptions, ToastStack};
use crate::shared::playlist_editor::PlaylistEditor;
use crate::shells::Shell;

const VIEW_BLUR: gpui::Pixels = px(1.5);
const VIEW_DURATION_EXTRA: Duration = Duration::from_millis(50);
const VIEW_ZOOM: f32 = 0.01;

#[derive(Clone, Copy)]
struct ContentTransition {
    from: f32,
    to: f32,
    started: Instant,
    span: Duration,
}

impl ContentTransition {
    fn hidden(self) -> f32 {
        if self.span.is_zero() {
            return self.to;
        }
        let elapsed = self.started.elapsed().as_secs_f32();
        let progress = (elapsed / self.span.as_secs_f32()).clamp(0., 1.);
        self.from + (self.to - self.from) * ease_out_expo(progress)
    }

    fn running(self) -> bool {
        self.started.elapsed() < self.span
    }
}

pub(crate) struct Workspace {
    sidebar: Entity<SidebarLeft>,
    player_bar: Entity<PlayerBar>,
    sidebar_right: Entity<SidebarRight>,
    playlist_editor: Entity<PlaylistEditor>,
    toasts: Entity<ToastStack>,
    content: AnyView,
    transition: Option<ContentTransition>,
    focus: FocusHandle,
}

impl Workspace {
    pub fn new(
        playback: Entity<Playback>,
        queue: Entity<Queue>,
        content: AnyView,
        cx: &mut Context<Self>,
    ) -> Self {
        let sidebar = cx.new(SidebarLeft::new);
        let sidebar_right = cx.new(|cx| SidebarRight::new(queue.clone(), playback.clone(), cx));
        let player_bar = cx.new(|cx| PlayerBar::new(playback, queue, cx));

        Self {
            sidebar,
            player_bar,
            sidebar_right,
            playlist_editor: PlaylistEditor::entity(cx),
            toasts: cx.new(ToastStack::new),
            content,
            transition: None,
            focus: cx.focus_handle(),
        }
    }

    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        window.focus(&self.focus, cx);
    }

    pub fn toggle_sidebar(&self, cx: &mut Context<Self>) {
        self.sidebar.update(cx, |sidebar, cx| sidebar.toggle(cx));
    }

    pub fn toggle_sidebar_right(&self, cx: &mut Context<Self>) {
        self.sidebar_right.update(cx, |panel, cx| panel.toggle(cx));
    }

    pub fn show_side(&self, tab: SideTab, cx: &mut Context<Self>) {
        self.sidebar_right
            .update(cx, |panel, cx| panel.show(tab, cx));
    }

    #[allow(dead_code)]
    pub fn content(&self) -> &AnyView {
        &self.content
    }

    pub fn set_content(&mut self, content: AnyView, cx: &mut Context<Self>) {
        self.content = content;
        cx.notify();
    }

    pub fn reveal_content(&mut self, cx: &mut Context<Self>) -> Duration {
        if cx.reduce_motion() {
            self.transition = None;
            return Duration::ZERO;
        }

        let from = self
            .transition
            .filter(|transition| transition.running())
            .map(ContentTransition::hidden)
            .unwrap_or(1.);
        self.transition_from(from, 0., cx)
    }

    pub fn finish_transition(&mut self, cx: &mut Context<Self>) {
        if self.transition.take().is_some() {
            cx.notify();
        }
    }

    fn transition_from(&mut self, from: f32, to: f32, cx: &mut Context<Self>) -> Duration {
        let distance = (to - from).abs();
        if distance <= f32::EPSILON {
            return Duration::ZERO;
        }

        let span = (Motion::Base.span() + VIEW_DURATION_EXTRA).mul_f32(distance);
        self.transition = Some(ContentTransition {
            from,
            to,
            started: Instant::now(),
            span,
        });
        cx.notify();
        span
    }

    fn hidden(&mut self, window: &mut Window, cx: &Context<Self>) -> f32 {
        if cx.reduce_motion() {
            self.transition = None;
            return 0.;
        }
        let Some(transition) = self.transition else {
            return 0.;
        };
        if transition.running() {
            window.request_animation_frame();
        }
        transition.hidden()
    }

    #[allow(dead_code)]
    pub fn player_bar(&self) -> &Entity<PlayerBar> {
        &self.player_bar
    }
}

impl Shell for Workspace {
    fn title_bar(&self, content: Option<AnyView>, cx: &App) -> TitleBarOptions {
        let sidebar = self.sidebar.read(cx);

        TitleBarOptions {
            navigation: true,
            sidebar_open: sidebar.is_open(),
            sidebar_right: Some(self.sidebar_right.read(cx).is_open()),
            offset: sidebar.occupied_width(),
            border: true,
            content,
        }
    }
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sidebar
            .update(cx, |sidebar, cx| sidebar.adapt(window, cx));
        let left = self.sidebar.read(cx).occupied_width();
        let right = self.sidebar_right.read(cx).occupied_width(window);
        Chrome::publish(left, right, cx);
        let covered = self.sidebar_right.read(cx).covers_content(window);
        let overlay = self.sidebar.read(cx).overlays();
        let hidden = self.hidden(window, cx);
        let scale = 1. - VIEW_ZOOM * hidden;

        div()
            .relative()
            .flex()
            .flex_col()
            .w_full()
            .flex_1()
            .min_h_0()
            .key_context(WORKSPACE_CONTEXT)
            .track_focus(&self.focus)
            .child(
                div()
                    .relative()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .when(!overlay, |this| this.child(self.sidebar.clone()))
                    .child(
                        div()
                            .relative()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .min_h_0()
                            .when(hidden > 0., |this| this.overflow_hidden())
                            .when(covered, |this| this.hidden())
                            .child(
                                div()
                                    .absolute()
                                    .left_0()
                                    .right_0()
                                    .top_0()
                                    .bottom_0()
                                    .flex()
                                    .flex_col()
                                    .layer_scale(scale)
                                    .opacity(1. - hidden)
                                    .blur(VIEW_BLUR * hidden)
                                    .child(self.content.clone()),
                            ),
                    )
                    .child(self.sidebar_right.clone())
                    .when(overlay, |this| this.child(self.sidebar.clone())),
            )
            .child(
                div()
                    .relative()
                    .child(self.player_bar.clone())
                    .child(self.toasts.clone()),
            )
            .child(self.playlist_editor.clone())
    }
}
