use gpui::{Context, Entity, FontWeight, Render, uniform_list};
use state::{Library, LibraryState};
use ui::prelude::*;

const NAV: [&str; 3] = ["Home", "Search", "Your Library"];
const WIDTH: f32 = 220.;

pub struct Sidebar {
    library: Entity<Library>,
}

impl Sidebar {
    pub fn new(library: Entity<Library>, cx: &mut Context<Self>) -> Self {
        cx.observe(&library, |_, _, cx| cx.notify()).detach();
        Self { library }
    }

    fn playlists(&self, cx: &Context<Self>) -> Vec<SharedString> {
        match self.library.read(cx).state() {
            LibraryState::Ready { playlists, .. } => playlists
                .iter()
                .map(|playlist| SharedString::from(playlist.name.clone()))
                .collect(),
            _ => Vec::new(),
        }
    }
}

impl Render for Sidebar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::global(cx).clone();
        let playlists = self.playlists(cx);
        let loading = self.library.read(cx).is_loading();

        div()
            .flex()
            .flex_col()
            .w(px(WIDTH))
            .flex_none()
            .h_full()
            .bg(theme.surface)
            .border_r_1()
            .border_color(theme.border)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .p_3()
                    .children(NAV.into_iter().enumerate().map(|(index, label)| {
                        div()
                            .id(index)
                            .px_3()
                            .py_1p5()
                            .rounded_md()
                            .cursor_pointer()
                            .hover(move |style| style.bg(theme.elevated))
                            .child(Label::new(label).tone(Tone::Muted))
                    })),
            )
            .child(
                div().px_5().py_2().child(
                    Label::new("PLAYLISTS")
                        .tone(Tone::Muted)
                        .size(px(10.))
                        .weight(FontWeight::SEMIBOLD),
                ),
            )
            .child(if loading {
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .px_5()
                    .py_2()
                    .children((0..6).map(|index| {
                        Skeleton::new(index)
                            .w(px(120. - (index % 3) as f32 * 22.))
                            .h(px(10.))
                    }))
                    .into_any_element()
            } else {
                uniform_list(
                    "sidebar-playlists",
                    playlists.len(),
                    cx.processor(move |this, range: std::ops::Range<usize>, _window, cx| {
                        let theme = Theme::global(cx).clone();
                        let playlists = this.playlists(cx);

                        range
                            .filter_map(|index| {
                                let name = playlists.get(index)?.clone();
                                Some(
                                    div()
                                        .id(index)
                                        .px_5()
                                        .py_1p5()
                                        .cursor_pointer()
                                        .hover(move |style| style.bg(theme.elevated))
                                        .child(Label::new(name).tone(Tone::Muted).truncate()),
                                )
                            })
                            .collect()
                    }),
                )
                .flex_1()
                .into_any_element()
            })
    }
}
