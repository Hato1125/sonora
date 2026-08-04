use gpui::prelude::*;
use gpui::{AnyView, Context, Entity, Render};
use gpui::{Window, div};
use gpui_component::ActiveTheme as _;
use state::{Library, Playback, Session, SessionState};
use workspace::{Destination, Sidebar, SidebarEvent, Workspace};

use crate::{LibraryToolbar, LibraryView, LoginView, SettingsView};

struct Screens {
    library: Entity<LibraryView>,
    library_toolbar: Entity<LibraryToolbar>,
    settings: Entity<SettingsView>,
}

pub struct Root {
    session: Entity<Session>,
    login: Entity<LoginView>,
    workspace: Entity<Workspace>,
    screens: Screens,
    at: Destination,
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
        let sidebar = cx.new(|cx| Sidebar::new(library.clone(), cx));

        cx.subscribe(&sidebar, |this, _, event, cx| {
            let SidebarEvent::Navigate(destination) = event;
            this.go(*destination, cx);
        })
        .detach();

        let library_view =
            cx.new(|cx| LibraryView::new(library, playback.clone(), sidebar.clone(), window, cx));
        let library_toolbar = cx.new(|cx| LibraryToolbar::new(library_view.clone(), cx));
        let settings = cx.new(|cx| SettingsView::new(session.clone(), playback.clone(), cx));

        let workspace =
            cx.new(|cx| Workspace::new(sidebar, playback, library_view.clone().into(), cx));
        workspace.update(cx, |workspace, cx| {
            workspace.set_toolbar(Some(library_toolbar.clone().into()), cx);
        });

        Self {
            session,
            login,
            workspace,
            screens: Screens {
                library: library_view,
                library_toolbar,
                settings,
            },
            at: Destination::Library,
        }
    }

    fn go(&mut self, destination: Destination, cx: &mut Context<Self>) {
        if self.at == destination {
            return;
        }
        self.at = destination;

        let (content, toolbar): (AnyView, Option<AnyView>) = match destination {
            Destination::Library => (
                self.screens.library.clone().into(),
                Some(self.screens.library_toolbar.clone().into()),
            ),
            Destination::Settings => (self.screens.settings.clone().into(), None),
        };

        self.workspace.update(cx, |workspace, cx| {
            workspace.set_content(content, cx);
            workspace.set_toolbar(toolbar, cx);
        });
        cx.notify();
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
