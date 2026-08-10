// SPDX-License-Identifier: GPL-3.0-or-later

mod client;
mod playback;
mod wire;

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context as _, Result};
use async_trait::async_trait;
use ytmusic::oauth::ClientIdentity;
use ytmusic::{YtMusic, oauth};

use crate::youtube::client::YouTubeClient;
use crate::youtube::playback::Factory;
use crate::{InputSource, MusicProvider, PromptSink, ProviderSession, SignInPrompt};

pub struct YouTubeProvider {
    tokens: PathBuf,
    cookies: PathBuf,
    identity: Option<ClientIdentity>,
}

impl YouTubeProvider {
    pub fn new() -> Self {
        let cache = dirs::cache_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("sonora")
            .join("youtube");
        Self {
            tokens: cache.join("tokens.json"),
            cookies: cache.join("cookies.txt"),
            identity: identity_from_env(),
        }
    }

    async fn session(&self, api: Arc<YtMusic>) -> Result<ProviderSession> {
        let profile = wire::profile(api.profile().await?);
        Ok(ProviderSession {
            profile,
            api: Arc::new(YouTubeClient::new(api.clone())),
            playback: Arc::new(Factory::new(api)),
        })
    }

    async fn sign_in_with_cookies(
        &self,
        prompt: PromptSink,
        mut input: InputSource,
    ) -> Result<ProviderSession> {
        prompt(SignInPrompt::Secret);
        let cookies = input.recv().await.context("sign-in was cancelled")?;
        let cookies = cookies.trim().to_string();
        if cookies.is_empty() {
            anyhow::bail!("cookie header is empty");
        }
        let api = Arc::new(YtMusic::with_cookies(&cookies));
        let session = self
            .session(api)
            .await
            .context("cookies were not accepted")?;
        if let Some(parent) = self.cookies.parent() {
            std::fs::create_dir_all(parent).context("cannot create youtube cache dir")?;
        }
        std::fs::write(&self.cookies, &cookies).context("cannot store youtube cookies")?;
        log::debug!("youtube: cookie sign-in succeeded");
        Ok(session)
    }

    async fn sign_in_with_device(
        &self,
        identity: ClientIdentity,
        prompt: PromptSink,
    ) -> Result<ProviderSession> {
        let http = reqwest::Client::new();
        let device = oauth::request_device_code(&http, &identity)
            .await
            .context("cannot start youtube sign-in")?;
        prompt(SignInPrompt::Code {
            code: device.user_code.clone(),
            url: device.verification_url.clone(),
        });
        open_browser(&device.verification_url);
        let tokens = oauth::poll_token(&http, &identity, &device).await?;
        log::debug!("youtube: sign-in granted, saving tokens");
        tokens
            .save(&self.tokens)
            .context("cannot store youtube tokens")?;
        let api = Arc::new(YtMusic::new(tokens).persist_to(self.tokens.clone()));
        self.session(api).await
    }
}

impl Default for YouTubeProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MusicProvider for YouTubeProvider {
    fn name(&self) -> &'static str {
        "YouTube Music"
    }

    fn slug(&self) -> &'static str {
        "youtube"
    }

    async fn restore(&self) -> Result<Option<ProviderSession>> {
        if let Ok(cookies) = std::fs::read_to_string(&self.cookies) {
            log::debug!("youtube: restoring session from cached cookies");
            let api = Arc::new(YtMusic::with_cookies(cookies.trim()));
            return self.session(api).await.map(Some);
        }
        let Some(tokens) = oauth::Tokens::load(&self.tokens)? else {
            log::debug!("youtube: no cached credentials, staying signed out");
            return Ok(None);
        };
        log::debug!("youtube: restoring session from cached tokens");
        let api = Arc::new(YtMusic::new(tokens).persist_to(self.tokens.clone()));
        if let Err(error) = api.library_playlists().await {
            log::warn!("youtube: cached tokens no longer usable ({error:#}), signing out");
            self.sign_out();
            return Ok(None);
        }
        self.session(api).await.map(Some)
    }

    async fn sign_in(&self, prompt: PromptSink, input: InputSource) -> Result<ProviderSession> {
        match self.identity.clone() {
            Some(identity) => self.sign_in_with_device(identity, prompt).await,
            None => self.sign_in_with_cookies(prompt, input).await,
        }
    }

    fn sign_out(&self) {
        for path in [&self.tokens, &self.cookies] {
            if let Err(error) = std::fs::remove_file(path)
                && error.kind() != std::io::ErrorKind::NotFound
            {
                log::warn!("youtube: cannot remove credential cache: {error}");
            }
        }
    }
}

fn identity_from_env() -> Option<ClientIdentity> {
    let id = std::env::var("SONORA_YT_CLIENT_ID").ok()?;
    let secret = std::env::var("SONORA_YT_CLIENT_SECRET").ok()?;
    Some(ClientIdentity { id, secret })
}

fn open_browser(url: &str) {
    let opened = std::process::Command::new("xdg-open")
        .arg(url)
        .spawn()
        .is_ok();
    if !opened {
        log::warn!("youtube: cannot open browser for {url}");
    }
}
