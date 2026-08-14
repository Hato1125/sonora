use anyhow::{Result, bail};
use music::MusicApi as _;
use music::spotify::{AuthConfig, LibrespotClient, auth};

#[tokio::main]
async fn main() -> Result<()> {
    let term = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
    let Some(session) = auth::restore(&AuthConfig::from_env()).await? else {
        bail!("no cached Spotify credentials");
    };
    let client = LibrespotClient::new(session);

    for album in client.search_albums(&term).await? {
        println!(
            "album    {} {} — {} ({}) {:?}",
            album.id,
            album.name,
            album.artists,
            album.year,
            album.cover.as_deref()
        );
    }
    for playlist in client.search_playlists(&term).await? {
        println!(
            "playlist {} {} — {} {:?}",
            playlist.id,
            playlist.name,
            playlist.owner,
            playlist.cover.as_deref()
        );
    }
    Ok(())
}
