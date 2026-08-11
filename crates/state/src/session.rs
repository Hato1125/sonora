// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::Arc;

use anyhow::Error;
use gpui::{Context, Entity, EventEmitter, Task};
use music::{
    MusicApi, MusicProvider, PlaybackFactory, PromptSink, ProviderSession, SignIn, SignInPrompt,
    UserProfile,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::settings::AppSettings;
use crate::{Io, join};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionState {
    SignedOut,
    Restoring,
    Authorizing(Option<SignInPrompt>),
    SignedIn(UserProfile),
    Failed(String),
}

pub enum SessionEvent {
    SignedIn,
    SignedOut,
}

pub struct ProviderInfo {
    pub slug: &'static str,
    pub name: &'static str,
    pub options: Vec<SignIn>,
    pub stored: bool,
    pub active: bool,
}

pub struct Session {
    state: SessionState,
    providers: Vec<Arc<dyn MusicProvider>>,
    active: Option<usize>,
    settings: Entity<AppSettings>,
    client: Option<Arc<dyn MusicApi>>,
    playback: Option<Arc<dyn PlaybackFactory>>,
    authenticated: bool,
    playcounts: bool,
    io: Io,
    task: Option<Task<()>>,
    prompt_task: Option<Task<()>>,
    input: Option<UnboundedSender<String>>,
}

impl EventEmitter<SessionEvent> for Session {}

impl Session {
    pub fn new(
        providers: Vec<Arc<dyn MusicProvider>>,
        settings: Entity<AppSettings>,
        io: Io,
        cx: &mut Context<Self>,
    ) -> Self {
        let remembered = settings.read(cx).provider().to_string();
        let active = providers
            .iter()
            .position(|provider| provider.slug() == remembered);
        Self {
            state: SessionState::SignedOut,
            providers,
            active,
            settings,
            client: None,
            playback: None,
            authenticated: false,
            playcounts: false,
            io,
            task: None,
            prompt_task: None,
            input: None,
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

    pub fn providers(&self) -> impl Iterator<Item = ProviderInfo> + '_ {
        self.providers
            .iter()
            .enumerate()
            .map(|(index, provider)| ProviderInfo {
                slug: provider.slug(),
                name: provider.name(),
                options: provider.sign_in_options(),
                stored: provider.stored(),
                active: self.active == Some(index),
            })
    }

    pub fn connected(&self) -> impl Iterator<Item = ProviderInfo> + '_ {
        self.providers().filter(|info| info.stored)
    }

    pub fn forget(&mut self, slug: &str, cx: &mut Context<Self>) {
        let Some(index) = self
            .providers
            .iter()
            .position(|provider| provider.slug() == slug)
        else {
            return;
        };
        if self.active == Some(index) {
            return self.sign_out(cx);
        }
        self.providers[index].sign_out();
        cx.notify();
    }

    pub fn switch(&mut self, slug: &str, cx: &mut Context<Self>) {
        if self.is_pending() {
            return;
        }
        let Some(index) = self
            .providers
            .iter()
            .position(|provider| provider.slug() == slug)
        else {
            return;
        };
        if self.active == Some(index) && matches!(self.state, SessionState::SignedIn(_)) {
            return;
        }
        self.client = None;
        self.playback = None;
        self.authenticated = false;
        self.playcounts = false;
        self.active = Some(index);
        cx.emit(SessionEvent::SignedOut);
        self.restore(cx);
    }

    pub fn provider_name(&self) -> Option<&'static str> {
        let provider = &self.providers[self.active?];
        Some(provider.name())
    }

    pub fn provider_slug(&self) -> Option<&'static str> {
        let provider = &self.providers[self.active?];
        Some(provider.slug())
    }

    pub fn authenticated(&self) -> bool {
        self.authenticated
    }

    pub fn playcounts(&self) -> bool {
        self.playcounts
    }

    pub fn is_pending(&self) -> bool {
        matches!(
            self.state,
            SessionState::Restoring | SessionState::Authorizing(_)
        )
    }

    pub fn restore(&mut self, cx: &mut Context<Self>) {
        if self.is_pending() {
            return;
        }
        let Some(active) = self.active else {
            self.state = SessionState::SignedOut;
            cx.notify();
            cx.emit(SessionEvent::SignedOut);
            return;
        };
        self.state = SessionState::Restoring;
        cx.notify();

        let provider = self.providers[active].clone();
        let io = self.io.clone();
        self.task = Some(cx.spawn(async move |this, cx| {
            let restored = join(io.spawn(async move { provider.restore().await })).await;

            this.update(cx, |this, cx| match restored {
                Ok(Some(session)) => this.signed_in(session, active, cx),
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

    pub fn sign_in(&mut self, slug: &str, method: SignIn, cx: &mut Context<Self>) {
        if self.is_pending() {
            return;
        }
        let Some(index) = self
            .providers
            .iter()
            .position(|provider| provider.slug() == slug)
        else {
            return;
        };
        self.state = SessionState::Authorizing(None);
        cx.notify();

        let (input_tx, input_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        self.input = Some(input_tx);
        let (prompt_tx, mut prompt_rx) = tokio::sync::mpsc::unbounded_channel::<SignInPrompt>();
        self.prompt_task = Some(cx.spawn(async move |this, cx| {
            while let Some(prompt) = prompt_rx.recv().await {
                this.update(cx, |this, cx| {
                    if matches!(this.state, SessionState::Authorizing(_)) {
                        this.state = SessionState::Authorizing(Some(prompt));
                        cx.notify();
                    }
                })
                .ok();
            }
        }));
        let prompt: PromptSink = Arc::new(move |prompt| {
            prompt_tx.send(prompt).ok();
        });

        let provider = self.providers[index].clone();
        let io = self.io.clone();
        self.task = Some(cx.spawn(async move |this, cx| {
            let authorized =
                join(io.spawn(async move { provider.sign_in(method, prompt, input_rx).await }))
                    .await;

            this.update(cx, |this, cx| {
                this.prompt_task = None;
                this.input = None;
                match authorized {
                    Ok(session) => this.signed_in(session, index, cx),
                    Err(error) => this.failed(&error, cx),
                }
            })
            .ok();
        }));
    }

    pub fn cancel_sign_in(&mut self, cx: &mut Context<Self>) {
        if !matches!(self.state, SessionState::Authorizing(_)) {
            return;
        }
        self.task = None;
        self.prompt_task = None;
        self.input = None;
        self.state = SessionState::SignedOut;
        cx.notify();
        cx.emit(SessionEvent::SignedOut);
    }

    pub fn submit_input(&mut self, text: String, cx: &mut Context<Self>) {
        if let Some(input) = &self.input {
            input.send(text).ok();
            if let SessionState::Authorizing(Some(SignInPrompt::Secret)) = &self.state {
                self.state = SessionState::Authorizing(None);
                cx.notify();
            }
        }
    }

    pub fn sign_out(&mut self, cx: &mut Context<Self>) {
        if let Some(active) = self.active {
            self.providers[active].sign_out();
        }
        self.task = None;
        self.prompt_task = None;
        self.input = None;
        self.client = None;
        self.playback = None;
        self.authenticated = false;
        self.state = SessionState::SignedOut;
        cx.notify();
        cx.emit(SessionEvent::SignedOut);
    }

    fn signed_in(&mut self, session: ProviderSession, index: usize, cx: &mut Context<Self>) {
        self.active = Some(index);
        let slug = self.providers[index].slug();
        self.settings.update(cx, |settings, cx| {
            settings.set_provider(slug, cx);
        });
        self.client = Some(session.api);
        self.playback = Some(session.playback);
        self.authenticated = session.authenticated;
        self.playcounts = session.playcounts;
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
