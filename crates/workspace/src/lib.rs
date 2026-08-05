mod navigation;
mod player_bar;
mod sidebar;
mod title_bar;

pub use navigation::{Navigation, NavigationEvent};
pub use player_bar::PlayerBar;
pub use sidebar::{Destination, LibraryTab, Sidebar, SidebarEvent};
pub use title_bar::TitleBar;

use gpui::prelude::*;
use gpui::{AnyView, App, Context, Entity, FocusHandle, Render};
use gpui::{Window, div};
use input::WORKSPACE_CONTEXT;
use state::{Playback, Queue};

pub struct Workspace {
    title_bar: Entity<TitleBar>,
    sidebar: Entity<Sidebar>,
    player_bar: Entity<PlayerBar>,
    content: AnyView,
    focus: FocusHandle,
}

impl Workspace {
    pub fn new(
        sidebar: Entity<Sidebar>,
        navigation: Entity<Navigation>,
        playback: Entity<Playback>,
        queue: Entity<Queue>,
        content: AnyView,
        cx: &mut Context<Self>,
    ) -> Self {
        let title_bar = cx.new(|cx| TitleBar::new(sidebar.clone(), navigation, cx));
        let player_bar = cx.new(|cx| PlayerBar::new(playback, queue, cx));

        Self {
            title_bar,
            sidebar,
            player_bar,
            content,
            focus: cx.focus_handle(),
        }
    }

    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        window.focus(&self.focus, cx);
    }

    pub fn content(&self) -> &AnyView {
        &self.content
    }

    pub fn set_content(&mut self, content: AnyView, cx: &mut Context<Self>) {
        self.content = content;
        cx.notify();
    }

    pub fn set_toolbar(&mut self, toolbar: Option<AnyView>, cx: &mut Context<Self>) {
        self.title_bar
            .update(cx, |bar, cx| bar.set_content(toolbar, cx));
    }

    pub fn player_bar(&self) -> &Entity<PlayerBar> {
        &self.player_bar
    }
}

impl Render for Workspace {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .key_context(WORKSPACE_CONTEXT)
            .track_focus(&self.focus)
            .child(self.title_bar.clone())
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .child(self.sidebar.clone())
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .child(self.content.clone()),
                    ),
            )
            .child(self.player_bar.clone())
    }
}
