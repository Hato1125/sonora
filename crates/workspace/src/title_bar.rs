use gpui::{Context, Corner, Entity, Render};
use gpui::{MouseButton, prelude::*};
use gpui::{Window, div, px};
use gpui_component::ActiveTheme as _;
use gpui_component::avatar::Avatar;
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{Input, InputState};
use gpui_component::label::Label;
use gpui_component::popover::Popover;
use gpui_component::skeleton::Skeleton;
use gpui_component::{Disableable as _, Icon};
use state::{Session, SessionState};

use crate::Sidebar;

const HEIGHT: f32 = 40.;

pub struct TitleBar {
    session: Entity<Session>,
    sidebar: Entity<Sidebar>,
}

impl TitleBar {
    pub fn new(session: Entity<Session>, sidebar: Entity<Sidebar>, cx: &mut Context<Self>) -> Self {
        cx.observe(&session, |_, _, cx| cx.notify()).detach();
        cx.observe(&sidebar, |_, _, cx| cx.notify()).detach();
        Self { session, sidebar }
    }
}

impl Render for TitleBar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();

        let session = self.session.clone();
        let sidebar = self.sidebar.clone();
        let sidebar_open = sidebar.read(cx).is_open();
        let sidebar_icon = if sidebar_open {
            "icons/panel-right-close.svg"
        } else {
            "icons/panel-right-open.svg"
        };
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Search..."));

        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .w_full()
            .h(px(HEIGHT))
            .flex_none()
            .px_5()
            .bg(theme.title_bar)
            .border_b_1()
            .border_color(theme.title_bar_border)
            .window_control_area(gpui::WindowControlArea::Drag)
            .on_mouse_down(MouseButton::Left, |_, window, _| {
                window.start_window_move();
            })
            .child(
                div()
                    .flex()
                    .w_12()
                    .flex_none()
                    .items_center()
                    .occlude()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| {
                        cx.stop_propagation();
                    })
                    .child(
                        div()
                            .id("sidebar-toggle")
                            .flex()
                            .h_16()
                            .w_16()
                            .ml(px(-8.))
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .child(Icon::default().path(sidebar_icon).size_5())
                            .on_click(move |_, _, cx| {
                                sidebar.update(cx, |sidebar, cx| sidebar.toggle(cx));
                            }),
                    ),
            )
            .child(
                div().flex().flex_1().items_center().justify_center().child(
                    div()
                        .flex()
                        .occlude()
                        .on_mouse_down(MouseButton::Left, |_, _, cx| {
                            cx.stop_propagation();
                        })
                        .child(
                            Input::new(&input)
                                .w((window.viewport_size().width - px(280.)).min(px(580.)))
                                .h_12()
                                .text_lg()
                                .rounded_none()
                                .rounded_l_full(),
                        )
                        .child(
                            Button::new("search")
                                .h_12()
                                .w_16()
                                .rounded_none()
                                .shadow_xs()
                                .rounded_r_full()
                                .outline()
                                .border_l_0()
                                .icon(Icon::default().path("icons/search.svg")),
                        ),
                ),
            )
            .child(match self.session.read(cx).state() {
                SessionState::SignedIn(profile) => {
                    let display_name = profile.display_name.clone();
                    let panel_name = display_name.clone();
                    let panel_session = session.clone();

                    div()
                        .occlude()
                        .on_mouse_down(MouseButton::Left, |_, _, cx| {
                            cx.stop_propagation();
                        })
                        .child(
                            Popover::new("profile-popover")
                                .anchor(Corner::TopRight)
                                .trigger(
                                    Button::new("profile-avatar")
                                        .ghost()
                                        .p_0()
                                        .size_12()
                                        .rounded_full()
                                        .child(Avatar::new().name(display_name).size_12()),
                                )
                                .content(move |_, _, _| {
                                    let session = panel_session.clone();

                                    div()
                                        .flex()
                                        .w(px(260.))
                                        .flex_col()
                                        .gap_2()
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap_3()
                                                .child(
                                                    Avatar::new()
                                                        .name(panel_name.clone())
                                                        .size_16(),
                                                )
                                                .child(
                                                    Label::new(panel_name.clone())
                                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                                        .truncate(),
                                                ),
                                        )
                                        .child(div().h(px(1.)).w_full().bg(theme.border))
                                        .child(
                                            Button::new("profile-settings")
                                                .label("Settings")
                                                .ghost()
                                                .w_full()
                                                .justify_start()
                                                .disabled(true),
                                        )
                                        .child(
                                            Button::new("profile-about")
                                                .label("About Spotty")
                                                .ghost()
                                                .w_full()
                                                .justify_start()
                                                .disabled(true),
                                        )
                                        .child(div().h(px(1.)).w_full().bg(theme.border))
                                        .child(
                                            Button::new("profile-sign-out")
                                                .label("Sign out")
                                                .ghost()
                                                .w_full()
                                                .justify_start()
                                                .icon(Icon::default().path("icons/log-out.svg"))
                                                .on_click(move |_, _, cx| {
                                                    session.update(cx, |session, cx| {
                                                        session.sign_out(cx)
                                                    });
                                                }),
                                        )
                                }),
                        )
                        .into_any_element()
                }
                _ => Skeleton::new().size_12().rounded_full().into_any_element(),
            })
    }
}
