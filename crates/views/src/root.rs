use gpui::prelude::*;
use gpui::{AnyView, Context, Entity, Render};
use gpui::{Window, div};
use state::{Detail, Io, Library, Playback, Queue, Session, SessionState};
use ui::ActiveTheme as _;
use workspace::{
    Destination, LibraryTab, Navigation, NavigationEvent, Sidebar, SidebarEvent, Workspace,
};

use crate::tracks::{ALBUM_COLUMNS, LIBRARY_COLUMNS};
use crate::{DetailView, LibraryToolbar, LibraryView, LoginView, SettingsView};

struct Screens {
    library: Entity<LibraryView>,
    library_toolbar: Entity<LibraryToolbar>,
    album: Entity<DetailView>,
    album_detail: Entity<Detail>,
    playlist: Entity<DetailView>,
    playlist_detail: Entity<Detail>,
    settings: Entity<SettingsView>,
}

pub struct Root {
    session: Entity<Session>,
    login: Entity<LoginView>,
    workspace: Entity<Workspace>,
    screens: Screens,
}

impl Root {
    pub fn new(
        session: Entity<Session>,
        library: Entity<Library>,
        playback: Entity<Playback>,
        queue: Entity<Queue>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&session, |_, _, cx| cx.notify()).detach();

        let login = cx.new(|cx| LoginView::new(session.clone(), cx));
        let sidebar = cx.new(Sidebar::new);

        let navigation = cx.new(|_| Navigation::new(Destination::Library(LibraryTab::Songs)));

        cx.subscribe(&sidebar, {
            let navigation = navigation.clone();
            move |_, _, event, cx| {
                let SidebarEvent::Navigate(destination) = event;
                let destination = destination.clone();
                navigation.update(cx, |navigation, cx| navigation.go(destination, cx));
            }
        })
        .detach();

        cx.subscribe(&navigation, |this, _, event, cx| {
            let NavigationEvent::Moved(destination) = event;
            this.show(destination.clone(), cx);
        })
        .detach();

        let library_view = cx.new(|cx| {
            LibraryView::new(
                library.clone(),
                playback.clone(),
                navigation.clone(),
                sidebar.clone(),
                window,
                cx,
            )
        });
        let library_toolbar =
            cx.new(|cx| LibraryToolbar::new(library_view.clone(), navigation.clone(), cx));

        let io = Io::global(cx);
        let album_detail =
            cx.new(|cx| Detail::new(session.clone(), library.clone(), io.clone(), cx));
        let album = cx.new(|cx| {
            DetailView::new(
                album_detail.clone(),
                playback.clone(),
                sidebar.clone(),
                navigation.clone(),
                ALBUM_COLUMNS,
                window,
                cx,
            )
        });

        let playlist_detail = cx.new(|cx| Detail::new(session.clone(), library, io, cx));
        let playlist = cx.new(|cx| {
            DetailView::new(
                playlist_detail.clone(),
                playback.clone(),
                sidebar.clone(),
                navigation.clone(),
                LIBRARY_COLUMNS,
                window,
                cx,
            )
        });

        let settings = cx.new(|cx| SettingsView::new(session.clone(), playback.clone(), cx));

        let start = navigation.read(cx).current();
        let workspace = cx.new(|cx| {
            Workspace::new(
                sidebar,
                navigation,
                playback,
                queue,
                library_view.clone().into(),
                cx,
            )
        });

        let mut root = Self {
            session,
            login,
            workspace,
            screens: Screens {
                library: library_view,
                library_toolbar,
                album,
                album_detail,
                playlist,
                playlist_detail,
                settings,
            },
        };
        root.show(start, cx);
        root
    }

    fn show(&mut self, destination: Destination, cx: &mut Context<Self>) {
        let (content, toolbar): (AnyView, Option<AnyView>) = match destination {
            Destination::Library(tab) => {
                self.screens
                    .library
                    .update(cx, |library, cx| library.select(tab.into(), cx));
                (
                    self.screens.library.clone().into(),
                    Some(self.screens.library_toolbar.clone().into()),
                )
            }
            Destination::Album(id) => {
                self.screens
                    .album_detail
                    .update(cx, |detail, cx| detail.open_album(&id, cx));
                (self.screens.album.clone().into(), None)
            }
            Destination::Playlist(id) => {
                self.screens
                    .playlist_detail
                    .update(cx, |detail, cx| detail.open_playlist(&id, cx));
                (self.screens.playlist.clone().into(), None)
            }
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
