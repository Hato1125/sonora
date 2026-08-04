use gpui::prelude::*;
use gpui::{Context, Entity, MouseButton, Render, SharedString, Window, div};
use gpui_component::button::Button;
use gpui_component::{Disableable as _, Selectable as _, Sizable as _};

use super::{LibraryView, Section};
use workspace::{Destination, LibraryTab, Navigation};

pub struct LibraryToolbar {
    view: Entity<LibraryView>,
    navigation: Entity<Navigation>,
}

impl LibraryToolbar {
    pub fn new(
        view: Entity<LibraryView>,
        navigation: Entity<Navigation>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&view, |_, _, cx| cx.notify()).detach();
        cx.observe(&navigation, |_, _, cx| cx.notify()).detach();
        Self { view, navigation }
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
                let destination = Destination::Library(LibraryTab::from(section));
                this.navigation
                    .update(cx, |navigation, cx| navigation.go(destination, cx));
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
