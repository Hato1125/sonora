use gpui::{App, Global, Hsla, rgb};

#[derive(Clone, Debug)]
pub struct Theme {
    pub background: Hsla,
    pub surface: Hsla,
    pub elevated: Hsla,
    pub border: Hsla,
    pub text: Hsla,
    pub text_muted: Hsla,
    pub accent: Hsla,
    pub accent_hover: Hsla,
    pub on_accent: Hsla,
    pub danger: Hsla,
}

impl Global for Theme {}

impl Default for Theme {
    fn default() -> Self {
        Self {
            background: rgb(0x0d0d0f).into(),
            surface: rgb(0x16161a).into(),
            elevated: rgb(0x22222a).into(),
            border: rgb(0x2c2c35).into(),
            text: rgb(0xf2f2f5).into(),
            text_muted: rgb(0x9b9ba6).into(),
            accent: rgb(0x1db954).into(),
            accent_hover: rgb(0x23d765).into(),
            on_accent: rgb(0x07130c).into(),
            danger: rgb(0xf2545b).into(),
        }
    }
}

impl Theme {
    pub fn global(cx: &App) -> &Self {
        cx.global()
    }
}

pub fn init(cx: &mut App) {
    cx.set_global(Theme::default());
}
