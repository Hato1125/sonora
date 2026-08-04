use gpui::prelude::*;
use gpui::{AnyView, Context, Entity, MouseButton, Pixels, Render};
use gpui::{Window, div, px};
use gpui_component::ActiveTheme as _;
use gpui_component::Icon;

use crate::Sidebar;

const HEIGHT: f32 = 36.;

pub struct TitleBar {
    sidebar: Entity<Sidebar>,
    content: Option<AnyView>,
}

impl TitleBar {
    pub fn new(sidebar: Entity<Sidebar>, cx: &mut Context<Self>) -> Self {
        cx.observe(&sidebar, |_, _, cx| cx.notify()).detach();
        Self {
            sidebar,
            content: None,
        }
    }

    pub fn set_content(&mut self, content: Option<AnyView>, cx: &mut Context<Self>) {
        self.content = content;
        cx.notify();
    }

    fn toggle(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let sidebar = self.sidebar.clone();
        let hover = cx.theme().sidebar_accent;
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
                    .child(Icon::default().path(icon).size_4())
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
                    .when(offset > Pixels::ZERO, |this| this.w(offset))
                    .child(self.toggle(cx)),
            )
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_w_0()
                    .items_center()
                    .pr_3()
                    .children(self.content.clone()),
            )
    }
}
