use gpui::prelude::*;
use gpui::{
    App, ClickEvent, Div, ElementId, FontWeight, Hsla, Interactivity, Pixels, SharedString,
    Stateful, StyleRefinement, Window, div, px, rgb,
};

use crate::artwork::Artwork;
use crate::metrics::{Text, snapped};
use crate::theme::ActiveTheme as _;

const RATIO: f32 = 0.5;
const ART: f32 = 0.38;
const INSET: Pixels = px(14.);
const LIFT: Pixels = px(8.);
const DIM: f32 = 0.62;
const HOVER: f32 = 0.86;

type Press = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

#[derive(IntoElement)]
pub struct Tile {
    base: Stateful<Div>,
    title: SharedString,
    width: Pixels,
    wash: Option<Hsla>,
    cover: Option<String>,
    press: Option<Press>,
}

impl Tile {
    #[track_caller]
    pub fn new(id: impl Into<ElementId>, title: impl Into<SharedString>, width: Pixels) -> Self {
        Self {
            base: div().id(id.into()),
            title: title.into(),
            width,
            wash: None,
            cover: None,
            press: None,
        }
    }

    pub fn wash(mut self, wash: Option<Hsla>) -> Self {
        self.wash = wash;
        self
    }

    pub fn cover(mut self, cover: Option<String>) -> Self {
        self.cover = cover;
        self
    }

    pub fn press(mut self, handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.press = Some(Box::new(handler));
        self
    }

    pub fn height(width: Pixels, window: &Window) -> Pixels {
        snapped(width * RATIO, window)
    }
}

pub fn paint(color: u32) -> Hsla {
    Hsla::from(rgb(color))
}

impl Styled for Tile {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl InteractiveElement for Tile {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl RenderOnce for Tile {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Self {
            mut base,
            title,
            width,
            wash,
            cover,
            press,
        } = self;
        let theme = *cx.theme();
        let overrides = std::mem::take(base.style());
        let wash = wash.unwrap_or(theme.secondary);
        let art = snapped(width * ART, window);

        let mut tile = base
            .relative()
            .flex_none()
            .w(width)
            .h(Tile::height(width, window))
            .overflow_hidden()
            .rounded(theme.radius)
            .bg(wash)
            .cursor_pointer()
            .hover(|style| style.opacity(HOVER))
            .p(INSET)
            .text_color(ink(wash))
            .text_size(theme.text(Text::Body))
            .font_weight(FontWeight::BOLD)
            .child(div().max_w(width - art - INSET).line_clamp(2).child(title))
            .child(
                div()
                    .absolute()
                    .bottom(-LIFT)
                    .right(-LIFT)
                    .w(art)
                    .h(art)
                    .shadow_md()
                    .child(Artwork::new(cover).size(art)),
            )
            .when_some(press, |this, press| {
                this.on_click(move |event, window, cx| press(event, window, cx))
            });

        tile.style().refine(&overrides);
        tile
    }
}

fn ink(wash: Hsla) -> Hsla {
    match wash.l > DIM {
        true => gpui::black(),
        false => gpui::white(),
    }
}
