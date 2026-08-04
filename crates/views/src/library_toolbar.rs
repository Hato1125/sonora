use gpui::prelude::*;
use gpui::{Context, Entity, MouseButton, Render, SharedString, Window, div};
use gpui_component::button::Button;
use gpui_component::{Disableable as _, Selectable as _, Sizable as _};

use crate::library::{LibraryView, Section};

pub struct LibraryToolbar {
    view: Entity<LibraryView>,
}

impl LibraryToolbar {
    pub fn new(view: Entity<LibraryView>, cx: &mut Context<Self>) -> Self {
        cx.observe(&view, |_, _, cx| cx.notify()).detach();
        Self { view }
    }

    fn tab(&self, section: Section, count: usize, cx: &mut Context<Self>) -> Button {
        let label = if count == 0 {
            section.label().to_owned()
        } else {
            format!("{} ({count})", section.label())
        };
        let selected = self.view.read(cx).section() == section;

        Button::new(SharedString::from(section.label()))
            .label(label)
            .small()
            .selected(selected)
            .disabled(count == 0)
            .on_click(cx.listener(move |this, _, _, cx| {
                this.view.update(cx, |view, cx| view.select(section, cx));
            }))
    }
}

impl Render for LibraryToolbar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (tracks, playlists) = self.view.read(cx).counts(cx);

        div()
            .flex()
            .flex_1()
            .items_center()
            .justify_between()
            .gap_3()
            .child(
                div()
                    .flex()
                    .flex_none()
                    .gap_1()
                    .occlude()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(self.tab(Section::Tracks, tracks, cx))
                    .child(self.tab(Section::Playlists, playlists, cx)),
            )
    }
}
