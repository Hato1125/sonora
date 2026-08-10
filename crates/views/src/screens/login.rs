// SPDX-License-Identifier: GPL-3.0-or-later

use gpui::prelude::*;
use gpui::{Context, Entity, FontWeight, IntoElement, Render, SharedString, Window, div, px};
use i18n::t;
use state::{Session, SessionState};
use ui::ActiveTheme as _;
use ui::{Button, Text};

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
        let providers: Vec<(&'static str, &'static str)> =
            self.session.read(cx).providers().collect();

        let status = match &state {
            SessionState::SignedOut => t!("login-signed-out"),
            SessionState::Restoring => t!("login-restoring"),
            SessionState::Authorizing(_) => t!("login-authorizing"),
            SessionState::SignedIn(profile) => t!("login-signed-in", name = &profile.display_name),
            SessionState::Failed(error) => SharedString::from(error.clone()),
        };

        let prompt = match &state {
            SessionState::Authorizing(prompt) => prompt.clone(),
            _ => None,
        };

        let theme = *cx.theme();
        let status_color = match matches!(state, SessionState::Failed(_)) {
            true => theme.danger,
            false => theme.muted_foreground,
        };

        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_4()
            .size_full()
            .child(
                div()
                    .child("sonora")
                    .text_size(theme.text(Text::Display))
                    .font_weight(FontWeight::BOLD),
            )
            .child(
                div()
                    .max_w(px(560.))
                    .text_center()
                    .text_size(theme.text(Text::Body))
                    .text_color(status_color)
                    .child(status),
            )
            .when_some(prompt, |this, prompt| {
                this.child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .text_size(theme.text(Text::Small))
                                .text_color(theme.muted_foreground)
                                .child(t!("login-device-code", url = &prompt.url)),
                        )
                        .child(
                            div()
                                .text_size(theme.text(Text::Title))
                                .font_weight(FontWeight::BOLD)
                                .child(SharedString::from(prompt.code)),
                        ),
                )
            })
            .child(div().flex().flex_col().items_center().gap_2().children(
                providers.into_iter().map(|(slug, name)| {
                    let session = self.session.clone();
                    Button::new(SharedString::from(format!("sign-in-{slug}")))
                        .label(t!("login-sign-in", provider = name))
                        .primary()
                        .disabled(pending)
                        .on_click(move |_, _, cx| {
                            session.update(cx, |session, cx| session.sign_in(slug, cx));
                        })
                }),
            ))
    }
}
