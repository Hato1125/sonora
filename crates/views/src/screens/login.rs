use crate::shared::browsers::BrowserPicker;
use gpui::prelude::*;
use gpui::{
    AnyElement, ClipboardItem, Context, Entity, FontWeight, IntoElement, Pixels, Render,
    SharedString, Window, div, px, svg,
};
use i18n::t;
use music::{SignIn, SignInPrompt};
use state::{Session, SessionState};
use ui::ActiveTheme as _;
use ui::{Button, Input, Modal, Separator, Text};

const COLUMN: Pixels = px(280.);
const LOGO: Pixels = px(48.);
const RULE: Pixels = px(220.);

struct Column {
    slug: &'static str,
    name: &'static str,
    options: Vec<SignIn>,
    disabled: bool,
}

pub struct LoginView {
    session: Entity<Session>,
    secret: Entity<Input>,
    browsers: Option<(&'static str, Vec<SharedString>)>,
}

impl LoginView {
    pub fn new(session: Entity<Session>, cx: &mut Context<Self>) -> Self {
        cx.observe(&session, |_, _, cx| cx.notify()).detach();
        Self {
            session,
            secret: cx.new(|cx| Input::new("login-cookie-hint", cx)),
            browsers: None,
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

    fn abandon(&mut self, cx: &mut Context<Self>) {
        self.secret.update(cx, |input, cx| input.set_text("", cx));
        self.session
            .update(cx, |session, cx| session.cancel_sign_in(cx));
    }

    fn start(&self, slug: &'static str, method: SignIn, cx: &mut Context<Self>) {
        self.session
            .update(cx, |session, cx| session.sign_in(slug, method, cx));
    }

    fn option_button(
        &self,
        slug: &'static str,
        provider: &str,
        method: &SignIn,
        disabled: bool,
        cx: &mut Context<Self>,
    ) -> Button {
        let (id, label) = match method {
            SignIn::Default => (
                format!("sign-in-{slug}"),
                t!("login-sign-in", provider = provider),
            ),
            SignIn::Anonymous => (
                format!("sign-in-{slug}-guest"),
                t!("login-use", provider = provider),
            ),
            SignIn::Browser(_) => (
                format!("sign-in-{slug}-browser"),
                t!("login-import-browser"),
            ),
            SignIn::Secret => (
                format!("sign-in-{slug}-cookies"),
                t!("login-connect-cookies"),
            ),
            SignIn::Path(_) => (
                format!("sign-in-{slug}-path"),
                t!("login-sign-in", provider = provider),
            ),
        };
        let primary = matches!(method, SignIn::Default | SignIn::Anonymous);
        let method = method.clone();
        let button = Button::new(SharedString::from(id))
            .label(label)
            .w_full()
            .disabled(disabled)
            .on_click(cx.listener(move |this, _, _, cx| match &method {
                SignIn::Browser(_) => this.open_browsers(slug, cx),
                method => this.start(slug, method.clone(), cx),
            }));
        match primary {
            true => button.primary(),
            false => button.outline(),
        }
    }

    fn open_browsers(&mut self, slug: &'static str, cx: &mut Context<Self>) {
        let names = self
            .session
            .read(cx)
            .providers()
            .find(|info| info.slug == slug)
            .map(|info| {
                info.options
                    .iter()
                    .filter_map(|option| match option {
                        SignIn::Browser(name) => Some(SharedString::from(name.clone())),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if names.is_empty() {
            return;
        }
        self.browsers = Some((slug, names));
        cx.notify();
    }

    fn column(&self, column: Column, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.theme();
        let Column {
            slug,
            name,
            options,
            disabled,
        } = column;
        let mut seen_browser = false;
        let options: Vec<&SignIn> = options
            .iter()
            .filter(|option| match option {
                SignIn::Anonymous => false,
                SignIn::Browser(_) => !std::mem::replace(&mut seen_browser, true),
                _ => true,
            })
            .collect();

        div()
            .flex()
            .flex_col()
            .items_center()
            .gap_3()
            .w(COLUMN)
            .child(
                svg()
                    .path(crate::shared::provider_logo(slug))
                    .size(LOGO)
                    .flex_none()
                    .text_color(theme.foreground),
            )
            .child(
                div()
                    .text_size(theme.text(Text::Large))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(SharedString::from(name.to_string())),
            )
            .child(
                div().flex().flex_col().gap_2().w_full().children(
                    options
                        .into_iter()
                        .map(|method| self.option_button(slug, name, method, disabled, cx)),
                ),
            )
    }

    fn guest_mode(
        &self,
        guests: Vec<(&'static str, &'static str)>,
        pending: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = *cx.theme();
        let alone = guests.len() == 1;
        let mut buttons = Vec::new();
        for (slug, name) in guests {
            let label = match alone {
                true => t!("login-guest-use"),
                false => t!("login-use", provider = name),
            };
            buttons.push(
                Button::new(SharedString::from(format!("guest-{slug}")))
                    .label(label)
                    .outline()
                    .disabled(pending)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.start(slug, SignIn::Anonymous, cx);
                    })),
            );
        }

        div()
            .flex()
            .flex_col()
            .items_center()
            .gap_2()
            .pt_4()
            .border_t_1()
            .border_color(theme.border)
            .w(COLUMN * 2. + px(64.))
            .child(
                div()
                    .font_weight(FontWeight::MEDIUM)
                    .child(t!("login-guest-title")),
            )
            .child(
                div()
                    .max_w(px(420.))
                    .text_center()
                    .text_size(theme.text(Text::Small))
                    .text_color(theme.muted_foreground)
                    .child(t!("login-guest-detail")),
            )
            .child(div().flex().gap_2().children(buttons))
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
        Modal::new("cookie-prompt", t!("login-cookie-title"))
            .width(px(560.))
            .detail(t!("login-cookie-detail"))
            .child(self.secret.clone())
            .action(
                Button::new("cancel-cookies")
                    .ghost()
                    .label(t!("common-cancel"))
                    .on_click(cx.listener(|this, _, _, cx| this.abandon(cx))),
            )
            .action(
                Button::new("submit-cookies")
                    .label(t!("login-cookie-submit"))
                    .primary()
                    .on_click(cx.listener(|this, _, _, cx| this.submit(cx))),
            )
            .on_dismiss(cx.listener(|this, _, _, cx| this.abandon(cx)))
    }

    fn browser_modal(
        &self,
        slug: &'static str,
        names: Vec<SharedString>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        BrowserPicker::new(names)
            .on_pick(cx.listener(move |this, name: &SharedString, _, cx| {
                this.browsers = None;
                this.start(slug, SignIn::Browser(name.to_string()), cx);
            }))
            .on_cancel(cx.listener(|this, _, _, cx| {
                this.browsers = None;
                cx.notify();
            }))
    }
}

impl Render for LoginView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.session.read(cx).state().clone();
        let pending = self.session.read(cx).is_pending();
        let providers: Vec<state::ProviderInfo> = self.session.read(cx).providers().collect();
        let guests: Vec<(&'static str, &'static str)> = providers
            .iter()
            .filter(|info| {
                info.options
                    .iter()
                    .any(|option| matches!(option, SignIn::Anonymous))
            })
            .map(|info| (info.slug, info.name))
            .collect();
        let columns: Vec<Column> = providers
            .into_iter()
            .map(|info| Column {
                slug: info.slug,
                name: info.name,
                options: info.options,
                disabled: pending,
            })
            .collect();

        let status = match &state {
            SessionState::SignedOut => t!("login-signed-out"),
            SessionState::Restoring => t!("login-restoring"),
            SessionState::Authorizing(Some(SignInPrompt::Secret)) => t!("login-signed-out"),
            SessionState::Authorizing(_) => t!("login-authorizing"),
            SessionState::SignedIn(profile) => t!("login-signed-in", name = &profile.display_name),
            SessionState::Failed(error) => SharedString::from(error.clone()),
        };

        let prompt = match &state {
            SessionState::Authorizing(prompt) => prompt.clone(),
            _ => None,
        };
        let secret = matches!(prompt, Some(SignInPrompt::Secret));
        let code = match prompt {
            Some(SignInPrompt::Code { code, url }) => Some((code, url)),
            _ => None,
        };

        let theme = *cx.theme();
        let status_color = match matches!(state, SessionState::Failed(_)) {
            true => theme.danger,
            false => theme.muted_foreground,
        };
        let browsers = self.browsers.clone();

        div()
            .relative()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_6()
            .size_full()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .child("Sonora")
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
                    ),
            )
            .when_some(code, |this, (code, url)| {
                this.child(self.code_prompt(code, url, cx).into_any_element())
            })
            .child(
                div()
                    .flex()
                    .items_start()
                    .justify_center()
                    .gap_8()
                    .children(interleave(columns, cx, |column, cx| {
                        self.column(column, cx).into_any_element()
                    })),
            )
            .when(!guests.is_empty(), |this| {
                this.child(self.guest_mode(guests, pending, cx))
            })
            .when(secret, |this| {
                this.child(self.secret_prompt(cx).into_any_element())
            })
            .when_some(browsers, |this, (slug, names)| {
                this.child(self.browser_modal(slug, names, cx).into_any_element())
            })
    }
}

fn interleave<F>(
    columns: Vec<Column>,
    cx: &mut Context<LoginView>,
    mut render: F,
) -> Vec<AnyElement>
where
    F: FnMut(Column, &mut Context<LoginView>) -> AnyElement,
{
    let last = columns.len().saturating_sub(1);
    let mut children = Vec::new();
    for (index, column) in columns.into_iter().enumerate() {
        children.push(render(column, cx));
        if index < last {
            children.push(Separator::vertical().h(RULE).into_any_element());
        }
    }
    children
}
