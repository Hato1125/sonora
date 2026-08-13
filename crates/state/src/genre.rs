use gpui::{Context, Entity, Task};
use music::{Genre, GenreDetail, GenreSection};
use tokio::task::AbortHandle;

use crate::{Io, Session, SessionEvent, join};

pub struct Genres {
    genres: Vec<Genre>,
    loading: bool,
    error: Option<String>,
    session: Entity<Session>,
    io: Io,
    task: Option<Task<()>>,
}

impl Genres {
    pub fn new(session: Entity<Session>, io: Io, cx: &mut Context<Self>) -> Self {
        cx.subscribe(&session, |this, _, event, cx| match event {
            SessionEvent::SignedIn => this.load(cx),
            SessionEvent::SignedOut => {
                this.task = None;
                this.genres.clear();
                this.loading = false;
                this.error = None;
                cx.notify();
            }
            SessionEvent::LocalChanged => {}
        })
        .detach();

        Self {
            genres: Vec::new(),
            loading: false,
            error: None,
            session,
            io,
            task: None,
        }
    }

    pub fn genres(&self) -> &[Genre] {
        &self.genres
    }

    pub fn is_loading(&self) -> bool {
        self.loading
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn load(&mut self, cx: &mut Context<Self>) {
        if self.loading || !self.genres.is_empty() {
            return;
        }
        let Some(client) = self.session.read(cx).client() else {
            return;
        };

        self.loading = true;
        self.error = None;
        cx.notify();

        let io = self.io.clone();
        self.task = Some(cx.spawn(async move |this, cx| {
            let loaded = join(io.spawn(async move { client.genres().await })).await;

            this.update(cx, |this, cx| {
                this.loading = false;
                match loaded {
                    Ok(genres) => this.genres = genres,
                    Err(error) => this.error = Some(format!("{error:#}")),
                }
                cx.notify();
            })
            .ok();
        }));
    }
}

pub struct GenreDetails {
    id: Option<String>,
    detail: Option<GenreDetail>,
    loading: bool,
    error: Option<String>,
    session: Entity<Session>,
    io: Io,
    task: Option<Task<()>>,
    request: Option<AbortHandle>,
}

impl GenreDetails {
    pub fn new(session: Entity<Session>, io: Io, cx: &mut Context<Self>) -> Self {
        cx.subscribe(&session, |this, _, event, cx| match event {
            SessionEvent::SignedIn => {
                if let Some(id) = this.id.clone() {
                    this.open(&id, cx);
                }
            }
            SessionEvent::SignedOut => {
                this.clear();
                cx.notify();
            }
            SessionEvent::LocalChanged => {}
        })
        .detach();

        Self {
            id: None,
            detail: None,
            loading: false,
            error: None,
            session,
            io,
            task: None,
            request: None,
        }
    }

    pub fn name(&self) -> Option<&str> {
        self.detail.as_ref().map(|detail| detail.name.as_str())
    }

    pub fn cover(&self) -> Option<&str> {
        self.detail
            .as_ref()
            .and_then(|detail| detail.cover.as_deref())
    }

    pub fn color(&self) -> Option<u32> {
        self.detail.as_ref().and_then(|detail| detail.color)
    }

    pub fn sections(&self) -> &[GenreSection] {
        self.detail
            .as_ref()
            .map(|detail| detail.sections.as_slice())
            .unwrap_or_default()
    }

    pub fn is_loading(&self) -> bool {
        self.loading
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn open(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.id.as_deref() == Some(id) && (self.loading || self.detail.is_some()) {
            return;
        }

        self.clear();
        self.id = Some(id.to_owned());

        let Some(client) = self.session.read(cx).client() else {
            cx.notify();
            return;
        };

        self.loading = true;
        cx.notify();

        let id = id.to_owned();
        let request = self.io.spawn({
            let id = id.clone();
            async move { client.genre(&id).await }
        });
        self.request = Some(request.abort_handle());
        self.task = Some(cx.spawn(async move |this, cx| {
            let loaded = join(request).await;

            this.update(cx, |this, cx| {
                if this.id.as_deref() != Some(id.as_str()) {
                    return;
                }

                this.loading = false;
                this.request = None;
                match loaded {
                    Ok(detail) => this.detail = Some(detail),
                    Err(error) => this.error = Some(format!("{error:#}")),
                }
                cx.notify();
            })
            .ok();
        }));
    }

    fn clear(&mut self) {
        self.task = None;
        if let Some(request) = self.request.take() {
            request.abort();
        }
        self.id = None;
        self.detail = None;
        self.loading = false;
        self.error = None;
    }
}
