use gpui::{Context, Entity, Task};
use spotify::{Album, Track};

use crate::{Io, Library, Session, SessionEvent, join};

pub struct Header {
    pub kind: &'static str,
    pub title: String,
    pub meta: String,
    pub cover: Option<String>,
}

pub struct Detail {
    id: Option<String>,
    header: Option<Header>,
    tracks: Vec<Track>,
    loading: bool,
    error: Option<String>,
    session: Entity<Session>,
    library: Entity<Library>,
    io: Io,
    task: Option<Task<()>>,
}

impl Detail {
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
            id: None,
            header: None,
            tracks: Vec::new(),
            loading: false,
            error: None,
            session,
            library,
            io,
            task: None,
        }
    }

    pub fn header(&self) -> Option<&Header> {
        self.header.as_ref()
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

    pub fn open_album(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.shows(id) {
            return;
        }

        let known = self.library.read(cx).album(id).map(album_header);
        self.clear();
        self.id = Some(id.to_owned());
        self.header = known;

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
        let same = self.id.as_deref() == Some(id);
        same && (self.loading || !self.tracks.is_empty())
    }

    fn clear(&mut self) {
        self.task = None;
        self.id = None;
        self.header = None;
        self.tracks.clear();
        self.loading = false;
        self.error = None;
    }
}

fn album_header(album: &Album) -> Header {
    let mut parts = vec![album.artists.clone()];
    if album.year > 0 {
        parts.push(format!("{}", album.year));
    }
    if album.track_count > 0 {
        parts.push(format!("{} songs", album.track_count));
    }

    Header {
        kind: "ALBUM",
        title: album.name.clone(),
        meta: parts.join(" • "),
        cover: album.cover_large.clone(),
    }
}
