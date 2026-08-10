// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::Arc;

use anyhow::Error;
use gpui::{Context, EventEmitter, Task};
use music::{MusicApi, MusicProvider, PlaybackFactory, ProviderSession, UserProfile};

use crate::{Io, join};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionState {
    SignedOut,
    Restoring,
    Authorizing,
    SignedIn(UserProfile),
    Failed(String),
}

pub enum SessionEvent {
    SignedIn,
    SignedOut,
}

pub struct Session {
    state: SessionState,
    provider: Arc<dyn MusicProvider>,
    client: Option<Arc<dyn MusicApi>>,
    playback: Option<Arc<dyn PlaybackFactory>>,
    io: Io,
    task: Option<Task<()>>,
}

impl EventEmitter<SessionEvent> for Session {}

impl Session {
    pub fn new(provider: Arc<dyn MusicProvider>, io: Io) -> Self {
        Self {
            state: SessionState::SignedOut,
            provider,
            client: None,
            playback: None,
            io,
            task: None,
        }
    }

    pub fn state(&self) -> &SessionState {
        &self.state
    }

    pub fn client(&self) -> Option<Arc<dyn MusicApi>> {
        self.client.clone()
    }

    pub fn playback(&self) -> Option<Arc<dyn PlaybackFactory>> {
        self.playback.clone()
    }

    pub fn is_pending(&self) -> bool {
        matches!(
            self.state,
            SessionState::Restoring | SessionState::Authorizing
        )
    }

    pub fn restore(&mut self, cx: &mut Context<Self>) {
        if self.is_pending() {
            return;
        }
        self.state = SessionState::Restoring;
        cx.notify();

        let provider = self.provider.clone();
        let io = self.io.clone();
        self.task = Some(cx.spawn(async move |this, cx| {
            let restored = join(io.spawn(async move { provider.restore().await })).await;

            this.update(cx, |this, cx| match restored {
                Ok(Some(session)) => this.signed_in(session, cx),
                Ok(None) => {
                    this.state = SessionState::SignedOut;
                    cx.notify();
                    cx.emit(SessionEvent::SignedOut);
                }
                Err(error) => this.failed(&error, cx),
            })
            .ok();
        }));
    }

    pub fn sign_in(&mut self, cx: &mut Context<Self>) {
        if self.is_pending() {
            return;
        }
        self.state = SessionState::Authorizing;
        cx.notify();

        let provider = self.provider.clone();
        let io = self.io.clone();
        self.task = Some(cx.spawn(async move |this, cx| {
            let authorized = join(io.spawn(async move { provider.sign_in().await })).await;

            this.update(cx, |this, cx| match authorized {
                Ok(session) => this.signed_in(session, cx),
                Err(error) => this.failed(&error, cx),
            })
            .ok();
        }));
    }

    pub fn sign_out(&mut self, cx: &mut Context<Self>) {
        self.provider.sign_out();
        self.task = None;
        self.client = None;
        self.playback = None;
        self.state = SessionState::SignedOut;
        cx.notify();
        cx.emit(SessionEvent::SignedOut);
    }

    fn signed_in(&mut self, session: ProviderSession, cx: &mut Context<Self>) {
        self.client = Some(session.api);
        self.playback = Some(session.playback);
        self.state = SessionState::SignedIn(session.profile);
        cx.notify();
        cx.emit(SessionEvent::SignedIn);
    }

    fn failed(&mut self, error: &Error, cx: &mut Context<Self>) {
        self.client = None;
        self.playback = None;
        self.state = SessionState::Failed(format!("{error:#}"));
        cx.notify();
        cx.emit(SessionEvent::SignedOut);
    }
}
