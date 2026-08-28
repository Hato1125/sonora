use std::sync::Arc;

use gpui::{Context, Entity, Task};
use music::{Playlist, UserDetail};
use tokio::task::AbortHandle;

use crate::{Io, Session, SessionEvent, join};

pub struct Profile {
    id: Option<String>,
    user: Option<Arc<UserDetail>>,
    loading: bool,
    error: Option<String>,
    session: Entity<Session>,
    io: Io,
    task: Option<Task<()>>,
    request: Option<AbortHandle>,
}

impl Profile {
    pub fn new(session: Entity<Session>, io: Io, cx: &mut Context<Self>) -> Self {
        cx.subscribe(&session, |this, _, event, cx| match event {
            SessionEvent::SignedIn => {
                if let Some(id) = this.id.clone() {
                    this.clear();
                    this.open(&id, cx);
                }
            }
            SessionEvent::SignedOut => {
                this.clear();
                cx.notify();
            }
            SessionEvent::Reconnected | SessionEvent::LocalChanged => {}
        })
        .detach();

        Self {
            id: None,
            user: None,
            loading: false,
            error: None,
            session,
            io,
            task: None,
            request: None,
        }
    }

    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    pub fn user(&self) -> Option<&UserDetail> {
        self.user.as_deref()
    }

    pub fn playlists(&self) -> &[Playlist] {
        self.user
            .as_ref()
            .map(|user| user.playlists.as_slice())
            .unwrap_or_default()
    }

    pub fn is_loading(&self) -> bool {
        self.loading
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn open(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.id.as_deref() == Some(id) && (self.loading || self.user.is_some()) {
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
            async move { client.user(&id).await }
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
                    Ok(user) => this.user = Some(Arc::new(user)),
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
        self.user = None;
        self.loading = false;
        self.error = None;
    }
}
