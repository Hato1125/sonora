use gpui::{App, Application, KeyBinding, Menu, MenuItem, actions};
use state::Spotty;

actions!(spotty, [Quit, SignOut, RefreshLibrary]);

fn main() {
    Application::new().run(|cx: &mut App| {
        ui::theme::init(cx);

        if let Err(error) = state::init(cx) {
            eprintln!("spotty: cannot start runtime: {error:#}");
            cx.quit();
            return;
        }

        register_actions(cx);

        let Spotty { session, library } = Spotty::global(cx);
        let (session, library) = (session.clone(), library.clone());

        ui::open_window(session.clone(), library, cx);
        session.update(cx, |session, cx| session.restore(cx));

        cx.activate(true);
    });
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
