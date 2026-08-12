mod client;
mod playback;
mod wire;

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context as _, Result};
use async_trait::async_trait;
use ytmusic::YtMusic;
use ytmusic::browser::{self, Browser, Family};

use crate::youtube::client::YouTubeClient;
use crate::youtube::playback::Factory;
use crate::{InputSource, MusicProvider, ProviderSession, SignIn, SignInPrompt, UserProfile};

const GUEST_ID: &str = "youtube-guest";

pub struct YouTubeProvider {
    cookies: PathBuf,
    guest: PathBuf,
    resolved: PathBuf,
}

impl YouTubeProvider {
    pub fn new() -> Self {
        let cache = dirs::cache_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("sonora")
            .join("youtube");
        Self {
            cookies: cache.join("cookies.txt"),
            guest: cache.join("guest"),
            resolved: cache.join("resolved.json"),
        }
    }

    fn cookie_client(&self, cookies: &str) -> Arc<YtMusic> {
        Arc::new(YtMusic::with_cookies(cookies).cache_resolutions(self.resolved.clone()))
    }

    fn authenticated_session(&self, api: Arc<YtMusic>, profile: UserProfile) -> ProviderSession {
        let client = YouTubeClient::new(api.clone()).owned_by(profile.display_name.clone());
        ProviderSession {
            profile,
            api: Arc::new(client),
            playback: Arc::new(Factory::new(api)),
            authenticated: true,
            playcounts: false,
        }
    }

    fn guest_session(&self, api: Arc<YtMusic>) -> ProviderSession {
        ProviderSession {
            profile: UserProfile {
                id: GUEST_ID.to_string(),
                display_name: "YouTube Music".to_string(),
            },
            api: Arc::new(YouTubeClient::new(api.clone())),
            playback: Arc::new(Factory::new(api)),
            authenticated: false,
            playcounts: false,
        }
    }

    async fn connect(&self, cookies: &str) -> Result<ProviderSession> {
        let cookies = cookies.trim().to_string();
        if cookies.is_empty() {
            anyhow::bail!("cookie header is empty");
        }
        let api = self.cookie_client(&cookies);
        let profile = wire::profile(
            api.profile()
                .await
                .context("cookies were not accepted; sign in to the browser first")?,
        );
        self.store_cookies(&cookies)?;
        log::debug!("youtube: cookie sign-in succeeded");
        Ok(self.authenticated_session(api, profile))
    }

    fn store_cookies(&self, cookies: &str) -> Result<()> {
        if let Some(parent) = self.cookies.parent() {
            std::fs::create_dir_all(parent).context("cannot create youtube cache dir")?;
        }
        std::fs::write(&self.cookies, cookies).context("cannot store youtube cookies")?;
        let _ = std::fs::remove_file(&self.guest);
        Ok(())
    }

    fn store_guest(&self) {
        if let Some(parent) = self.guest.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&self.guest, b"");
    }

    fn browsers(&self) -> Vec<Browser> {
        let mut browsers = browser::detect();
        #[cfg(target_os = "windows")]
        {
            browsers.retain(|browser| browser.family == Family::Firefox);
            detect_windows_browsers(&mut browsers);
        }
        browsers.sort_by_key(|browser| browser.name);
        browsers
    }
}

#[cfg(target_os = "windows")]
fn detect_windows_browsers(found: &mut Vec<Browser>) {
    const FIREFOX: &[(&str, &str)] = &[
        ("Firefox", "Mozilla/Firefox/Profiles"),
        ("Zen", "zen/Profiles"),
        ("LibreWolf", "librewolf/Profiles"),
        ("Floorp", "Floorp/Profiles"),
        ("Waterfox", "Waterfox/Profiles"),
        ("Mullvad Browser", "Mullvad/MullvadBrowser/Profiles"),
        ("Pale Moon", "Moonchild Productions/Pale Moon/Profiles"),
        ("SeaMonkey", "Mozilla/SeaMonkey/Profiles"),
    ];
    let home = dirs::home_dir();
    let roaming = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| home.as_ref().map(|home| home.join("AppData/Roaming")));

    if let Some(root) = roaming {
        for &(name, relative) in FIREFOX {
            let root = root.join(relative);
            if has_firefox_cookies(&root) {
                push_browser(found, name, Family::Firefox, root);
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn push_browser(found: &mut Vec<Browser>, name: &'static str, family: Family, root: PathBuf) {
    if !found.iter().any(|browser| browser.name == name) {
        found.push(Browser { name, family, root });
    }
}

#[cfg(target_os = "windows")]
fn has_firefox_cookies(root: &std::path::Path) -> bool {
    std::fs::read_dir(root).is_ok_and(|profiles| {
        profiles
            .flatten()
            .any(|profile| profile.path().join("cookies.sqlite").exists())
    })
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

    fn sign_in_options(&self) -> Vec<SignIn> {
        let mut options = vec![SignIn::Anonymous];
        for browser in self.browsers() {
            options.push(SignIn::Browser(browser.name.to_string()));
        }
        options.push(SignIn::Secret);
        options
    }

    fn stored(&self) -> bool {
        self.cookies.exists() || self.guest.exists()
    }

    async fn restore(&self) -> Result<Option<ProviderSession>> {
        if let Ok(cookies) = std::fs::read_to_string(&self.cookies) {
            log::debug!("youtube: restoring authenticated session from cached cookies");
            let api = self.cookie_client(cookies.trim());
            let profile = api.profile().await;
            match profile {
                Ok(profile) => {
                    return Ok(Some(
                        self.authenticated_session(api, wire::profile(profile)),
                    ));
                }
                Err(error) => {
                    log::warn!("youtube: cached cookies are no longer usable: {error:#}");
                }
            }
        }
        if self.guest.exists() {
            log::debug!("youtube: restoring guest session");
            return Ok(Some(self.guest_session(Arc::new(YtMusic::anonymous()))));
        }
        Ok(None)
    }

    async fn sign_in(
        &self,
        method: SignIn,
        prompt: crate::PromptSink,
        mut input: InputSource,
    ) -> Result<ProviderSession> {
        match method {
            SignIn::Anonymous | SignIn::Default => {
                self.store_guest();
                Ok(self.guest_session(Arc::new(YtMusic::anonymous())))
            }
            SignIn::Browser(name) => {
                let browser = self
                    .browsers()
                    .into_iter()
                    .find(|browser| browser.name == name)
                    .with_context(|| format!("{name} is no longer available"))?;
                let cookies = browser::cookies(&browser)
                    .with_context(|| format!("cannot read cookies from {name}"))?;
                self.connect(&cookies).await
            }
            SignIn::Secret => {
                prompt(SignInPrompt::Secret);
                let cookies = input.recv().await.context("sign-in was cancelled")?;
                self.connect(&cookies).await
            }
            SignIn::Path(_) => Err(anyhow::anyhow!(
                "youtube does not sign in with a folder path"
            )),
        }
    }

    fn sign_out(&self) {
        for path in [&self.cookies, &self.guest] {
            if let Err(error) = std::fs::remove_file(path)
                && error.kind() != std::io::ErrorKind::NotFound
            {
                log::warn!("youtube: cannot remove credential cache: {error}");
            }
        }
    }
}
