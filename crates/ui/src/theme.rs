use gpui::{App, Global, Hsla, Pixels, px, rgb, rgba};

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

    pub fn init(cx: &mut App) {
        cx.set_global(Self::dark());
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
