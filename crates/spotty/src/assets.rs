use std::borrow::Cow;

use anyhow::Result;
use gpui::{AssetSource, SharedString};

macro_rules! icons {
    ($($name:literal),* $(,)?) => {
        &[$((concat!("icons/", $name, ".svg"), include_bytes!(concat!("../../../assets/icons/", $name, ".svg")).as_slice())),*]
    };
}

const ICONS: &[(&str, &[u8])] = icons![
    "chevron-down",
    "columns-3",
    "chevron-left",
    "chevron-right",
    "chevrons-up-down",
    "chevron-up",
    "house",
    "library-big",
    "log-out",
    "music",
    "music-2",
    "pause",
    "panel-right-close",
    "panel-right-open",
    "play",
    "play-off",
    "refresh-cw",
    "search",
    "settings",
    "skip-back",
    "skip-forward",
    "volume-1",
    "volume-2",
    "volume-x",
    "window-close",
    "window-maximize",
    "window-minimize",
    "window-restore",
    "x",
];

pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if let Some((_, bytes)) = ICONS.iter().find(|(name, _)| *name == path) {
            return Ok(Some(Cow::Borrowed(bytes)));
        }
        log::warn!("assets: {path} is not registered in ICONS");
        Ok(None)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(ICONS
            .iter()
            .filter(|(name, _)| name.starts_with(path))
            .map(|(name, _)| SharedString::from(*name))
            .collect())
    }
}
