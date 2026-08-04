use gpui::{Context, Entity, Task};
use spotify::{Album, Track};

use crate::{Io, Library, Session, SessionEvent, join};

pub struct AlbumDetail {
    album: Option<Album>,
    tracks: Vec<Track>,
    loading: bool,
    error: Option<String>,
    session: Entity<Session>,
    library: Entity<Library>,
    io: Io,
    task: Option<Task<()>>,
}

impl AlbumDetail {
    pub fn new(
        session: Entity<Session>,
        library: Entity<Library>,
        io: Io,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.subscribe(&session, |this, _, event, cx| {
            if matches!(event, SessionEvent::SignedOut) {
                this.clear();
                cx.notify();
            }
        })
        .detach();

        Self {
            album: None,
            tracks: Vec::new(),
            loading: false,
            error: None,
            session,
            library,
            io,
            task: None,
        }
    }

    pub fn album(&self) -> Option<&Album> {
        self.album.as_ref()
    }

    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }

    pub fn is_loading(&self) -> bool {
        self.loading
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn open(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.shows(id) {
            return;
        }

        let known = self.library.read(cx).album(id).cloned();
        self.clear();
        self.album = known;

        let Some(client) = self.session.read(cx).client() else {
            cx.notify();
            return;
        };

        self.loading = true;
        cx.notify();

        let io = self.io.clone();
        let id = id.to_owned();
        self.task = Some(cx.spawn(async move |this, cx| {
            let loaded = join(io.spawn(async move { client.album_tracks(&id).await })).await;

            this.update(cx, |this, cx| {
                this.loading = false;
                match loaded {
                    Ok(tracks) => this.tracks = tracks,
                    Err(error) => this.error = Some(format!("{error:#}")),
                }
                cx.notify();
            })
            .ok();
        }));
    }

    fn shows(&self, id: &str) -> bool {
        let same = self.album.as_ref().is_some_and(|album| album.id == id);
        same && (self.loading || !self.tracks.is_empty())
    }

    fn clear(&mut self) {
        self.task = None;
        self.album = None;
        self.tracks.clear();
        self.loading = false;
        self.error = None;
    }
}
