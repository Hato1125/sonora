// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use gpui::{Context, Task};
use serde::{Deserialize, Serialize};
use ui::{Layout, Look, Mode, Pin, Rounding, Sorting, ThemeKind, ThemeOverrides};

use crate::Repeat;
use crate::queue::gap_target;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SideTab {
    #[default]
    Queue,
    Lyrics,
}

const SAVE_DELAY: Duration = Duration::from_millis(300);
const DEFAULT_VOLUME: f32 = 0.7;
const DEFAULT_SIDEBAR_WIDTH: f32 = 220.;
const DEFAULT_SIDEBAR_RIGHT_WIDTH: f32 = 380.;
const DEFAULT_FONT_SIZE: f32 = 14.;
const DEFAULT_STARTUP: &str = "home";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
struct Values {
    version: u32,
    volume: f32,
    normalisation: bool,
    gapless: bool,
    sidebar_width: f32,
    sidebar_open: bool,
    sidebar_right_width: f32,
    sidebar_right_open: bool,
    sidebar_right_tab: SideTab,
    shuffle: bool,
    repeat: Repeat,
    language: String,
    provider: String,
    startup: String,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    hidden_columns: HashMap<String, Vec<String>>,
    tables: HashMap<String, Layout>,
    sorting: HashMap<String, Option<Sorting>>,
    views: HashMap<String, Mode>,
    pinned: Vec<Pin>,
    appearance: Appearance,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
struct Appearance {
    theme: String,
    adaptive_theme: bool,
    rounding: String,
    font_size: f32,
    transparent: bool,
    transparency: f32,
    window_controls: bool,
    controls_on_left: bool,
    theme_overrides: ThemeOverrides,
}

impl Default for Values {
    fn default() -> Self {
        Self {
            version: 1,
            volume: DEFAULT_VOLUME,
            normalisation: true,
            gapless: true,
            sidebar_width: DEFAULT_SIDEBAR_WIDTH,
            sidebar_open: true,
            sidebar_right_width: DEFAULT_SIDEBAR_RIGHT_WIDTH,
            sidebar_right_open: false,
            sidebar_right_tab: SideTab::Queue,
            shuffle: false,
            repeat: Repeat::Off,
            language: i18n::AUTO.to_owned(),
            provider: "spotify".to_owned(),
            startup: DEFAULT_STARTUP.to_owned(),
            hidden_columns: HashMap::new(),
            tables: HashMap::new(),
            sorting: HashMap::new(),
            views: HashMap::new(),
            pinned: Vec::new(),
            appearance: Appearance::default(),
        }
    }
}

impl Values {
    fn migrate(&mut self) {
        for (table, hidden) in self.hidden_columns.drain() {
            self.tables.entry(table).or_insert_with(|| Layout {
                hidden,
                ..Layout::default()
            });
        }
    }
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            theme: "dark".to_owned(),
            adaptive_theme: false,
            rounding: "subtle".to_owned(),
            font_size: DEFAULT_FONT_SIZE,
            transparent: false,
            transparency: 0.15,
            window_controls: true,
            controls_on_left: false,
            theme_overrides: ThemeOverrides::default(),
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
        let mut values = match fs::read(&path) {
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
        values.migrate();

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

    pub fn gapless(&self) -> bool {
        self.values.gapless
    }

    pub fn sidebar_width(&self) -> f32 {
        self.values.sidebar_width
    }

    pub fn sidebar_open(&self) -> bool {
        self.values.sidebar_open
    }

    pub fn sidebar_right_width(&self) -> f32 {
        self.values.sidebar_right_width
    }

    pub fn sidebar_right_open(&self) -> bool {
        self.values.sidebar_right_open
    }

    pub fn sidebar_right_tab(&self) -> SideTab {
        self.values.sidebar_right_tab
    }

    pub fn shuffle(&self) -> bool {
        self.values.shuffle
    }

    pub fn repeat(&self) -> Repeat {
        self.values.repeat
    }

    pub fn language(&self) -> &str {
        &self.values.language
    }

    pub fn provider(&self) -> &str {
        &self.values.provider
    }

    pub fn startup(&self) -> &str {
        &self.values.startup
    }

    pub fn theme(&self) -> &str {
        &self.values.appearance.theme
    }

    pub fn adaptive_theme(&self) -> bool {
        self.values.appearance.adaptive_theme
    }

    pub fn rounding(&self) -> &str {
        &self.values.appearance.rounding
    }

    pub fn look(&self) -> Look {
        Look {
            kind: ThemeKind::from_id(self.theme()),
            rounding: Rounding::from_id(self.rounding()),
            font: self.font_size(),
            transparent: self.transparent(),
            transparency: self.transparency(),
            tint: None,
        }
    }

    pub fn window_controls(&self) -> bool {
        self.values.appearance.window_controls
    }

    pub fn controls_on_left(&self) -> bool {
        self.values.appearance.controls_on_left
    }

    pub fn font_size(&self) -> f32 {
        self.values
            .appearance
            .font_size
            .clamp(ui::MIN_FONT, ui::MAX_FONT)
    }

    pub fn transparent(&self) -> bool {
        self.values.appearance.transparent
    }

    pub fn transparency(&self) -> f32 {
        self.values
            .appearance
            .transparency
            .clamp(0., ui::MAX_TRANSPARENCY)
    }

    pub fn theme_overrides(&self) -> &ThemeOverrides {
        &self.values.appearance.theme_overrides
    }

    pub fn ensure_file(&self) -> PathBuf {
        if !self.path.exists() {
            self.save_now();
        }
        self.path.clone()
    }

    pub fn set_volume(&mut self, volume: f32, cx: &mut Context<Self>) {
        self.values.volume = volume.clamp(0., 1.);
        self.schedule_save(cx);
    }

    pub fn set_normalisation(&mut self, normalisation: bool, cx: &mut Context<Self>) {
        self.values.normalisation = normalisation;
        self.schedule_save(cx);
    }

    pub fn set_gapless(&mut self, gapless: bool, cx: &mut Context<Self>) {
        self.values.gapless = gapless;
        self.schedule_save(cx);
    }

    pub fn table(&self, table: &str) -> Layout {
        self.values.tables.get(table).cloned().unwrap_or_default()
    }

    pub fn set_table(&mut self, table: &str, layout: Layout, cx: &mut Context<Self>) {
        if self.values.tables.get(table) == Some(&layout) {
            return;
        }
        self.values.tables.insert(table.to_owned(), layout);
        self.schedule_save(cx);
    }

    pub fn view(&self, table: &str) -> Mode {
        self.values.views.get(table).copied().unwrap_or_default()
    }

    pub fn set_view(&mut self, table: &str, mode: Mode, cx: &mut Context<Self>) {
        if self.values.views.get(table) == Some(&mode) {
            return;
        }
        self.values.views.insert(table.to_owned(), mode);
        self.schedule_save(cx);
    }

    pub fn sorting(&self, table: &str) -> Option<Option<Sorting>> {
        self.values.sorting.get(table).cloned()
    }

    pub fn set_sorting(&mut self, table: &str, sorting: Option<Sorting>, cx: &mut Context<Self>) {
        if self.values.sorting.get(table) == Some(&sorting) {
            return;
        }
        self.values.sorting.insert(table.to_owned(), sorting);
        self.schedule_save(cx);
    }

    pub fn pinned(&self) -> &[Pin] {
        &self.values.pinned
    }

    pub fn pin(&mut self, pin: Pin, gap: Option<usize>, cx: &mut Context<Self>) {
        if !place(&mut self.values.pinned, pin, gap) {
            return;
        }
        self.schedule_save(cx);
    }

    pub fn unpin(&mut self, pin: &Pin, cx: &mut Context<Self>) {
        let Some(index) = self.values.pinned.iter().position(|it| it.same(pin)) else {
            return;
        };
        self.values.pinned.remove(index);
        self.schedule_save(cx);
    }

    pub fn set_sidebar(&mut self, width: f32, open: bool, cx: &mut Context<Self>) {
        self.values.sidebar_width = width;
        self.values.sidebar_open = open;
        self.schedule_save(cx);
    }

    pub fn set_sidebar_right_width(&mut self, width: f32, cx: &mut Context<Self>) {
        self.values.sidebar_right_width = width;
        self.schedule_save(cx);
    }

    pub fn set_sidebar_right_open(&mut self, open: bool, cx: &mut Context<Self>) {
        self.values.sidebar_right_open = open;
        self.schedule_save(cx);
    }

    pub fn set_sidebar_right_tab(&mut self, tab: SideTab, cx: &mut Context<Self>) {
        if self.values.sidebar_right_tab == tab {
            return;
        }
        self.values.sidebar_right_tab = tab;
        self.schedule_save(cx);
    }

    pub fn set_shuffle(&mut self, shuffle: bool, cx: &mut Context<Self>) {
        self.values.shuffle = shuffle;
        self.schedule_save(cx);
    }

    pub fn set_repeat(&mut self, repeat: Repeat, cx: &mut Context<Self>) {
        self.values.repeat = repeat;
        self.schedule_save(cx);
    }

    pub fn set_language(&mut self, language: impl Into<String>, cx: &mut Context<Self>) {
        self.values.language = language.into();
        i18n::set(i18n::resolve(&self.values.language));
        cx.refresh_windows();
        self.schedule_save(cx);
    }

    pub fn set_startup(&mut self, screen: impl Into<String>, cx: &mut Context<Self>) {
        let screen = screen.into();
        if self.values.startup == screen {
            return;
        }
        self.values.startup = screen;
        self.schedule_save(cx);
    }

    pub fn set_provider(&mut self, provider: impl Into<String>, cx: &mut Context<Self>) {
        let provider = provider.into();
        if self.values.provider == provider {
            return;
        }
        self.values.provider = provider;
        self.schedule_save(cx);
    }

    pub fn set_theme(&mut self, theme: impl Into<String>, cx: &mut Context<Self>) {
        self.values.appearance.theme = theme.into();
        self.schedule_save(cx);
    }

    pub fn set_adaptive_theme(&mut self, adaptive: bool, cx: &mut Context<Self>) {
        self.values.appearance.adaptive_theme = adaptive;
        self.schedule_save(cx);
    }

    pub fn set_rounding(&mut self, rounding: impl Into<String>, cx: &mut Context<Self>) {
        self.values.appearance.rounding = rounding.into();
        self.schedule_save(cx);
    }

    pub fn set_window_controls(&mut self, shown: bool, cx: &mut Context<Self>) {
        self.values.appearance.window_controls = shown;
        self.schedule_save(cx);
    }

    pub fn set_controls_on_left(&mut self, left: bool, cx: &mut Context<Self>) {
        self.values.appearance.controls_on_left = left;
        self.schedule_save(cx);
    }

    pub fn set_font_size(&mut self, size: f32, cx: &mut Context<Self>) {
        self.values.appearance.font_size = size.clamp(ui::MIN_FONT, ui::MAX_FONT);
        self.schedule_save(cx);
    }

    pub fn set_transparent(&mut self, transparent: bool, cx: &mut Context<Self>) {
        self.values.appearance.transparent = transparent;
        self.schedule_save(cx);
    }

    pub fn set_transparency(&mut self, transparency: f32, cx: &mut Context<Self>) {
        self.values.appearance.transparency = transparency.clamp(0., ui::MAX_TRANSPARENCY);
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
        .join("sonora")
        .join("settings.json")
}

fn place(pinned: &mut Vec<Pin>, pin: Pin, gap: Option<usize>) -> bool {
    let gap = gap.unwrap_or(pinned.len()).min(pinned.len());
    let Some(from) = pinned.iter().position(|it| it.same(&pin)) else {
        pinned.insert(gap, pin);
        return true;
    };

    let to = gap_target(from, gap, pinned.len());
    if to == from {
        return false;
    }

    let moved = pinned.remove(from);
    pinned.insert(to, moved);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui::PinKind;

    fn pin(id: &str) -> Pin {
        Pin::new(PinKind::Album, id, id)
    }

    fn ids(pinned: &[Pin]) -> Vec<&str> {
        pinned.iter().map(|it| it.id.as_str()).collect()
    }

    #[test]
    fn a_fresh_pin_lands_at_the_gap() {
        let mut pinned = vec![pin("a"), pin("b")];

        assert!(place(&mut pinned, pin("c"), Some(1)));
        assert_eq!(ids(&pinned), ["a", "c", "b"]);
    }

    #[test]
    fn no_gap_appends() {
        let mut pinned = vec![pin("a")];

        assert!(place(&mut pinned, pin("b"), None));
        assert_eq!(ids(&pinned), ["a", "b"]);
    }

    #[test]
    fn a_gap_past_the_end_still_appends() {
        let mut pinned = vec![pin("a")];

        assert!(place(&mut pinned, pin("b"), Some(9)));
        assert_eq!(ids(&pinned), ["a", "b"]);
    }

    #[test]
    fn pinning_twice_moves_instead_of_duplicating() {
        let mut pinned = vec![pin("a"), pin("b"), pin("c")];

        assert!(place(&mut pinned, pin("a"), Some(3)));
        assert_eq!(ids(&pinned), ["b", "c", "a"]);
    }

    #[test]
    fn a_move_backwards_keeps_the_gap() {
        let mut pinned = vec![pin("a"), pin("b"), pin("c")];

        assert!(place(&mut pinned, pin("c"), Some(0)));
        assert_eq!(ids(&pinned), ["c", "a", "b"]);
    }

    #[test]
    fn the_gaps_around_an_item_are_no_ops() {
        let mut pinned = vec![pin("a"), pin("b"), pin("c")];

        assert!(!place(&mut pinned, pin("b"), Some(1)));
        assert!(!place(&mut pinned, pin("b"), Some(2)));
        assert_eq!(ids(&pinned), ["a", "b", "c"]);
    }

    #[test]
    fn kinds_with_the_same_id_stay_apart() {
        let mut pinned = vec![Pin::new(PinKind::Album, "x", "x")];

        assert!(place(&mut pinned, Pin::new(PinKind::Song, "x", "x"), None));
        assert_eq!(pinned.len(), 2);
    }
}
