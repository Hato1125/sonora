mod artist;
mod catalog;
mod cover;
mod detail;
mod genre;
mod home;
mod library;
mod lyrics;
mod mosaic;
mod playback;
mod queue;
mod remote;
mod search;
mod session;
mod settings;
mod sheets;
mod song;
mod toast;
mod updates;

pub use artist::ArtistDetail;
pub use cover::Cover;
pub use detail::{Collection, Detail, Header};
pub use genre::{GenreDetails, Genres};
pub use home::Home;
pub use library::{Library, LibraryEvent, LibraryPart, LibraryState, Problem};
pub use lyrics::{Lyrics, LyricsState};
pub use playback::{Origin, Playback, PlaybackState, Repeat};
pub use queue::{Named, Queue, Resume, Stub};
pub use remote::{Remote, attach as attach_remote};
pub use search::{AlbumHit, ArtistHit, Hit, Kind, PlaylistHit, Search};
pub use session::{Failure, ProviderInfo, Session, SessionEvent, SessionState};
pub use settings::{
    AppSettings, RomanizationScripts, SYSTEM_FONT, SideTab, remember_window, window_placement,
};
pub use song::SongDetail;
pub use toast::{Outcome, Toast, Toasts};
pub use updates::{Release, UpdateState, Updates};

use std::future::Future;
use std::sync::Arc;

use anyhow::Result;
use gpui::{App, AppContext as _, Entity, Global};
use music::{LyricsProvider, MusicProvider};
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;

#[derive(Clone)]
pub struct Io(Arc<Runtime>);

impl Global for Io {}

impl Io {
    pub fn new() -> Result<Self> {
        Ok(Self(Arc::new(Runtime::new()?)))
    }

    pub fn global(cx: &App) -> Self {
        cx.global::<Self>().clone()
    }

    pub fn handle(&self) -> tokio::runtime::Handle {
        self.0.handle().clone()
    }

    pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.0.spawn(future)
    }
}

pub(crate) async fn join<T>(handle: JoinHandle<Result<T>>) -> Result<T> {
    handle.await?
}

pub struct Sonora {
    pub session: Entity<Session>,
    pub cover: Entity<Cover>,
    pub library: Entity<Library>,
    pub lyrics: Entity<Lyrics>,
    pub playback: Entity<Playback>,
    pub queue: Entity<Queue>,
    pub settings: Entity<AppSettings>,
    pub updates: Entity<Updates>,
}

impl Global for Sonora {}

impl Sonora {
    pub fn global(cx: &App) -> &Self {
        cx.global()
    }
}

pub fn init(
    cx: &mut App,
    io: Io,
    providers: Vec<Arc<dyn MusicProvider>>,
    local_provider: Arc<dyn MusicProvider>,
    lyrics_providers: Vec<Arc<dyn LyricsProvider>>,
) {
    cx.set_global(io.clone());

    let settings = cx.new(|_| AppSettings::load());
    let session =
        cx.new(|cx| Session::new(providers, local_provider, settings.clone(), io.clone(), cx));
    let library = cx.new(|cx| Library::new(session.clone(), io.clone(), cx));
    let queue = cx.new(|cx| Queue::new(session.clone(), settings.clone(), cx));
    let playback = cx.new(|cx| Playback::new(session.clone(), queue.clone(), settings.clone(), cx));
    let lyrics = cx.new(|cx| {
        Lyrics::new(
            playback.clone(),
            queue.clone(),
            session.clone(),
            lyrics_providers,
            io.clone(),
            cx,
        )
    });
    let cover = cx.new(|cx| Cover::new(session.clone(), playback.clone(), io.clone(), cx));
    let updates = cx.new(|cx| Updates::new(settings.clone(), io, cx));

    cx.set_global(Sonora {
        session,
        cover,
        library,
        lyrics,
        playback,
        queue,
        settings,
        updates,
    });
}
