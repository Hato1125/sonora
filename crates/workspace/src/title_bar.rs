use gpui::prelude::*;
use gpui::{AnyView, Context, Entity, MouseButton, Pixels, Render};
use gpui::{Window, div, px, svg};
use ui::ActiveTheme as _;

use crate::{Navigation, Sidebar};

const HEIGHT: f32 = 36.;

pub struct TitleBar {
    sidebar: Entity<Sidebar>,
    navigation: Entity<Navigation>,
    content: Option<AnyView>,
}

impl TitleBar {
    pub fn new(
        sidebar: Entity<Sidebar>,
        navigation: Entity<Navigation>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&sidebar, |_, _, cx| cx.notify()).detach();
        cx.observe(&navigation, |_, _, cx| cx.notify()).detach();
        Self {
            sidebar,
            navigation,
            content: None,
        }
    }

    pub fn set_content(&mut self, content: Option<AnyView>, cx: &mut Context<Self>) {
        self.content = content;
        cx.notify();
    }

    fn chrome(
        id: &'static str,
        icon: &'static str,
        enabled: bool,
        hover: gpui::Hsla,
        muted: gpui::Hsla,
        on_click: impl Fn(&mut Window, &mut gpui::App) + 'static,
    ) -> impl IntoElement {
        div()
            .id(id)
            .flex()
            .size_8()
            .items_center()
            .justify_center()
            .rounded_md()
            .when(enabled, |this| {
                this.cursor_pointer().hover(move |this| this.bg(hover))
            })
            .child(svg().path(icon).size_4().text_color(if enabled {
                muted
            } else {
                muted.opacity(0.4)
            }))
            .when(enabled, |this| {
                this.on_click(move |_, window, cx| on_click(window, cx))
            })
    }

    fn history(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let hover = cx.theme().sidebar_accent;
        let muted = cx.theme().muted_foreground;
        let navigation = self.navigation.read(cx);
        let (can_back, can_forward) = (navigation.can_go_back(), navigation.can_go_forward());
        let back = self.navigation.clone();
        let forward = self.navigation.clone();

        div()
            .flex()
            .flex_none()
            .items_center()
            .gap_1()
            .occlude()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(Self::chrome(
                "history-back",
                "icons/chevron-left.svg",
                can_back,
                hover,
                muted,
                move |_, cx| back.update(cx, |navigation, cx| navigation.back(cx)),
            ))
            .child(Self::chrome(
                "history-forward",
                "icons/chevron-right.svg",
                can_forward,
                hover,
                muted,
                move |_, cx| forward.update(cx, |navigation, cx| navigation.forward(cx)),
            ))
    }

    fn toggle(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let sidebar = self.sidebar.clone();
        let hover = cx.theme().sidebar_accent;
        let icon_color = cx.theme().foreground;
        let icon = if sidebar.read(cx).is_open() {
            "icons/panel-right-close.svg"
        } else {
            "icons/panel-right-open.svg"
        };

        div()
            .flex()
            .flex_none()
            .items_center()
            .occlude()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .id("sidebar-toggle")
                    .flex()
                    .size_8()
                    .items_center()
                    .justify_center()
                    .rounded_md()
                    .cursor_pointer()
                    .hover(move |this| this.bg(hover))
                    .child(svg().path(icon).size_4().text_color(icon_color))
                    .on_click(move |_, _, cx| {
                        sidebar.update(cx, |sidebar, cx| sidebar.toggle(cx));
                    }),
            )
    }
}

impl Render for TitleBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let offset = self.sidebar.read(cx).occupied_width();

        div()
            .flex()
            .items_center()
            .w_full()
            .h(px(HEIGHT))
            .flex_none()
            .bg(theme.background)
            .border_b_1()
            .border_color(theme.title_bar_border)
            .window_control_area(gpui::WindowControlArea::Drag)
            .on_mouse_down(MouseButton::Left, |_, window, _| {
                window.start_window_move();
            })
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .px_3()
                    .gap_1()
                    .when(offset > Pixels::ZERO, |this| this.w(offset))
                    .child(self.toggle(cx)),
            )
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_w_0()
                    .gap_1()
                    .items_center()
                    .child(self.history(cx))
                    .children(self.content.clone()),
            )
    }
}
