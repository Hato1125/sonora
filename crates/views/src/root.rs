use gpui::{AppContext as _, Context, Entity, Render};
use state::{Library, Session, SessionState};
use ui::prelude::*;

use crate::{LibraryView, LoginView};

pub struct Root {
    session: Entity<Session>,
    login: Entity<LoginView>,
    library: Entity<LibraryView>,
}

impl Root {
    pub fn new(session: Entity<Session>, library: Entity<Library>, cx: &mut Context<Self>) -> Self {
        cx.observe(&session, |_, _, cx| cx.notify()).detach();

        let login = cx.new(|cx| LoginView::new(session.clone(), cx));
        let library = cx.new(|cx| LibraryView::new(session.clone(), library, cx));

        Self {
            session,
            login,
            library,
        }
    }
}

impl Render for Root {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::global(cx).clone();
        let signed_in = matches!(self.session.read(cx).state(), SessionState::SignedIn(_));

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.background)
            .text_color(theme.text)
            .when_else(
                signed_in,
                |this| this.child(self.library.clone()),
                |this| this.child(self.login.clone()),
            )
    }
}
