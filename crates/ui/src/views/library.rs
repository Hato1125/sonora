use gpui::{
    AnyElement, Context, Entity, IntoElement, ParentElement, Render, SharedString, Styled, Window,
    div, prelude::*, px, uniform_list,
};
use state::{Library, LibraryState, Session, SessionState};

use crate::components::{Button, ButtonVariant, format_duration};
use crate::theme::Theme;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Tracks,
    Playlists,
}

pub struct LibraryView {
    session: Entity<Session>,
    library: Entity<Library>,
    tab: Tab,
}

impl LibraryView {
    pub fn new(session: Entity<Session>, library: Entity<Library>, cx: &mut Context<Self>) -> Self {
        cx.observe(&session, |_, _, cx| cx.notify()).detach();
        cx.observe(&library, |_, _, cx| cx.notify()).detach();
        Self {
            session,
            library,
            tab: Tab::Tracks,
        }
    }

    fn header(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::global(cx).clone();
        let name = match self.session.read(cx).state() {
            SessionState::SignedIn(profile) => profile.display_name.clone(),
            _ => "Library".to_owned(),
        };

        let session = self.session.clone();
        let library = self.library.clone();
        let loading = self.library.read(cx).is_loading();

        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .px_5()
            .py_4()
            .border_b_1()
            .border_color(theme.border)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_size(px(15.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child(name),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme.text_muted)
                            .child("Your saved tracks and playlists"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        Button::new("refresh", if loading { "Loading..." } else { "Refresh" })
                            .variant(ButtonVariant::Ghost)
                            .disabled(loading)
                            .on_click(move |_, _, cx| {
                                library.update(cx, |library, cx| library.refresh(cx));
                            }),
                    )
                    .child(
                        Button::new("sign-out", "Sign out")
                            .variant(ButtonVariant::Ghost)
                            .on_click(move |_, _, cx| {
                                session.update(cx, |session, cx| session.sign_out(cx));
                            }),
                    ),
            )
            .into_any_element()
    }

    fn tabs(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::global(cx).clone();
        let (tracks, playlists) = match self.library.read(cx).state() {
            LibraryState::Ready {
                tracks, playlists, ..
            } => (tracks.len(), playlists.len()),
            _ => (0, 0),
        };

        div()
            .flex()
            .gap_1()
            .px_5()
            .py_3()
            .child(self.tab_button(Tab::Tracks, format!("Songs ({tracks})"), &theme, cx))
            .child(self.tab_button(
                Tab::Playlists,
                format!("Playlists ({playlists})"),
                &theme,
                cx,
            ))
            .into_any_element()
    }

    fn tab_button(
        &self,
        tab: Tab,
        label: String,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let selected = self.tab == tab;
        let id: SharedString = match tab {
            Tab::Tracks => "tab-tracks".into(),
            Tab::Playlists => "tab-playlists".into(),
        };

        div()
            .id(id)
            .px_3()
            .py_1p5()
            .rounded_full()
            .cursor_pointer()
            .text_size(px(12.))
            .bg(if selected {
                theme.elevated
            } else {
                theme.background
            })
            .text_color(if selected {
                theme.text
            } else {
                theme.text_muted
            })
            .hover(|style| style.text_color(theme.text))
            .child(label)
            .on_click(cx.listener(move |this, _, _, cx| {
                this.tab = tab;
                cx.notify();
            }))
            .into_any_element()
    }

    fn body(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::global(cx).clone();

        match self.library.read(cx).state() {
            LibraryState::Empty => message("Nothing loaded yet", theme.text_muted),
            LibraryState::Loading => message("Loading your library...", theme.text_muted),
            LibraryState::Failed(error) => message(error.clone(), theme.danger),
            LibraryState::Ready {
                tracks,
                playlists,
                problems,
            } => {
                let (count, label) = match self.tab {
                    Tab::Tracks => (tracks.len(), "Songs"),
                    Tab::Playlists => (playlists.len(), "Playlists"),
                };

                if count == 0 {
                    let problem = problems
                        .iter()
                        .find(|problem| problem.starts_with(label))
                        .cloned();

                    return match problem {
                        Some(problem) => message(problem, theme.danger),
                        None => message("Nothing here", theme.text_muted),
                    };
                }

                let list_id = match self.tab {
                    Tab::Tracks => "library-tracks",
                    Tab::Playlists => "library-playlists",
                };

                uniform_list(
                    list_id,
                    count,
                    cx.processor(move |this, range: std::ops::Range<usize>, _window, cx| {
                        let theme = Theme::global(cx).clone();
                        let library = this.library.read(cx);
                        let tab = this.tab;

                        let LibraryState::Ready {
                            tracks, playlists, ..
                        } = library.state()
                        else {
                            return Vec::new();
                        };

                        range
                            .map(|index| match tab {
                                Tab::Tracks => {
                                    let track = &tracks[index];
                                    row(
                                        index,
                                        track.name.clone(),
                                        track.artists.clone(),
                                        track.album.clone(),
                                        format_duration(track.duration),
                                        &theme,
                                    )
                                }
                                Tab::Playlists => {
                                    let playlist = &playlists[index];
                                    row(
                                        index,
                                        playlist.name.clone(),
                                        playlist.owner.clone(),
                                        String::new(),
                                        format!("{} tracks", playlist.track_count),
                                        &theme,
                                    )
                                }
                            })
                            .collect()
                    }),
                )
                .flex_1()
                .into_any_element()
            }
        }
    }
}

impl Render for LibraryView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .child(self.header(cx))
            .child(self.tabs(cx))
            .child(self.body(cx))
    }
}

fn message(text: impl Into<SharedString>, color: gpui::Hsla) -> AnyElement {
    div()
        .flex()
        .flex_1()
        .items_center()
        .justify_center()
        .px_6()
        .child(
            div()
                .max_w(px(560.))
                .text_center()
                .text_size(px(13.))
                .text_color(color)
                .child(text.into()),
        )
        .into_any_element()
}

fn row(
    index: usize,
    primary: String,
    secondary: String,
    tertiary: String,
    trailing: String,
    theme: &Theme,
) -> AnyElement {
    div()
        .id(index)
        .flex()
        .w_full()
        .items_center()
        .gap_3()
        .h(px(44.))
        .px_5()
        .text_size(px(13.))
        .hover(|style| style.bg(theme.surface))
        .child(
            div()
                .w(px(28.))
                .flex_none()
                .text_color(theme.text_muted)
                .child(format!("{}", index + 1)),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_w_0()
                .child(div().truncate().child(primary))
                .child(
                    div()
                        .truncate()
                        .text_size(px(11.))
                        .text_color(theme.text_muted)
                        .child(secondary),
                ),
        )
        .child(
            div()
                .w(px(220.))
                .flex_none()
                .truncate()
                .text_color(theme.text_muted)
                .child(tertiary),
        )
        .child(
            div()
                .w(px(72.))
                .flex_none()
                .text_right()
                .text_color(theme.text_muted)
                .child(trailing),
        )
        .into_any_element()
}
