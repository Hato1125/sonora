use gpui::{Context, Entity, FontWeight, Render, uniform_list};
use gpui_component::Icon;
use state::{Library, LibraryState};
use ui::prelude::*;

const NAV: [(&str, &str); 3] = [
    ("Home", "icons/house.svg"),
    ("Search", "icons/search.svg"),
    ("Your Library", "icons/library-big.svg"),
];
const WIDTH: f32 = 220.;

pub struct Sidebar {
    library: Entity<Library>,
}

impl Sidebar {
    pub fn new(library: Entity<Library>, cx: &mut Context<Self>) -> Self {
        cx.observe(&library, |_, _, cx| cx.notify()).detach();
        Self { library }
    }

    fn playlist_count(&self, cx: &Context<Self>) -> usize {
        match self.library.read(cx).state() {
            LibraryState::Ready { playlists, .. } => playlists.len(),
            _ => 0,
        }
    }
}

impl Render for Sidebar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let sidebar_accent = theme.sidebar_accent;
        let muted = theme.muted_foreground;
        let sidebar_bg = theme.sidebar;
        let sidebar_border = theme.sidebar_border;
        let count = self.playlist_count(cx);
        let loading = self.library.read(cx).is_loading();

        div()
            .flex()
            .flex_col()
            .w(px(WIDTH))
            .flex_none()
            .h_full()
            .bg(sidebar_bg)
            .border_r_1()
            .border_color(sidebar_border)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .p_3()
                    .children(NAV.into_iter().enumerate().map(|(index, (label, icon))| {
                        div()
                            .id(index)
                            .flex()
                            .items_center()
                            .gap_2p5()
                            .px_3()
                            .py_1p5()
                            .rounded_md()
                            .cursor_pointer()
                            .hover(move |style| style.bg(sidebar_accent))
                            .child(Icon::default().path(icon).size_4().text_color(muted))
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
                    count,
                    cx.processor(move |this, range: std::ops::Range<usize>, _window, cx| {
                        let elevated = cx.theme().sidebar_accent;
                        let LibraryState::Ready { playlists, .. } = this.library.read(cx).state()
                        else {
                            return Vec::new();
                        };

                        range
                            .filter_map(|index| {
                                let playlist = playlists.get(index)?;
                                Some(
                                    div()
                                        .id(index)
                                        .px_5()
                                        .py_1p5()
                                        .cursor_pointer()
                                        .hover(move |style| style.bg(elevated))
                                        .child(
                                            Label::new(playlist.name.clone())
                                                .tone(Tone::Muted)
                                                .truncate(),
                                        ),
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
