use gpui::{
    Context, Entity, FontWeight, IntoElement, ParentElement as _, Render, Styled as _, Window, div,
    px,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::label::Label;
use gpui_component::{ActiveTheme as _, Disableable as _};
use state::{Session, SessionState};

pub struct LoginView {
    session: Entity<Session>,
}

impl LoginView {
    pub fn new(session: Entity<Session>, cx: &mut Context<Self>) -> Self {
        cx.observe(&session, |_, _, cx| cx.notify()).detach();
        Self { session }
    }
}

impl Render for LoginView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.session.read(cx).state().clone();
        let pending = self.session.read(cx).is_pending();

        let status = match &state {
            SessionState::SignedOut => "Sign in to load your Spotify library".to_owned(),
            SessionState::Restoring => "Checking your saved session...".to_owned(),
            SessionState::Authorizing => "Waiting for authorization in your browser...".to_owned(),
            SessionState::SignedIn(profile) => format!("Signed in as {}", profile.display_name),
            SessionState::Failed(error) => error.clone(),
        };

        let status_color = if matches!(state, SessionState::Failed(_)) {
            cx.theme().danger
        } else {
            cx.theme().muted_foreground
        };

        let session = self.session.clone();

        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_4()
            .size_full()
            .child(
                Label::new("spotty")
                    .text_size(px(28.))
                    .font_weight(FontWeight::BOLD),
            )
            .child(
                div()
                    .max_w(px(560.))
                    .text_center()
                    .text_size(px(13.))
                    .text_color(status_color)
                    .child(status),
            )
            .child(
                Button::new("sign-in")
                    .label("Sign in with Spotify")
                    .primary()
                    .disabled(pending)
                    .on_click(move |_, _, cx| {
                        session.update(cx, |session, cx| session.sign_in(cx));
                    }),
            )
    }
}
