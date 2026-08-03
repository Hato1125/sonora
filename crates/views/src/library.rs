use std::time::Duration;

use gpui::{AnyElement, Context, Entity, Render, uniform_list};
use state::{Library, LibraryState};
use ui::prelude::*;

const SKELETON_ROWS: usize = 12;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    Tracks,
    Playlists,
}

impl Section {
    fn label(self) -> &'static str {
        match self {
            Section::Tracks => "Songs",
            Section::Playlists => "Playlists",
        }
    }

    fn id(self) -> &'static str {
        match self {
            Section::Tracks => "tab-tracks",
            Section::Playlists => "tab-playlists",
        }
    }

    fn list_id(self) -> &'static str {
        match self {
            Section::Tracks => "library-tracks",
            Section::Playlists => "library-playlists",
        }
    }
}

pub struct LibraryView {
    library: Entity<Library>,
    section: Section,
}

impl LibraryView {
    pub fn new(library: Entity<Library>, cx: &mut Context<Self>) -> Self {
        cx.observe(&library, |_, _, cx| cx.notify()).detach();
        Self {
            library,
            section: Section::Tracks,
        }
    }

    fn tabs(&self, cx: &mut Context<Self>) -> AnyElement {
        let (tracks, playlists) = match self.library.read(cx).state() {
            LibraryState::Ready {
                tracks, playlists, ..
            } => (tracks.len(), playlists.len()),
            _ => (0, 0),
        };

        let library = self.library.clone();
        let loading = self.library.read(cx).is_loading();

        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .px_5()
            .py_3()
            .child(
                div()
                    .flex()
                    .gap_1()
                    .child(self.tab(Section::Tracks, tracks, cx))
                    .child(self.tab(Section::Playlists, playlists, cx)),
            )
            .child(
                Button::new("refresh", "Refresh")
                    .variant(ButtonVariant::Ghost)
                    .disabled(loading)
                    .on_click(move |_, _, cx| {
                        library.update(cx, |library, cx| library.refresh(cx));
                    }),
            )
            .into_any_element()
    }

    fn tab(&self, section: Section, count: usize, cx: &mut Context<Self>) -> Tab {
        Tab::new(section.id(), format!("{} ({count})", section.label()))
            .selected(self.section == section)
            .on_click(cx.listener(move |this, _, _, cx| {
                this.section = section;
                cx.notify();
            }))
    }

    fn body(&self, cx: &mut Context<Self>) -> AnyElement {
        match self.library.read(cx).state() {
            LibraryState::Empty => Message::new("Nothing loaded yet").into_any_element(),
            LibraryState::Loading => div()
                .flex()
                .flex_col()
                .flex_1()
                .children((0..SKELETON_ROWS).map(ListRow::loading))
                .into_any_element(),
            LibraryState::Failed(error) => Message::new(error.clone())
                .tone(Tone::Danger)
                .into_any_element(),
            LibraryState::Ready {
                tracks,
                playlists,
                problems,
            } => {
                let count = match self.section {
                    Section::Tracks => tracks.len(),
                    Section::Playlists => playlists.len(),
                };

                if count == 0 {
                    let label = self.section.label();
                    let problem = problems
                        .iter()
                        .find(|problem| problem.starts_with(label))
                        .cloned();

                    return match problem {
                        Some(problem) => {
                            Message::new(problem).tone(Tone::Danger).into_any_element()
                        }
                        None => Message::new("Nothing here").into_any_element(),
                    };
                }

                uniform_list(
                    self.section.list_id(),
                    count,
                    cx.processor(move |this, range: std::ops::Range<usize>, _window, cx| {
                        let section = this.section;
                        let LibraryState::Ready {
                            tracks, playlists, ..
                        } = this.library.read(cx).state()
                        else {
                            return Vec::new();
                        };

                        range
                            .map(|index| match section {
                                Section::Tracks => {
                                    let track = &tracks[index];
                                    ListRow::new(index, track.name.clone())
                                        .secondary(track.artists.clone())
                                        .meta(track.album.clone())
                                        .trailing(format_duration(track.duration))
                                }
                                Section::Playlists => {
                                    let playlist = &playlists[index];
                                    ListRow::new(index, playlist.name.clone())
                                        .secondary(playlist.owner.clone())
                                        .meta("")
                                        .trailing(format!("{} tracks", playlist.track_count))
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
            .child(self.tabs(cx))
            .child(self.body(cx))
    }
}

fn format_duration(duration: Duration) -> String {
    let total = duration.as_secs();
    format!("{}:{:02}", total / 60, total % 60)
}
