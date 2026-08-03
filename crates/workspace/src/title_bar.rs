use gpui::{Context, Entity, FontWeight, Render};
use state::{Session, SessionState};
use ui::prelude::*;

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
        let theme = Theme::global(cx).clone();
        let name = match self.session.read(cx).state() {
            SessionState::SignedIn(profile) => profile.display_name.clone(),
            _ => "spotty".to_owned(),
        };

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
            .bg(theme.surface)
            .border_b_1()
            .border_color(theme.border)
            .child(Label::new(name).size(px(14.)).weight(FontWeight::SEMIBOLD))
            .child(
                Button::new("sign-out", "Sign out")
                    .variant(ButtonVariant::Ghost)
                    .on_click(move |_, _, cx| {
                        session.update(cx, |session, cx| session.sign_out(cx));
                    }),
            )
    }
}
