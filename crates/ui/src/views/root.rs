use gpui::prelude::FluentBuilder as _;
use gpui::{
    App, AppContext as _, Context, Entity, IntoElement, ParentElement, Render, Styled, Window, div,
};
use state::{Library, Session, SessionState};

use crate::theme::Theme;
use crate::views::{LibraryView, LoginView};

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

pub fn open_window(session: Entity<Session>, library: Entity<Library>, cx: &mut App) {
    use gpui::{Bounds, TitlebarOptions, WindowBounds, WindowOptions, point, px, size};

    let bounds = Bounds::centered(None, size(px(920.), px(640.)), cx);
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions {
                title: Some("spotty".into()),
                appears_transparent: false,
                traffic_light_position: Some(point(px(9.), px(9.))),
            }),
            app_id: Some("spotty".into()),
            window_min_size: Some(size(px(520.), px(400.))),
            ..Default::default()
        },
        |_, cx| cx.new(|cx| Root::new(session, library, cx)),
    )
    .expect("failed to open window");
}
