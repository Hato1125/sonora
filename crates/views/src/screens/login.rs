// SPDX-License-Identifier: GPL-3.0-or-later

use gpui::prelude::*;
use gpui::{
    ClipboardItem, Context, Entity, FontWeight, IntoElement, Render, SharedString, Window, div, px,
};
use i18n::t;
use music::SignInPrompt;
use state::{Session, SessionState};
use ui::ActiveTheme as _;
use ui::{Button, Input, Text};

pub struct LoginView {
    session: Entity<Session>,
    secret: Entity<Input>,
}

impl LoginView {
    pub fn new(session: Entity<Session>, cx: &mut Context<Self>) -> Self {
        cx.observe(&session, |_, _, cx| cx.notify()).detach();
        Self {
            session,
            secret: cx.new(|cx| Input::new("login-cookie-hint", cx)),
        }
    }

    fn submit(&mut self, cx: &mut Context<Self>) {
        let text = self.secret.read(cx).text().to_string();
        if text.trim().is_empty() {
            return;
        }
        self.secret.update(cx, |input, cx| input.set_text("", cx));
        self.session
            .update(cx, |session, cx| session.submit_input(text, cx));
    }

    fn code_prompt(&self, code: String, url: String, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.theme();
        div()
            .flex()
            .flex_col()
            .items_center()
            .gap_2()
            .child(
                div()
                    .text_size(theme.text(Text::Small))
                    .text_color(theme.muted_foreground)
                    .child(t!("login-device-code", url = &url)),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_size(theme.text(Text::Title))
                            .font_weight(FontWeight::BOLD)
                            .child(SharedString::from(code.clone())),
                    )
                    .child(
                        Button::new("copy-code")
                            .icon("icons/copy.svg")
                            .ghost()
                            .small()
                            .on_click(move |_, _, cx| {
                                cx.write_to_clipboard(ClipboardItem::new_string(code.clone()));
                            }),
                    ),
            )
    }

    fn secret_prompt(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.theme();
        div()
            .flex()
            .flex_col()
            .items_center()
            .gap_2()
            .w(px(560.))
            .child(
                div()
                    .text_size(theme.text(Text::Small))
                    .text_color(theme.muted_foreground)
                    .text_center()
                    .child(t!("login-cookie-detail")),
            )
            .child(div().w_full().child(self.secret.clone()))
            .child(
                Button::new("submit-cookies")
                    .label(t!("login-cookie-submit"))
                    .primary()
                    .on_click(cx.listener(|this, _, _, cx| this.submit(cx))),
            )
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
            SessionState::Authorizing(Some(SignInPrompt::Secret)) => t!("login-cookie-title"),
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
            .when_some(prompt, |this, prompt| match prompt {
                SignInPrompt::Code { code, url } => {
                    this.child(self.code_prompt(code, url, cx).into_any_element())
                }
                SignInPrompt::Secret => this.child(self.secret_prompt(cx).into_any_element()),
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
