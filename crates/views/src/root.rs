use gpui::prelude::*;
use gpui::{Context, Entity, Render};
use gpui::{Window, div};
use gpui_component::ActiveTheme as _;
use state::{Library, Playback, Session, SessionState};
use workspace::Workspace;

use crate::{LibraryView, LoginView};

pub struct Root {
    session: Entity<Session>,
    login: Entity<LoginView>,
    workspace: Entity<Workspace>,
}

impl Root {
    pub fn new(
        session: Entity<Session>,
        library: Entity<Library>,
        playback: Entity<Playback>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&session, |_, _, cx| cx.notify()).detach();

        let login = cx.new(|cx| LoginView::new(session.clone(), cx));
        let library_view =
            cx.new(|cx| LibraryView::new(library.clone(), playback.clone(), window, cx));
        let workspace = cx
            .new(|cx| Workspace::new(session.clone(), library, playback, library_view.into(), cx));

        Self {
            session,
            login,
            workspace,
        }
    }
}

impl Render for Root {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let show_sign_in = match self.session.read(cx).state() {
            SessionState::SignedOut | SessionState::Failed(_) | SessionState::Authorizing => true,
            SessionState::Restoring | SessionState::SignedIn(_) => false,
        };

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.background)
            .text_color(theme.foreground)
            .when_else(
                show_sign_in,
                |this| this.child(self.login.clone()),
                |this| this.child(self.workspace.clone()),
            )
    }
}
