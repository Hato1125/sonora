use gpui::{App, Menu, MenuItem};
use input::{Quit, RefreshLibrary, SignOut, TogglePlayback};
use state::Spotty;

pub fn register(cx: &mut App) {
    cx.bind_keys(input::bindings());

    cx.on_action(|_: &Quit, cx: &mut App| cx.quit());

    cx.on_window_closed(|cx, _| {
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

    cx.on_action(|_: &TogglePlayback, cx: &mut App| {
        let playback = Spotty::global(cx).playback.clone();
        playback.update(cx, |playback, cx| playback.toggle_play(cx));
    });

    cx.set_menus(vec![Menu {
        name: "spotty".into(),
        disabled: false,
        items: vec![
            MenuItem::action("Refresh Library", RefreshLibrary),
            MenuItem::action("Sign Out", SignOut),
            MenuItem::separator(),
            MenuItem::action("Quit", Quit),
        ],
    }]);
}
