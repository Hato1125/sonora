use anyhow::{Context as _, Result};
#[cfg(target_os = "windows")]
use ytmusic::browser::Family;
use ytmusic::browser::{self, Browser};

pub struct YoutubeSession {
    cookies: String,
}

impl YoutubeSession {
    pub fn cookies(&self) -> &str {
        &self.cookies
    }
}

pub fn direct(browser: &Browser) -> Result<YoutubeSession> {
    let cookies = browser::cookies(browser)
        .with_context(|| format!("cannot read cookies from {}", browser.name))?;
    Ok(YoutubeSession { cookies })
}

pub async fn acquire(browser: &Browser, prompt: &crate::PromptSink) -> Result<YoutubeSession> {
    #[cfg(target_os = "windows")]
    if browser.family == Family::Chromium {
        return windows::interactive(browser, prompt).await;
    }
    direct(browser)
}

#[cfg(target_os = "windows")]
pub fn windows_chromium_browsers() -> Vec<Browser> {
    ["Chrome", "Chromium", "Edge", "Brave"]
        .into_iter()
        .filter_map(|name| {
            windows::executable(name).map(|root| Browser {
                name,
                family: Family::Chromium,
                root,
            })
        })
        .collect()
}

#[cfg(target_os = "windows")]
mod windows {
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};
    use std::process::{Child, Command, Stdio};
    use std::time::{Duration, Instant};

    use anyhow::{Context as _, Result, bail};
    use futures::{SinkExt as _, StreamExt as _};
    use serde::Deserialize;
    use serde_json::{Value, json};
    use tempfile::TempDir;
    use tokio::net::TcpStream;
    use tokio::time::sleep;
    use tokio_tungstenite::tungstenite::Message;
    use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};
    use ytmusic::YtMusic;
    use ytmusic::browser::Browser;

    use super::YoutubeSession;
    use crate::SignInPrompt;

    const STARTUP_TIMEOUT: Duration = Duration::from_secs(30);
    const LOGIN_TIMEOUT: Duration = Duration::from_secs(10 * 60);
    const POLL_INTERVAL: Duration = Duration::from_secs(1);
    const COOKIE_NAMES: &[&str] = &[
        "SID",
        "__Secure-1PSID",
        "__Secure-3PSID",
        "__Secure-1PSIDTS",
        "__Secure-3PSIDTS",
        "__Secure-1PSIDCC",
        "__Secure-3PSIDCC",
        "HSID",
        "SSID",
        "APISID",
        "SAPISID",
        "__Secure-1PAPISID",
        "__Secure-3PAPISID",
        "LOGIN_INFO",
        "SIDCC",
        "PREF",
        "VISITOR_INFO1_LIVE",
        "VISITOR_PRIVACY_METADATA",
        "__Secure-YNID",
        "__Secure-ROLLOUT_TOKEN",
    ];

    type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

    struct ProcessGuard(Child);

    impl Drop for ProcessGuard {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    struct BrowserGuard {
        child: Child,
        profile: Option<TempDir>,
    }

    impl BrowserGuard {
        fn close(&mut self) {
            let _ = self.child.kill();
            let _ = self.child.wait();
            if let Some(profile) = self.profile.take()
                && let Err(error) = profile.close()
            {
                log::warn!("youtube: cannot remove temporary browser profile: {error}");
            }
        }
    }

    impl Drop for BrowserGuard {
        fn drop(&mut self) {
            self.close();
        }
    }

    #[derive(Deserialize)]
    struct Cookie {
        name: String,
        value: String,
        domain: String,
    }

    pub async fn interactive(
        browser: &Browser,
        prompt: &crate::PromptSink,
    ) -> Result<YoutubeSession> {
        let executable = executable(browser.name)
            .with_context(|| format!("{} is no longer available", browser.name))?;
        let profile = tempfile::Builder::new()
            .prefix("sonora-youtube-")
            .tempdir()
            .context("cannot create temporary Chromium profile")?;

        let login = launch_login(&executable, profile.path())
            .with_context(|| format!("cannot launch {}", browser.name))?;
        prompt(SignInPrompt::Browser);
        let mut login = ProcessGuard(login);
        wait_for_login_window(&mut login).await?;

        let child = launch_cdp(&executable, profile.path())
            .with_context(|| format!("cannot inspect the {} session", browser.name))?;
        let mut guard = BrowserGuard {
            child,
            profile: Some(profile),
        };
        let endpoint = endpoint(&mut guard).await?;
        let (mut socket, _) = connect_async(&endpoint)
            .await
            .context("cannot connect to Chromium DevTools Protocol")?;
        let result = command(&mut socket, 1, "Storage.getCookies", json!({})).await?;
        let cookies: Vec<Cookie> = serde_json::from_value(
            result
                .get("cookies")
                .cloned()
                .context("Chromium returned no cookies")?,
        )
        .context("cannot decode Chromium cookies")?;
        let header = cookie_header(cookies)
            .context("no usable YouTube Music account was found; finish signing in before closing the browser")?;
        YtMusic::with_cookies(&header)
            .profile()
            .await
            .context("session verification failed")?;
        let _ = command(&mut socket, 2, "Browser.close", json!({})).await;
        guard.close();
        Ok(YoutubeSession { cookies: header })
    }

    async fn wait_for_login_window(child: &mut ProcessGuard) -> Result<()> {
        let deadline = Instant::now() + LOGIN_TIMEOUT;
        loop {
            if let Some(status) = child
                .0
                .try_wait()
                .context("cannot inspect browser process")?
            {
                if status.success() {
                    return Ok(());
                }
                bail!("browser closed before login completed ({status})");
            }
            if Instant::now() >= deadline {
                bail!(
                    "YouTube Music login timed out; sign in and close the browser window when finished"
                );
            }
            sleep(POLL_INTERVAL).await;
        }
    }

    fn launch_login(executable: &Path, profile: &Path) -> Result<Child> {
        base_command(executable, profile)
            .arg("--new-window")
            .spawn()
            .context("Chromium login process could not be started")
    }

    fn launch_cdp(executable: &Path, profile: &Path) -> Result<Child> {
        base_command(executable, profile)
            .arg("--remote-debugging-port=0")
            .arg("--headless=new")
            .arg("about:blank")
            .spawn()
            .context("Chromium inspection process could not be started")
    }

    fn base_command(executable: &Path, profile: &Path) -> Command {
        let mut command = Command::new(executable);
        command
            .arg(format!("--user-data-dir={}", profile.display()))
            .arg("--window-size=520,720")
            .arg("--no-first-run")
            .arg("--no-default-browser-check")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        command
    }

    async fn endpoint(guard: &mut BrowserGuard) -> Result<String> {
        let file = guard
            .profile
            .as_ref()
            .context("temporary profile was already removed")?
            .path()
            .join("DevToolsActivePort");
        let deadline = Instant::now() + STARTUP_TIMEOUT;
        loop {
            if let Some(status) = guard
                .child
                .try_wait()
                .context("cannot inspect browser process")?
            {
                bail!("browser closed before its login window opened ({status})");
            }
            if let Ok(contents) = std::fs::read_to_string(&file) {
                let mut lines = contents.lines();
                if let (Some(port), Some(path)) = (lines.next(), lines.next())
                    && port.parse::<u16>().is_ok()
                    && path.starts_with("/devtools/browser/")
                {
                    return Ok(format!("ws://127.0.0.1:{port}{path}"));
                }
            }
            if Instant::now() >= deadline {
                bail!("Chromium did not enable its debugging endpoint");
            }
            sleep(Duration::from_millis(100)).await;
        }
    }

    async fn command(socket: &mut Socket, id: u64, method: &str, params: Value) -> Result<Value> {
        socket
            .send(Message::Text(
                json!({ "id": id, "method": method, "params": params })
                    .to_string()
                    .into(),
            ))
            .await
            .with_context(|| format!("cannot send CDP command {method}"))?;
        loop {
            let message = socket
                .next()
                .await
                .context("browser closed its debugging connection")?
                .context("browser debugging connection failed")?;
            match message {
                Message::Text(text) => {
                    let response: Value = serde_json::from_str(&text)
                        .context("cannot decode Chromium debugging response")?;
                    if response.get("id").and_then(Value::as_u64) != Some(id) {
                        continue;
                    }
                    if let Some(error) = response.get("error") {
                        bail!("Chromium rejected {method}: {error}");
                    }
                    return response
                        .get("result")
                        .cloned()
                        .context("Chromium returned no command result");
                }
                Message::Ping(payload) => socket.send(Message::Pong(payload)).await?,
                Message::Close(_) => bail!("browser closed before login completed"),
                _ => {}
            }
        }
    }

    fn cookie_header(cookies: Vec<Cookie>) -> Option<String> {
        let mut values = HashMap::new();
        for cookie in cookies {
            let domain = cookie.domain.trim_start_matches('.');
            if (domain == "youtube.com" || domain.ends_with(".youtube.com"))
                && COOKIE_NAMES.contains(&cookie.name.as_str())
            {
                values.insert(cookie.name, cookie.value);
            }
        }
        if !values.contains_key("SAPISID") && !values.contains_key("__Secure-3PAPISID") {
            return None;
        }
        let mut header = String::new();
        for name in COOKIE_NAMES {
            if let Some(value) = values.get(*name) {
                if !header.is_empty() {
                    header.push_str("; ");
                }
                header.push_str(name);
                header.push('=');
                header.push_str(value);
            }
        }
        Some(header)
    }

    pub(super) fn executable(name: &str) -> Option<PathBuf> {
        let local = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
        let program = std::env::var_os("PROGRAMFILES").map(PathBuf::from);
        let program_x86 = std::env::var_os("PROGRAMFILES(X86)").map(PathBuf::from);
        let candidates: Vec<PathBuf> = match name {
            "Chrome" => [
                local.map(|root| root.join("Google/Chrome/Application/chrome.exe")),
                program.map(|root| root.join("Google/Chrome/Application/chrome.exe")),
                program_x86.map(|root| root.join("Google/Chrome/Application/chrome.exe")),
            ]
            .into_iter()
            .flatten()
            .collect(),
            "Chromium" => [
                local.map(|root| root.join("Chromium/Application/chrome.exe")),
                program.map(|root| root.join("Chromium/Application/chrome.exe")),
            ]
            .into_iter()
            .flatten()
            .collect(),
            "Edge" => [
                program_x86.map(|root| root.join("Microsoft/Edge/Application/msedge.exe")),
                program.map(|root| root.join("Microsoft/Edge/Application/msedge.exe")),
            ]
            .into_iter()
            .flatten()
            .collect(),
            "Brave" => [
                local.map(|root| root.join("BraveSoftware/Brave-Browser/Application/brave.exe")),
                program.map(|root| root.join("BraveSoftware/Brave-Browser/Application/brave.exe")),
                program_x86
                    .map(|root| root.join("BraveSoftware/Brave-Browser/Application/brave.exe")),
            ]
            .into_iter()
            .flatten()
            .collect(),
            _ => Vec::new(),
        };
        candidates.into_iter().find(|path| path.is_file())
    }

    #[cfg(test)]
    mod tests {
        use super::{Cookie, cookie_header};

        fn cookie(name: &str, value: &str, domain: &str) -> Cookie {
            Cookie {
                name: name.to_string(),
                value: value.to_string(),
                domain: domain.to_string(),
            }
        }

        #[test]
        fn builds_header_from_required_youtube_cookies() {
            let header = cookie_header(vec![
                cookie("SAPISID", "auth", ".youtube.com"),
                cookie("PREF", "preferences", "music.youtube.com"),
                cookie("unrelated", "ignored", ".youtube.com"),
                cookie("SID", "wrong-domain", ".notyoutube.com"),
            ])
            .unwrap();

            assert_eq!(header, "SAPISID=auth; PREF=preferences");
        }

        #[test]
        fn rejects_anonymous_cookie_sets() {
            assert!(cookie_header(vec![cookie("PREF", "guest", ".youtube.com")]).is_none());
        }
    }
}
