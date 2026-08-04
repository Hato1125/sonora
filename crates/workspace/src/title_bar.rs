use gpui::prelude::*;
use gpui::{Context, Entity, FontWeight, Render};
use gpui::{Window, div, px};
use gpui_component::ActiveTheme as _;
use gpui_component::Icon;
use gpui_component::Sizable as _;
use gpui_component::button::Button;
use gpui_component::label::Label;
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
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();

        let session = self.session.clone();

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
            .child(match self.session.read(cx).state() {
                SessionState::SignedIn(profile) => Label::new(profile.display_name.clone())
                    .text_size(px(14.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .into_any_element(),
                _ => Skeleton::new().w(px(80.)).h(px(11.)).into_any_element(),
            })
            .child(
                Button::new("sign-out")
                    .label("Sign out")
                    .small()
                    .icon(Icon::default().path("icons/log-out.svg"))
                    .on_click(move |_, _, cx| {
                        session.update(cx, |session, cx| session.sign_out(cx));
                    }),
            )
    }
}
