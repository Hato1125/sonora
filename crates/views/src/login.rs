use gpui::{Context, Entity, FontWeight, Render};
use state::{Session, SessionState};
use ui::prelude::*;

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

        let tone = if matches!(state, SessionState::Failed(_)) {
            Tone::Danger
        } else {
            Tone::Muted
        };

        let session = self.session.clone();

        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_4()
            .size_full()
            .child(Label::new("spotty").size(px(28.)).weight(FontWeight::BOLD))
            .child(Message::new(status).tone(tone))
            .child(
                Button::new("sign-in", "Sign in with Spotify")
                    .disabled(pending)
                    .on_click(move |_, _, cx| {
                        session.update(cx, |session, cx| session.sign_in(cx));
                    }),
            )
    }
}
