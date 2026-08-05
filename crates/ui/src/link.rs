use gpui::prelude::*;
use gpui::{App, Div, MouseButton, Stateful, Window};

pub trait Linked: Sized {
    fn link(self, press: impl Fn(&mut Window, &mut App) + 'static) -> Self;
}

impl Linked for Stateful<Div> {
    fn link(self, press: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.cursor_pointer()
            .hover(|style| style.underline())
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_click(move |_, window, cx| press(window, cx))
    }
}
