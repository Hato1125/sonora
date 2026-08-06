use std::path::Path;
use std::process::Command;

use gpui::{
    AnyElement, Context, Entity, FontWeight, Pixels, Render, SharedString, Window, div, px,
};
use gpui::{ScrollHandle, prelude::*};
use state::{AppSettings, Playback, Session, SessionState, Spotty};
use ui::{ActiveTheme as _, Scrollbar, Scroller};
use ui::{
    Button, Initials, Look, MAX_FONT, MIN_FONT, Menu, MenuItem, Rounding, Skeleton, Text, Theme,
    ThemeKind,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Appearance,
    Playback,
    Account,
}

impl Tab {
    const ALL: [Self; 3] = [Self::Appearance, Self::Playback, Self::Account];

    fn label(self) -> &'static str {
        match self {
            Self::Appearance => "Appearance",
            Self::Playback => "Playback",
            Self::Account => "Account",
        }
    }
}

pub struct SettingsView {
    session: Entity<Session>,
    playback: Entity<Playback>,
    settings: Entity<AppSettings>,
    tab: Tab,
    scrollbar: Entity<Scrollbar>,
    themes_open: bool,
    corners_open: bool,
}

impl SettingsView {
    pub fn new(
        session: Entity<Session>,
        playback: Entity<Playback>,
        cx: &mut Context<Self>,
    ) -> Self {
        let settings = Spotty::global(cx).settings.clone();
        cx.observe(&session, |_, _, cx| cx.notify()).detach();
        cx.observe(&settings, |_, _, cx| cx.notify()).detach();
        Self {
            session,
            playback,
            settings,
            tab: Tab::Appearance,
            scrollbar: cx.new(|_| Scrollbar::new(ScrollHandle::new())),
            themes_open: false,
            corners_open: false,
        }
    }

    fn tabs(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div().flex().gap_1().children(Tab::ALL.map(|tab| {
            Button::new(SharedString::from(tab.label()))
                .label(tab.label())
                .small()
                .selected(self.tab == tab)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.tab = tab;
                    this.themes_open = false;
                    this.corners_open = false;
                    cx.notify();
                }))
        }))
    }

    fn panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let border = cx.theme().border;
        let rows: Vec<AnyElement> = match self.tab {
            Tab::Appearance => vec![
                self.theme_row(cx).into_any_element(),
                self.corners_row(cx).into_any_element(),
                self.font_row(cx).into_any_element(),
                self.auto_hide_row(cx).into_any_element(),
            ]
            .into_iter()
            .chain(decorated().then(|| self.decorations_row(cx).into_any_element()))
            .chain(decorated().then(|| self.side_row(cx).into_any_element()))
            .collect(),
            Tab::Playback => vec![self.playback_row(cx).into_any_element()],
            Tab::Account => vec![self.account_row(cx).into_any_element()],
        };

        let mut panel = div().flex().flex_col();
        for (index, row) in rows.into_iter().enumerate() {
            if index > 0 {
                panel = panel.child(div().h(px(1.)).w_full().bg(border));
            }
            panel = panel.child(row);
        }
        panel
    }

    fn look(&self, cx: &Context<Self>) -> Look {
        let settings = self.settings.read(cx);

        Look {
            kind: ThemeKind::from_id(settings.theme()),
            rounding: Rounding::from_id(settings.rounding()),
            font: settings.font_size(),
        }
    }

    fn corners_row(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.theme();
        let muted = theme.muted_foreground;
        let small = theme.text(Text::Small);
        let look = self.look(cx);
        let overrides = self.settings.read(cx).theme_overrides().clone();

        let picker = div()
            .relative()
            .child(
                Button::new("corners-picker")
                    .label(format!("{}  ▾", look.rounding.label()))
                    .small()
                    .outline()
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.corners_open = !this.corners_open;
                        cx.notify();
                    })),
            )
            .when(self.corners_open, |this| {
                this.child(
                    Menu::new("corners-dropdown")
                        .top(px(30.))
                        .right_0()
                        .w(px(170.))
                        .on_dismiss(cx.listener(|this, _, _, cx| {
                            this.corners_open = false;
                            cx.notify();
                        }))
                        .items(Rounding::ALL.into_iter().map(|rounding| {
                            let overrides = overrides.clone();
                            MenuItem::new(rounding.id(), rounding.label())
                                .selected(look.rounding == rounding)
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.settings.update(cx, |settings, cx| {
                                        settings.set_rounding(rounding.id(), cx);
                                    });
                                    this.corners_open = false;
                                    Theme::set(Look { rounding, ..look }, &overrides, cx);
                                    cx.notify();
                                }))
                        })),
                )
            });

        self.row(
            "Corners",
            "How rounded surfaces and controls are",
            muted,
            small,
            picker.into_any_element(),
        )
    }

    fn font_row(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.theme();
        let muted = theme.muted_foreground;
        let small = theme.text(Text::Small);
        let look = self.look(cx);
        let overrides = self.settings.read(cx).theme_overrides().clone();

        let step = move |id: &'static str, label: &'static str, delta: f32| {
            let overrides = overrides.clone();
            let wanted = (look.font + delta).clamp(MIN_FONT, MAX_FONT);

            Button::new(id)
                .label(label)
                .small()
                .outline()
                .disabled(wanted == look.font)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.settings
                        .update(cx, |settings, cx| settings.set_font_size(wanted, cx));
                    Theme::set(
                        Look {
                            font: wanted,
                            ..look
                        },
                        &overrides,
                        cx,
                    );
                    cx.notify();
                }))
        };

        let actions = div()
            .flex()
            .items_center()
            .gap_2()
            .child(step("font-smaller", "−", -1.))
            .child(div().child(format!("{:.0} px", look.font)))
            .child(step("font-larger", "+", 1.));

        self.row(
            "Font size",
            "Base text size, everything else scales with it",
            muted,
            small,
            actions.into_any_element(),
        )
    }

    fn decorations_row(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.theme();
        let muted = theme.muted_foreground;
        let small = theme.text(Text::Small);
        let on = self.settings.read(cx).window_controls();

        self.row(
            "Window controls",
            "Draw minimise, maximise and close in the title bar",
            muted,
            small,
            Button::new("window-controls")
                .label(if on { "On" } else { "Off" })
                .small()
                .outline()
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.settings
                        .update(cx, |settings, cx| settings.set_window_controls(!on, cx));
                }))
                .into_any_element(),
        )
    }

    fn side_row(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.theme();
        let muted = theme.muted_foreground;
        let small = theme.text(Text::Small);
        let settings = self.settings.read(cx);
        let left = settings.controls_on_left();
        let shown = settings.window_controls();

        self.row(
            "Controls side",
            "Which end of the title bar the controls sit on",
            muted,
            small,
            Button::new("controls-side")
                .label(if left { "Left" } else { "Right" })
                .small()
                .outline()
                .disabled(!shown)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.settings
                        .update(cx, |settings, cx| settings.set_controls_on_left(!left, cx));
                }))
                .into_any_element(),
        )
    }

    fn auto_hide_row(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.theme();
        let muted = theme.muted_foreground;
        let small = theme.text(Text::Small);
        let on = self.settings.read(cx).auto_hide_sidebar();

        self.row(
            "Auto-hide sidebar",
            "Collapse the sidebar when the window gets narrow",
            muted,
            small,
            Button::new("auto-hide-sidebar")
                .label(if on { "On" } else { "Off" })
                .small()
                .outline()
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.settings
                        .update(cx, |settings, cx| settings.set_auto_hide_sidebar(!on, cx));
                }))
                .into_any_element(),
        )
    }

    fn profile(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.theme();
        let muted = theme.muted_foreground;

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
                            .text_size(theme.text(Text::Large))
                            .font_weight(FontWeight::SEMIBOLD)
                            .into_any_element(),
                        _ => Skeleton::new().w(px(140.)).h(px(14.)).into_any_element(),
                    })
                    .child(match self.session.read(cx).state() {
                        SessionState::SignedIn(profile) => div()
                            .child(profile.id.clone())
                            .text_color(muted)
                            .text_size(theme.text(Text::Small))
                            .into_any_element(),
                        _ => Skeleton::new().w(px(90.)).h(px(10.)).into_any_element(),
                    }),
            )
    }

    fn theme_row(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.theme();
        let muted = theme.muted_foreground;
        let small = theme.text(Text::Small);
        let look = self.look(cx);
        let current = look.kind;
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
                                    Theme::set(Look { kind, ..look }, &overrides, cx);
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
            small,
            actions.into_any_element(),
        )
    }

    fn playback_row(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.theme();
        let muted = theme.muted_foreground;
        let small = theme.text(Text::Small);
        let on = self.playback.read(cx).normalisation();

        self.row(
            "Normalise loudness",
            "Keeps tracks at a consistent volume",
            muted,
            small,
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

    fn account_row(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.theme();
        let muted = theme.muted_foreground;
        let small = theme.text(Text::Small);
        let session = self.session.clone();

        self.row(
            "Account",
            "Sign out of Spotify on this device",
            muted,
            small,
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
        small: Pixels,
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
                    .child(div().text_color(muted).text_size(small).child(detail)),
            )
            .child(action)
    }
}

fn decorated() -> bool {
    cfg!(not(target_os = "macos"))
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

        Scroller::new("settings", &self.scrollbar)
            .flex()
            .flex_col()
            .items_center()
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
                    .child(self.tabs(cx))
                    .child(self.panel(cx)),
            )
    }
}
