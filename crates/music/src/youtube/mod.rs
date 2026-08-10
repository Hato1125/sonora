// SPDX-License-Identifier: GPL-3.0-or-later

mod client;
mod playback;
mod wire;

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context as _, Result};
use async_trait::async_trait;
use ytmusic::{YtMusic, oauth};

use crate::youtube::client::YouTubeClient;
use crate::youtube::playback::Factory;
use crate::{MusicProvider, PromptSink, ProviderSession, SignInPrompt};

pub struct YouTubeProvider {
    tokens: PathBuf,
}

impl YouTubeProvider {
    pub fn new() -> Self {
        let cache = dirs::cache_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("sonora")
            .join("youtube");
        Self {
            tokens: cache.join("tokens.json"),
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
        let Some(tokens) = oauth::Tokens::load(&self.tokens)? else {
            log::debug!("youtube: no cached tokens, staying signed out");
            return Ok(None);
        };
        log::debug!("youtube: restoring session from cached tokens");
        let api = Arc::new(YtMusic::new(tokens).persist_to(self.tokens.clone()));
        self.session(api).await.map(Some)
    }

    async fn sign_in(&self, prompt: PromptSink) -> Result<ProviderSession> {
        let http = reqwest::Client::new();
        let identity = oauth::fetch_identity(&http).await;
        let device = oauth::request_device_code(&http, &identity)
            .await
            .context("cannot start youtube sign-in")?;
        prompt(SignInPrompt {
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

    fn sign_out(&self) {
        if let Err(error) = std::fs::remove_file(&self.tokens)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            log::warn!("youtube: cannot remove token cache: {error}");
        }
    }
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
