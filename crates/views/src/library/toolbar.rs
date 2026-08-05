use gpui::prelude::*;
use gpui::{Context, Entity, MouseButton, Render, SharedString, Window, div};
use ui::Button;

use super::{LibraryView, Section};
use router::{Destination, LibraryTab, navigate};

pub struct LibraryToolbar {
    view: Entity<LibraryView>,
}

impl LibraryToolbar {
    pub fn new(view: Entity<LibraryView>, cx: &mut Context<Self>) -> Self {
        cx.observe(&view, |_, _, cx| cx.notify()).detach();
        Self { view }
    }

    fn tab(&self, section: Section, count: usize, cx: &mut Context<Self>) -> Button {
        let label = section.label().to_owned();
        let selected = self.view.read(cx).section() == section;

        Button::new(SharedString::from(section.label()))
            .label(label)
            .small()
            .selected(selected)
            .disabled(count == 0)
            .on_click(cx.listener(move |_, _, _, cx| {
                navigate(Destination::Library(LibraryTab::from(section)), cx);
            }))
    }
}

impl Render for LibraryToolbar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let counts = self.view.read(cx).counts(cx);

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
                    .child(self.tab(Section::Tracks, counts.tracks, cx))
                    .child(self.tab(Section::Albums, counts.albums, cx))
                    .child(self.tab(Section::Playlists, counts.playlists, cx)),
            )
    }
}
