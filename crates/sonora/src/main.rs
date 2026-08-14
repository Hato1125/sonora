#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod actions;
mod assets;
mod http;
mod logging;
mod memory;
mod single;

use std::sync::Arc;

use gpui::{
    App, AppContext as _, Bounds, Entity, TitlebarOptions, WindowBackgroundAppearance,
    WindowBounds, WindowOptions, point, px, size,
};
use music::LyricsProvider;
use router::Screen;
use state::{Library, Playback, Queue, Session, Sonora};
use ui::ActiveTheme as _;
use views::Root;

fn main() {
    logging::init();

    let opened = std::env::args().skip(1).find(|arg| !arg.starts_with('-'));
    let (sender, mut links) = tokio::sync::mpsc::unbounded_channel();
    if let single::Instance::Running = single::claim(opened.as_deref(), sender.clone()) {
        return;
    }
    let opened_start = opened.as_deref().and_then(router::destination);

    let io = match state::Io::new() {
        Ok(io) => io,
        Err(error) => {
            eprintln!("sonora: cannot start runtime: {error:#}");
            return;
        }
    };

    let app = gpui_platform::application()
        .with_assets(assets::Assets)
        .with_http_client(Arc::new(http::Client::new(io.handle())));
    app.on_open_urls(move |opened| {
        for link in opened {
            sender.send(link).ok();
        }
    });

    app.run(move |cx: &mut App| {
        if let Err(error) = assets::Assets.load_fonts(cx) {
            log::error!("sonora: cannot load bundled fonts: {error:#}");
        }

        let providers: Vec<Arc<dyn music::MusicProvider>> = vec![
            Arc::new(music::spotify::SpotifyProvider::from_env()),
            Arc::new(music::youtube::YouTubeProvider::new()),
        ];
        let local_provider: Arc<dyn music::MusicProvider> =
            Arc::new(music::local::LocalProvider::new(
                dirs::config_dir()
                    .unwrap_or_else(std::env::temp_dir)
                    .join("sonora"),
            ));
        let lyrics: Vec<Arc<dyn LyricsProvider>> = vec![Arc::new(music::lrclib::LrcLib::new())];
        state::init(cx, io, providers, local_provider, lyrics);
        let start = opened_start.unwrap_or_else(|| {
            let startup = Sonora::global(cx).settings.read(cx).startup().to_owned();
            Screen::from_id(&startup)
                .unwrap_or(Screen::Home)
                .destination()
        });
        router::init(start, cx);
        let (look, overrides, language) = {
            let settings = Sonora::global(cx).settings.read(cx);
            (
                settings.look(),
                settings.theme_overrides().clone(),
                settings.language().to_owned(),
            )
        };
        i18n::set(i18n::resolve(&language));
        ui::Theme::init(look, &overrides, cx);

        actions::register(cx);
        memory::watch(cx);

        let Sonora {
            session,
            library,
            lyrics: _,
            playback,
            queue,
            settings: _,
        } = Sonora::global(cx);
        let (session, library, playback, queue) = (
            session.clone(),
            library.clone(),
            playback.clone(),
            queue.clone(),
        );

        open_window(session.clone(), library, playback, queue, cx);
        session.update(cx, |session, cx| session.restore(cx));

        cx.spawn(async move |cx| {
            while let Some(link) = links.recv().await {
                cx.update(|cx| follow(&link, cx));
            }
        })
        .detach();

        cx.activate(true);
    });
}

fn follow(link: &str, cx: &mut App) {
    if let Some(destination) = router::destination(link) {
        router::navigate(destination, cx);
    }
    cx.activate(true);
}

fn open_window(
    session: Entity<Session>,
    library: Entity<Library>,
    playback: Entity<Playback>,
    queue: Entity<Queue>,
    cx: &mut App,
) {
    let bounds = Bounds::centered(None, size(px(920.), px(640.)), cx);
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            window_background: WindowBackgroundAppearance::Transparent,
            titlebar: Some(TitlebarOptions {
                title: Some("Sonora".into()),
                appears_transparent: true,
                traffic_light_position: Some(point(
                    px(9.),
                    px(if cfg!(target_os = "macos") { 12. } else { 9. }),
                )),
            }),
            inactive_frame_interval: None,
            is_movable: true,
            is_resizable: true,
            app_id: Some("sonora".into()),
            window_min_size: Some(size(px(480.), px(400.))),
            ..Default::default()
        },
        |window, cx| {
            window.set_rem_size(cx.theme().font_size);
            state::attach_remote(window_handle(window), cx);
            cx.new(|cx| Root::new(session, library, playback, queue, window, cx))
        },
    )
    .expect("failed to open window");
}

#[cfg(target_os = "windows")]
fn window_handle(window: &gpui::Window) -> Option<*mut std::ffi::c_void> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    match HasWindowHandle::window_handle(window).ok()?.as_raw() {
        RawWindowHandle::Win32(handle) => Some(handle.hwnd.get() as *mut std::ffi::c_void),
        _ => None,
    }
}

#[cfg(not(target_os = "windows"))]
fn window_handle(_window: &gpui::Window) -> Option<*mut std::ffi::c_void> {
    None
}
