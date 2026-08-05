use gpui::{App, Global, Hsla, Pixels, px, rgb, rgba};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeKind {
    Dark,
    Light,
    Midnight,
    Forest,
    Ocean,
    Rose,
    Lavender,
    Amber,
}

impl ThemeKind {
    pub const ALL: [Self; 8] = [
        Self::Dark,
        Self::Light,
        Self::Midnight,
        Self::Forest,
        Self::Ocean,
        Self::Rose,
        Self::Lavender,
        Self::Amber,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
            Self::Midnight => "midnight",
            Self::Forest => "forest",
            Self::Ocean => "ocean",
            Self::Rose => "rose",
            Self::Lavender => "lavender",
            Self::Amber => "amber",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Dark => "Dark",
            Self::Light => "Light",
            Self::Midnight => "Midnight",
            Self::Forest => "Forest",
            Self::Ocean => "Ocean",
            Self::Rose => "Rose",
            Self::Lavender => "Lavender",
            Self::Amber => "Amber",
        }
    }

    pub fn from_id(id: &str) -> Self {
        match id {
            "light" => Self::Light,
            "midnight" => Self::Midnight,
            "forest" => Self::Forest,
            "ocean" => Self::Ocean,
            "rose" => Self::Rose,
            "lavender" => Self::Lavender,
            "amber" => Self::Amber,
            _ => Self::Dark,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Theme {
    pub background: Hsla,
    pub foreground: Hsla,
    pub border: Hsla,
    pub muted: Hsla,
    pub muted_foreground: Hsla,
    pub secondary: Hsla,
    pub secondary_hover: Hsla,
    pub secondary_active: Hsla,
    pub primary: Hsla,
    pub primary_foreground: Hsla,
    pub primary_hover: Hsla,
    pub danger: Hsla,
    pub danger_foreground: Hsla,
    pub danger_hover: Hsla,
    pub popover: Hsla,
    pub popover_foreground: Hsla,
    pub progress_bar: Hsla,
    pub selection: Hsla,
    pub sidebar: Hsla,
    pub sidebar_accent: Hsla,
    pub sidebar_border: Hsla,
    pub title_bar_border: Hsla,
    pub table_head: Hsla,
    pub table_head_foreground: Hsla,
    pub table_row_border: Hsla,
    pub table_hover: Hsla,
    pub table_active: Hsla,
    pub table_active_border: Hsla,
    pub radius: Pixels,
    pub font_size: Pixels,
}

impl Global for Theme {}

impl Theme {
    pub fn dark() -> Self {
        Self {
            background: rgb(0x0a0a0a).into(),
            foreground: rgb(0xfafafa).into(),
            border: rgb(0x262626).into(),
            muted: rgb(0x262626).into(),
            muted_foreground: rgb(0x737373).into(),
            secondary: rgb(0x171717).into(),
            secondary_hover: rgb(0x1a1a1a).into(),
            secondary_active: rgb(0x262626).into(),
            primary: rgb(0xfafafa).into(),
            primary_foreground: rgb(0x171717).into(),
            primary_hover: rgb(0xe5e5e5).into(),
            danger: rgb(0x7f1d1d).into(),
            danger_foreground: rgb(0xfef2f2).into(),
            danger_hover: rgb(0x8b2020).into(),
            popover: rgb(0x0a0a0a).into(),
            popover_foreground: rgb(0xfafafa).into(),
            progress_bar: rgb(0xf5f5f5).into(),
            selection: rgb(0x1d4ed8).into(),
            sidebar: rgb(0x0a0a0a).into(),
            sidebar_accent: rgb(0x262626).into(),
            sidebar_border: rgb(0x262626).into(),
            title_bar_border: rgb(0x262626).into(),
            table_head: rgba(0x171717cc).into(),
            table_head_foreground: rgb(0x525252).into(),
            table_row_border: rgba(0x262626b3).into(),
            table_hover: rgb(0x262626).into(),
            table_active: rgba(0x1e40af33).into(),
            table_active_border: rgb(0x1d4ed8).into(),
            radius: px(6.),
            font_size: px(13.),
        }
    }

    pub fn light() -> Self {
        Self {
            background: rgb(0xfafafa).into(),
            foreground: rgb(0x171717).into(),
            border: rgb(0xd4d4d4).into(),
            muted: rgb(0xe5e5e5).into(),
            muted_foreground: rgb(0x737373).into(),
            secondary: rgb(0xf5f5f5).into(),
            secondary_hover: rgb(0xe5e5e5).into(),
            secondary_active: rgb(0xd4d4d4).into(),
            primary: rgb(0x171717).into(),
            primary_foreground: rgb(0xfafafa).into(),
            primary_hover: rgb(0x262626).into(),
            danger: rgb(0xb91c1c).into(),
            danger_foreground: rgb(0xfef2f2).into(),
            danger_hover: rgb(0x991b1b).into(),
            popover: rgb(0xffffff).into(),
            popover_foreground: rgb(0x171717).into(),
            progress_bar: rgb(0x262626).into(),
            selection: rgb(0x2563eb).into(),
            sidebar: rgb(0xf5f5f5).into(),
            sidebar_accent: rgb(0xe5e5e5).into(),
            sidebar_border: rgb(0xd4d4d4).into(),
            title_bar_border: rgb(0xd4d4d4).into(),
            table_head: rgba(0xf5f5f5e6).into(),
            table_head_foreground: rgb(0x737373).into(),
            table_row_border: rgba(0xd4d4d4b3).into(),
            table_hover: rgb(0xf0f0f0).into(),
            table_active: rgba(0x2563eb1f).into(),
            table_active_border: rgb(0x2563eb).into(),
            radius: px(6.),
            font_size: px(13.),
        }
    }

    pub fn midnight() -> Self {
        Self {
            background: rgb(0x07111f).into(),
            foreground: rgb(0xe6edf7).into(),
            border: rgb(0x1e344d).into(),
            muted: rgb(0x15283d).into(),
            muted_foreground: rgb(0x8296ad).into(),
            secondary: rgb(0x102238).into(),
            secondary_hover: rgb(0x17304d).into(),
            secondary_active: rgb(0x1e3b5d).into(),
            primary: rgb(0x38bdf8).into(),
            primary_foreground: rgb(0x07111f).into(),
            primary_hover: rgb(0x7dd3fc).into(),
            danger: rgb(0x991b1b).into(),
            danger_foreground: rgb(0xfff1f2).into(),
            danger_hover: rgb(0xb91c1c).into(),
            popover: rgb(0x0b1a2c).into(),
            popover_foreground: rgb(0xe6edf7).into(),
            progress_bar: rgb(0x38bdf8).into(),
            selection: rgb(0x0284c7).into(),
            sidebar: rgb(0x091827).into(),
            sidebar_accent: rgb(0x17304d).into(),
            sidebar_border: rgb(0x1e344d).into(),
            title_bar_border: rgb(0x1e344d).into(),
            table_head: rgba(0x102238e6).into(),
            table_head_foreground: rgb(0x8296ad).into(),
            table_row_border: rgba(0x1e344db3).into(),
            table_hover: rgb(0x132b45).into(),
            table_active: rgba(0x0284c733).into(),
            table_active_border: rgb(0x38bdf8).into(),
            radius: px(6.),
            font_size: px(13.),
        }
    }

    pub fn forest() -> Self {
        Self {
            background: rgb(0x0b1410).into(),
            foreground: rgb(0xecf7ef).into(),
            border: rgb(0x263d30).into(),
            muted: rgb(0x203328).into(),
            muted_foreground: rgb(0x86a58f).into(),
            secondary: rgb(0x16261d).into(),
            secondary_hover: rgb(0x203328).into(),
            secondary_active: rgb(0x2a4334).into(),
            primary: rgb(0x86efac).into(),
            primary_foreground: rgb(0x0b1410).into(),
            primary_hover: rgb(0xbbf7d0).into(),
            danger: rgb(0x991b1b).into(),
            danger_foreground: rgb(0xfff1f2).into(),
            danger_hover: rgb(0xb91c1c).into(),
            popover: rgb(0x101d16).into(),
            popover_foreground: rgb(0xecf7ef).into(),
            progress_bar: rgb(0x4ade80).into(),
            selection: rgb(0x16a34a).into(),
            sidebar: rgb(0x0d1812).into(),
            sidebar_accent: rgb(0x203328).into(),
            sidebar_border: rgb(0x263d30).into(),
            title_bar_border: rgb(0x263d30).into(),
            table_head: rgba(0x16261de6).into(),
            table_head_foreground: rgb(0x86a58f).into(),
            table_row_border: rgba(0x263d30b3).into(),
            table_hover: rgb(0x1b2e23).into(),
            table_active: rgba(0x16a34a33).into(),
            table_active_border: rgb(0x4ade80).into(),
            radius: px(6.),
            font_size: px(13.),
        }
    }

    pub fn ocean() -> Self {
        let mut theme = Self::midnight();
        theme.background = rgb(0x06171a).into();
        theme.border = rgb(0x1d4145).into();
        theme.muted = rgb(0x17373b).into();
        theme.muted_foreground = rgb(0x7fa9ad).into();
        theme.secondary = rgb(0x0f292d).into();
        theme.secondary_hover = rgb(0x17373b).into();
        theme.secondary_active = rgb(0x20474c).into();
        theme.primary = rgb(0x5eead4).into();
        theme.primary_foreground = rgb(0x06171a).into();
        theme.primary_hover = rgb(0x99f6e4).into();
        theme.popover = rgb(0x0a2024).into();
        theme.progress_bar = rgb(0x2dd4bf).into();
        theme.selection = rgb(0x0d9488).into();
        theme.sidebar = rgb(0x081c1f).into();
        theme.sidebar_accent = rgb(0x17373b).into();
        theme.sidebar_border = rgb(0x1d4145).into();
        theme.title_bar_border = rgb(0x1d4145).into();
        theme.table_head = rgba(0x0f292de6).into();
        theme.table_row_border = rgba(0x1d4145b3).into();
        theme.table_hover = rgb(0x123136).into();
        theme.table_active = rgba(0x0d948833).into();
        theme.table_active_border = rgb(0x2dd4bf).into();
        theme
    }

    pub fn rose() -> Self {
        let mut theme = Self::dark();
        theme.background = rgb(0x180b10).into();
        theme.border = rgb(0x4b2633).into();
        theme.muted = rgb(0x3b2029).into();
        theme.muted_foreground = rgb(0xb58a98).into();
        theme.secondary = rgb(0x2b161e).into();
        theme.secondary_hover = rgb(0x3b2029).into();
        theme.secondary_active = rgb(0x4b2633).into();
        theme.primary = rgb(0xfda4af).into();
        theme.primary_foreground = rgb(0x180b10).into();
        theme.primary_hover = rgb(0xfecdd3).into();
        theme.popover = rgb(0x211018).into();
        theme.progress_bar = rgb(0xfb7185).into();
        theme.selection = rgb(0xe11d48).into();
        theme.sidebar = rgb(0x1c0d13).into();
        theme.sidebar_accent = rgb(0x3b2029).into();
        theme.sidebar_border = rgb(0x4b2633).into();
        theme.title_bar_border = rgb(0x4b2633).into();
        theme.table_head = rgba(0x2b161ee6).into();
        theme.table_row_border = rgba(0x4b2633b3).into();
        theme.table_hover = rgb(0x341b24).into();
        theme.table_active = rgba(0xe11d4833).into();
        theme.table_active_border = rgb(0xfb7185).into();
        theme
    }

    pub fn lavender() -> Self {
        let mut theme = Self::dark();
        theme.background = rgb(0x120e1c).into();
        theme.border = rgb(0x3d3158).into();
        theme.muted = rgb(0x302745).into();
        theme.muted_foreground = rgb(0xa99bc2).into();
        theme.secondary = rgb(0x241c35).into();
        theme.secondary_hover = rgb(0x302745).into();
        theme.secondary_active = rgb(0x3d3158).into();
        theme.primary = rgb(0xc4b5fd).into();
        theme.primary_foreground = rgb(0x120e1c).into();
        theme.primary_hover = rgb(0xddd6fe).into();
        theme.popover = rgb(0x191326).into();
        theme.progress_bar = rgb(0xa78bfa).into();
        theme.selection = rgb(0x7c3aed).into();
        theme.sidebar = rgb(0x161020).into();
        theme.sidebar_accent = rgb(0x302745).into();
        theme.sidebar_border = rgb(0x3d3158).into();
        theme.title_bar_border = rgb(0x3d3158).into();
        theme.table_head = rgba(0x241c35e6).into();
        theme.table_row_border = rgba(0x3d3158b3).into();
        theme.table_hover = rgb(0x2a213d).into();
        theme.table_active = rgba(0x7c3aed33).into();
        theme.table_active_border = rgb(0xa78bfa).into();
        theme
    }

    pub fn amber() -> Self {
        let mut theme = Self::dark();
        theme.background = rgb(0x171108).into();
        theme.border = rgb(0x49371d).into();
        theme.muted = rgb(0x382b18).into();
        theme.muted_foreground = rgb(0xad9878).into();
        theme.secondary = rgb(0x291f11).into();
        theme.secondary_hover = rgb(0x382b18).into();
        theme.secondary_active = rgb(0x49371d).into();
        theme.primary = rgb(0xfcd34d).into();
        theme.primary_foreground = rgb(0x171108).into();
        theme.primary_hover = rgb(0xfde68a).into();
        theme.popover = rgb(0x20170c).into();
        theme.progress_bar = rgb(0xf59e0b).into();
        theme.selection = rgb(0xd97706).into();
        theme.sidebar = rgb(0x1b1409).into();
        theme.sidebar_accent = rgb(0x382b18).into();
        theme.sidebar_border = rgb(0x49371d).into();
        theme.title_bar_border = rgb(0x49371d).into();
        theme.table_head = rgba(0x291f11e6).into();
        theme.table_row_border = rgba(0x49371db3).into();
        theme.table_hover = rgb(0x312514).into();
        theme.table_active = rgba(0xd9770633).into();
        theme.table_active_border = rgb(0xf59e0b).into();
        theme
    }

    pub fn for_kind(kind: ThemeKind) -> Self {
        match kind {
            ThemeKind::Dark => Self::dark(),
            ThemeKind::Light => Self::light(),
            ThemeKind::Midnight => Self::midnight(),
            ThemeKind::Forest => Self::forest(),
            ThemeKind::Ocean => Self::ocean(),
            ThemeKind::Rose => Self::rose(),
            ThemeKind::Lavender => Self::lavender(),
            ThemeKind::Amber => Self::amber(),
        }
    }

    pub fn init(kind: ThemeKind, cx: &mut App) {
        cx.set_global(Self::for_kind(kind));
    }

    pub fn set(kind: ThemeKind, cx: &mut App) {
        cx.set_global(Self::for_kind(kind));
        cx.refresh_windows();
    }
}

pub trait ActiveTheme {
    fn theme(&self) -> &Theme;
}

impl ActiveTheme for App {
    fn theme(&self) -> &Theme {
        self.global::<Theme>()
    }
}
