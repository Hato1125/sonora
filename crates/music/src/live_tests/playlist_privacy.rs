use std::time::Duration;

use anyhow::{Context as _, Result, anyhow, bail};

use crate::spotify::SpotifyProvider;
use crate::youtube::YouTubeProvider;
use crate::{MusicApi, MusicProvider, ProviderSession};

const NAME: &str = "Sonora live privacy test — safe to delete";
const VERIFY_ATTEMPTS: usize = 30;

#[tokio::test]
#[ignore = "creates, changes, and deletes a playlist on the connected Spotify account"]
async fn spotify_can_make_a_playlist_public_and_private() -> Result<()> {
    let provider = SpotifyProvider::from_env();
    let session = connected(&provider).await?;

    exercise_playlist_privacy(session.api.as_ref()).await
}

#[tokio::test]
#[ignore = "creates, changes, and deletes a playlist on the connected YouTube Music account"]
async fn youtube_can_make_a_playlist_public_and_private() -> Result<()> {
    let provider = YouTubeProvider::new();
    let session = connected(&provider).await?;

    exercise_playlist_privacy(session.api.as_ref()).await
}

async fn connected(provider: &dyn MusicProvider) -> Result<ProviderSession> {
    let session = provider
        .restore()
        .await?
        .with_context(|| format!("{} has no stored Sonora session", provider.name()))?;
    if !session.authenticated {
        bail!(
            "{} restored a guest session, not an account",
            provider.name()
        );
    }
    Ok(session)
}

async fn exercise_playlist_privacy(api: &dyn MusicApi) -> Result<()> {
    let playlist_id = api.create_playlist(NAME).await?;

    let exercise = async {
        wait_until_public(api, &playlist_id, false).await?;
        api.set_playlist_public(&playlist_id, true).await?;
        wait_until_public(api, &playlist_id, true).await?;
        api.set_playlist_public(&playlist_id, false).await?;
        wait_until_public(api, &playlist_id, false).await
    }
    .await;

    let cleanup = api.delete_playlist(&playlist_id).await;
    match (exercise, cleanup) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) => Err(error.context("privacy cycle failed; playlist deleted")),
        (Ok(()), Err(error)) => Err(error.context("privacy cycle passed but cleanup failed")),
        (Err(exercise), Err(cleanup)) => Err(anyhow!(
            "privacy cycle failed: {exercise:#}; deleting the playlist also failed: {cleanup:#}"
        )),
    }
}

async fn wait_until_public(api: &dyn MusicApi, playlist_id: &str, expected: bool) -> Result<()> {
    let mut observed = None;
    for _ in 0..VERIFY_ATTEMPTS {
        if let Ok(detail) = api.playlist(playlist_id).await {
            observed = Some(detail.playlist.public);
            if observed == Some(expected) {
                return Ok(());
            }
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    bail!(
        "playlist {playlist_id} did not become {}; last observed public state: {observed:?}",
        if expected { "public" } else { "private" }
    )
}
