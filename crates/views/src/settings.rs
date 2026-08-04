use gpui::prelude::*;
use gpui::{Context, Entity, FontWeight, Render, Window, div, px};
use gpui_component::avatar::Avatar;
use gpui_component::button::Button;
use gpui_component::label::Label;
use gpui_component::skeleton::Skeleton;
use gpui_component::{ActiveTheme as _, Icon, Sizable as _};
use state::{Playback, Session, SessionState};

pub struct SettingsView {
    session: Entity<Session>,
    playback: Entity<Playback>,
}

impl SettingsView {
    pub fn new(
        session: Entity<Session>,
        playback: Entity<Playback>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&session, |_, _, cx| cx.notify()).detach();
        cx.observe(&playback, |_, _, cx| cx.notify()).detach();
        Self { session, playback }
    }

    fn profile(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;

        div()
            .flex()
            .items_center()
            .gap_4()
            .child(match self.session.read(cx).state() {
                SessionState::SignedIn(profile) => Avatar::new()
                    .name(profile.display_name.clone())
                    .size_16()
                    .into_any_element(),
                _ => Skeleton::new().size_16().rounded_full().into_any_element(),
            })
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(match self.session.read(cx).state() {
                        SessionState::SignedIn(profile) => Label::new(profile.display_name.clone())
                            .text_size(px(18.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .into_any_element(),
                        _ => Skeleton::new().w(px(140.)).h(px(14.)).into_any_element(),
                    })
                    .child(match self.session.read(cx).state() {
                        SessionState::SignedIn(profile) => Label::new(profile.id.clone())
                            .text_color(muted)
                            .text_size(px(11.))
                            .into_any_element(),
                        _ => Skeleton::new().w(px(90.)).h(px(10.)).into_any_element(),
                    }),
            )
    }

    fn playback_settings(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        let on = self.playback.read(cx).normalisation();

        self.row(
            "Normalise loudness",
            "Keeps tracks at a consistent volume",
            muted,
            Button::new("normalisation")
                .label(if on { "On" } else { "Off" })
                .small()
                .outline()
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.playback
                        .update(cx, |playback, cx| playback.set_normalisation(!on, cx));
                }))
                .into_any_element(),
        )
    }

    fn account(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        let session = self.session.clone();

        self.row(
            "Account",
            "Sign out of Spotify on this device",
            muted,
            Button::new("sign-out")
                .label("Sign out")
                .small()
                .outline()
                .icon(Icon::default().path("icons/log-out.svg"))
                .on_click(move |_, _, cx| {
                    session.update(cx, |session, cx| session.sign_out(cx));
                })
                .into_any_element(),
        )
    }

    fn row(
        &self,
        title: &'static str,
        detail: &'static str,
        muted: gpui::Hsla,
        action: gpui::AnyElement,
    ) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_4()
            .py_3()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(Label::new(title))
                    .child(Label::new(detail).text_color(muted).text_size(px(11.))),
            )
            .child(action)
    }
}

impl Render for SettingsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let border = cx.theme().border;

        div().flex().flex_col().size_full().overflow_hidden().child(
            div()
                .flex()
                .flex_col()
                .gap_6()
                .w_full()
                .max_w(px(640.))
                .p_6()
                .child(self.profile(cx))
                .child(div().h(px(1.)).w_full().bg(border))
                .child(self.playback_settings(cx))
                .child(div().h(px(1.)).w_full().bg(border))
                .child(self.account(cx)),
        )
    }
}
