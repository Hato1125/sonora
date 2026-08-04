use gpui::{Context, Entity, Render};
use gpui::{MouseButton, prelude::*};
use gpui::{Window, div, px};
use gpui_component::ActiveTheme as _;
use gpui_component::Icon;
use gpui_component::Sizable as _;
use gpui_component::avatar::Avatar;
use gpui_component::button::Button;
use gpui_component::input::{Input, InputState};
use gpui_component::skeleton::Skeleton;
use state::{Session, SessionState};

const HEIGHT: f32 = 52.;

pub struct TitleBar {
    session: Entity<Session>,
}

impl TitleBar {
    pub fn new(session: Entity<Session>, cx: &mut Context<Self>) -> Self {
        cx.observe(&session, |_, _, cx| cx.notify()).detach();
        Self { session }
    }
}

impl Render for TitleBar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();

        let session = self.session.clone();
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
            .child(match self.session.read(cx).state() {
                SessionState::SignedIn(profile) => Avatar::new()
                    .name(profile.display_name.clone())
                    .size_12()
                    .into_any_element(),
                _ => Skeleton::new().size_12().rounded_full().into_any_element(),
            })
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
            .child(
                div()
                    .occlude()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| {
                        cx.stop_propagation();
                    })
                    .child(
                        Button::new("sign-out")
                            .label("Sign out")
                            .small()
                            .icon(Icon::default().path("icons/log-out.svg"))
                            .on_click(move |_, _, cx| {
                                session.update(cx, |session, cx| session.sign_out(cx));
                            }),
                    ),
            )
    }
}
