mod player_bar;
mod sidebar;
mod title_bar;

pub use player_bar::PlayerBar;
pub use sidebar::Sidebar;
pub use title_bar::TitleBar;

use gpui::prelude::*;
use gpui::{AnyView, Context, Entity, Render};
use gpui::{Window, div};
use state::{Playback, Session};

pub struct Workspace {
    title_bar: Entity<TitleBar>,
    sidebar: Entity<Sidebar>,
    player_bar: Entity<PlayerBar>,
    content: AnyView,
}

impl Workspace {
    pub fn new(
        session: Entity<Session>,
        sidebar: Entity<Sidebar>,
        playback: Entity<Playback>,
        content: AnyView,
        cx: &mut Context<Self>,
    ) -> Self {
        let title_bar = cx.new(|cx| TitleBar::new(session, sidebar.clone(), cx));
        let player_bar = cx.new(|cx| PlayerBar::new(playback, cx));

        Self {
            title_bar,
            sidebar,
            player_bar,
            content,
        }
    }

    pub fn content(&self) -> &AnyView {
        &self.content
    }

    pub fn set_content(&mut self, content: AnyView, cx: &mut Context<Self>) {
        self.content = content;
        cx.notify();
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
