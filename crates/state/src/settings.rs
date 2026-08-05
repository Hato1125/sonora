use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use gpui::{Context, Task};
use serde::{Deserialize, Serialize};

const SAVE_DELAY: Duration = Duration::from_millis(300);
const DEFAULT_VOLUME: f32 = 0.7;
const DEFAULT_SIDEBAR_WIDTH: f32 = 220.;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
struct Values {
    version: u32,
    volume: f32,
    normalisation: bool,
    sidebar_width: f32,
    sidebar_open: bool,
    theme: String,
}

impl Default for Values {
    fn default() -> Self {
        Self {
            version: 1,
            volume: DEFAULT_VOLUME,
            normalisation: true,
            sidebar_width: DEFAULT_SIDEBAR_WIDTH,
            sidebar_open: true,
            theme: "dark".to_owned(),
        }
    }
}

pub struct AppSettings {
    values: Values,
    path: PathBuf,
    save: Option<Task<()>>,
}

impl AppSettings {
    pub fn load() -> Self {
        let path = settings_path();
        let values = match fs::read(&path) {
            Ok(bytes) => serde_json::from_slice::<Values>(&bytes).unwrap_or_else(|error| {
                log::warn!("settings: cannot parse {}: {error}", path.display());
                Values::default()
            }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Values::default(),
            Err(error) => {
                log::warn!("settings: cannot read {}: {error}", path.display());
                Values::default()
            }
        };

        Self {
            values,
            path,
            save: None,
        }
    }

    pub fn volume(&self) -> f32 {
        self.values.volume.clamp(0., 1.)
    }

    pub fn normalisation(&self) -> bool {
        self.values.normalisation
    }

    pub fn sidebar_width(&self) -> f32 {
        self.values.sidebar_width
    }

    pub fn sidebar_open(&self) -> bool {
        self.values.sidebar_open
    }

    pub fn theme(&self) -> &str {
        &self.values.theme
    }

    pub fn set_volume(&mut self, volume: f32, cx: &mut Context<Self>) {
        self.values.volume = volume.clamp(0., 1.);
        self.schedule_save(cx);
    }

    pub fn set_normalisation(&mut self, normalisation: bool, cx: &mut Context<Self>) {
        self.values.normalisation = normalisation;
        self.schedule_save(cx);
    }

    pub fn set_sidebar(&mut self, width: f32, open: bool, cx: &mut Context<Self>) {
        self.values.sidebar_width = width;
        self.values.sidebar_open = open;
        self.schedule_save(cx);
    }

    pub fn set_theme(&mut self, theme: impl Into<String>, cx: &mut Context<Self>) {
        self.values.theme = theme.into();
        self.schedule_save(cx);
    }

    fn schedule_save(&mut self, cx: &mut Context<Self>) {
        cx.notify();
        self.save = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(SAVE_DELAY).await;
            this.update(cx, |this, _| this.save_now()).ok();
        }));
    }

    fn save_now(&self) {
        let Some(parent) = self.path.parent() else {
            return;
        };
        if let Err(error) = fs::create_dir_all(parent) {
            log::error!("settings: cannot create {}: {error}", parent.display());
            return;
        }

        let bytes = match serde_json::to_vec_pretty(&self.values) {
            Ok(bytes) => bytes,
            Err(error) => {
                log::error!("settings: cannot serialize values: {error}");
                return;
            }
        };
        if let Err(error) = fs::write(&self.path, bytes) {
            log::error!("settings: cannot write {}: {error}", self.path.display());
        }
    }
}

fn settings_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("spotty")
        .join("settings.json")
}
