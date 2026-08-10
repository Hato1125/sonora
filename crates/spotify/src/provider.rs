// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use music::{MusicApi as _, MusicProvider, ProviderSession};

use crate::{AuthConfig, LibrespotClient, auth};

pub struct SpotifyProvider {
    config: AuthConfig,
}

impl SpotifyProvider {
    pub fn new(config: AuthConfig) -> Self {
        Self { config }
    }

    pub fn from_env() -> Self {
        Self::new(AuthConfig::from_env())
    }

    async fn session(&self, client: LibrespotClient) -> Result<ProviderSession> {
        let profile = client.profile().await?;
        let playback = Arc::new(audio::Factory::new(client.session().clone()));
        Ok(ProviderSession {
            profile,
            api: Arc::new(client),
            playback,
        })
    }
}

#[async_trait]
impl MusicProvider for SpotifyProvider {
    async fn restore(&self) -> Result<Option<ProviderSession>> {
        let Some(session) = auth::restore(&self.config).await? else {
            return Ok(None);
        };
        self.session(LibrespotClient::new(session)).await.map(Some)
    }

    async fn sign_in(&self) -> Result<ProviderSession> {
        let session = auth::login(&self.config).await?;
        self.session(LibrespotClient::new(session)).await
    }

    fn sign_out(&self) {
        auth::forget(&self.config);
    }
}
