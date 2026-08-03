use gpui::{App, ClickEvent, Window};

pub trait Disableable {
    fn disabled(self, disabled: bool) -> Self;
}

pub trait Clickable {
    fn on_click(self, handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self;
}

pub trait Selectable {
    fn selected(self, selected: bool) -> Self;
}
