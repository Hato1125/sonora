mod assets;
mod http;

use std::sync::Arc;

use gpui::{
    App, AppContext as _, Application, Bounds, Entity, KeyBinding, Menu, MenuItem, TitlebarOptions,
    WindowBounds, WindowOptions, actions, point, px, size,
};
use state::{Library, Playback, Session, Spotty};
use views::Root;

actions!(spotty, [Quit, SignOut, RefreshLibrary]);

fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("warn,symphonia=error"),
    )
    .format_timestamp(None)
    .format_module_path(false)
    .init();

    let io = match state::Io::new() {
        Ok(io) => io,
        Err(error) => {
            eprintln!("spotty: cannot start runtime: {error:#}");
            return;
        }
    };

    Application::new()
        .with_assets(assets::Assets)
        .with_http_client(Arc::new(http::Client::new(io.handle())))
        .run(move |cx: &mut App| {
            gpui_component::init(cx);
            gpui_component::Theme::change(gpui_component::ThemeMode::Dark, None, cx);
            gpui_component::Theme::global_mut(cx).font_size = px(13.);

            state::init(cx, io);

            register_actions(cx);

            let Spotty {
                session,
                library,
                playback,
            } = Spotty::global(cx);
            let (session, library, playback) = (session.clone(), library.clone(), playback.clone());

            open_window(session.clone(), library, playback, cx);
            session.update(cx, |session, cx| session.restore(cx));

            cx.activate(true);
        });
}

fn open_window(
    session: Entity<Session>,
    library: Entity<Library>,
    playback: Entity<Playback>,
    cx: &mut App,
) {
    let bounds = Bounds::centered(None, size(px(920.), px(640.)), cx);
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions {
                title: Some("spotty".into()),
                appears_transparent: true,
                traffic_light_position: Some(point(px(9.), px(9.))),
            }),
            is_movable: true,
            is_resizable: true,
            app_id: Some("spotty".into()),
            window_min_size: Some(size(px(480.), px(400.))),
            ..Default::default()
        },
        |window, cx| {
            let root = cx.new(|cx| Root::new(session, library, playback, window, cx));
            let view: gpui::AnyView = root.into();
            cx.new(|cx| gpui_component::Root::new(view, window, cx))
        },
    )
    .expect("failed to open window");
}

fn register_actions(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("ctrl-q", Quit, None),
        KeyBinding::new("cmd-r", RefreshLibrary, None),
        KeyBinding::new("ctrl-r", RefreshLibrary, None),
    ]);

    cx.on_action(|_: &Quit, cx: &mut App| cx.quit());

    cx.on_window_closed(|cx| {
        if cx.windows().is_empty() {
            cx.quit();
        }
    })
    .detach();

    cx.on_action(|_: &SignOut, cx: &mut App| {
        let session = Spotty::global(cx).session.clone();
        session.update(cx, |session, cx| session.sign_out(cx));
    });

    cx.on_action(|_: &RefreshLibrary, cx: &mut App| {
        let library = Spotty::global(cx).library.clone();
        library.update(cx, |library, cx| library.refresh(cx));
    });

    cx.set_menus(vec![Menu {
        name: "spotty".into(),
        items: vec![
            MenuItem::action("Refresh Library", RefreshLibrary),
            MenuItem::action("Sign Out", SignOut),
            MenuItem::separator(),
            MenuItem::action("Quit", Quit),
        ],
    }]);
}
