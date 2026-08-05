use std::path::Path;
use std::process::Command;

use gpui::prelude::*;
use gpui::{Context, Entity, FontWeight, Render, Window, div, px};
use state::{AppSettings, Playback, Session, SessionState, Spotty};
use ui::ActiveTheme as _;
use ui::{Button, Initials, Menu, MenuItem, Skeleton, Theme, ThemeKind};

pub struct SettingsView {
    session: Entity<Session>,
    playback: Entity<Playback>,
    settings: Entity<AppSettings>,
    themes_open: bool,
}

impl SettingsView {
    pub fn new(
        session: Entity<Session>,
        playback: Entity<Playback>,
        cx: &mut Context<Self>,
    ) -> Self {
        let settings = Spotty::global(cx).settings.clone();
        cx.observe(&session, |_, _, cx| cx.notify()).detach();
        cx.observe(&playback, |_, _, cx| cx.notify()).detach();
        cx.observe(&settings, |_, _, cx| cx.notify()).detach();
        Self {
            session,
            playback,
            settings,
            themes_open: false,
        }
    }

    fn profile(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;

        div()
            .flex()
            .items_center()
            .gap_4()
            .child(match self.session.read(cx).state() {
                SessionState::SignedIn(profile) => {
                    Initials::new(profile.display_name.clone(), px(64.)).into_any_element()
                }
                _ => Skeleton::new().size(px(64.)).circle().into_any_element(),
            })
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(match self.session.read(cx).state() {
                        SessionState::SignedIn(profile) => div()
                            .child(profile.display_name.clone())
                            .text_size(px(18.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .into_any_element(),
                        _ => Skeleton::new().w(px(140.)).h(px(14.)).into_any_element(),
                    })
                    .child(match self.session.read(cx).state() {
                        SessionState::SignedIn(profile) => div()
                            .child(profile.id.clone())
                            .text_color(muted)
                            .text_size(px(11.))
                            .into_any_element(),
                        _ => Skeleton::new().w(px(90.)).h(px(10.)).into_any_element(),
                    }),
            )
    }

    fn appearance_settings(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;
        let current = ThemeKind::from_id(self.settings.read(cx).theme());
        let overrides = self.settings.read(cx).theme_overrides().clone();

        let picker = div()
            .relative()
            .child(
                Button::new("theme-picker")
                    .label(format!("{}  ▾", current.label()))
                    .small()
                    .outline()
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.themes_open = !this.themes_open;
                        cx.notify();
                    })),
            )
            .when(self.themes_open, |this| {
                this.child(
                    Menu::new("theme-dropdown")
                        .top(px(30.))
                        .right_0()
                        .w(px(170.))
                        .on_dismiss(cx.listener(|this, _, _, cx| {
                            this.themes_open = false;
                            cx.notify();
                        }))
                        .items(ThemeKind::ALL.into_iter().map(|kind| {
                            let overrides = overrides.clone();
                            MenuItem::new(kind.id(), kind.label())
                                .selected(current == kind)
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.settings.update(cx, |settings, cx| {
                                        settings.set_theme(kind.id(), cx);
                                    });
                                    this.themes_open = false;
                                    Theme::set(kind, &overrides, cx);
                                    cx.notify();
                                }))
                        })),
                )
            });

        let settings = self.settings.clone();
        let actions = div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                Button::new("open-theme-config")
                    .label("Open config")
                    .small()
                    .outline()
                    .on_click(move |_, _, cx| {
                        let path = settings.update(cx, |settings, _| settings.ensure_file());
                        if let Err(error) = open_settings_file(&path) {
                            eprintln!("spotty: cannot open {}: {error}", path.display());
                        }
                    }),
            )
            .child(picker);

        self.row(
            "Theme",
            "Choose the application colour palette",
            muted,
            actions.into_any_element(),
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
                .icon("icons/log-out.svg")
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
                    .child(div().child(title))
                    .child(div().text_color(muted).text_size(px(11.)).child(detail)),
            )
            .child(action)
    }
}

fn open_settings_file(path: &Path) -> std::io::Result<()> {
    #[cfg(target_os = "windows")]
    Command::new("cmd")
        .args(["/C", "start", ""])
        .arg(path)
        .spawn()?;

    #[cfg(target_os = "macos")]
    Command::new("open").arg(path).spawn()?;

    #[cfg(target_os = "linux")]
    Command::new("xdg-open").arg(path).spawn()?;

    Ok(())
}

impl Render for SettingsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let border = cx.theme().border;

        div()
            .flex()
            .flex_col()
            .items_center()
            .size_full()
            .overflow_hidden()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_6()
                    .w_full()
                    .max_w(px(640.))
                    .p_6()
                    .child(self.profile(cx))
                    .child(div().h(px(1.)).w_full().bg(border))
                    .child(self.appearance_settings(cx))
                    .child(div().h(px(1.)).w_full().bg(border))
                    .child(self.playback_settings(cx))
                    .child(div().h(px(1.)).w_full().bg(border))
                    .child(self.account(cx)),
            )
    }
}
