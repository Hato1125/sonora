use gpui::Hsla;
use theme::Theme;

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
            Tone::Default => theme.text,
            Tone::Muted => theme.text_muted,
            Tone::Accent => theme.accent,
            Tone::Danger => theme.danger,
        }
    }
}
