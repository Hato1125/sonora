use gpui::Hsla;
use gpui_component::Theme;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Tone {
    #[default]
    Default,
    Muted,
    Accent,
    Danger,
}

impl Tone {
    pub fn color(self, theme: &Theme) -> Hsla {
        match self {
            Tone::Default => theme.foreground,
            Tone::Muted => theme.muted_foreground,
            Tone::Accent => theme.primary,
            Tone::Danger => theme.danger,
        }
    }
}
