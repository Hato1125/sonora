use std::cell::Cell;

use gpui::prelude::*;
use gpui::{
    Context, DragMoveEvent, Empty, Entity, EventEmitter, FontWeight, Pixels, Render, uniform_list,
};
use gpui::{Window, div, px};
use gpui_component::ActiveTheme as _;
use gpui_component::Icon;
use gpui_component::label::Label;
use gpui_component::skeleton::Skeleton;
use state::{Library, LibraryState};

const NAV: [(&str, &str, Option<Destination>); 4] = [
    ("Home", "icons/house.svg", None),
    ("Search", "icons/search.svg", None),
    (
        "Your Library",
        "icons/library-big.svg",
        Some(Destination::Library),
    ),
    (
        "Settings",
        "icons/settings.svg",
        Some(Destination::Settings),
    ),
];
const DEFAULT_WIDTH: Pixels = px(220.);
const MIN_WIDTH: Pixels = px(130.);
const MAX_WIDTH: Pixels = px(400.);

struct SidebarResize {
    start_width: Pixels,
    start_x: Cell<Pixels>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Destination {
    Library,
    Settings,
}

pub enum SidebarEvent {
    Navigate(Destination),
}

pub struct Sidebar {
    library: Entity<Library>,
    width: Pixels,
    open: bool,
}

impl EventEmitter<SidebarEvent> for Sidebar {}

impl Sidebar {
    pub fn new(library: Entity<Library>, cx: &mut Context<Self>) -> Self {
        cx.observe(&library, |_, _, cx| cx.notify()).detach();
        Self {
            library,
            width: DEFAULT_WIDTH,
            open: true,
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn occupied_width(&self) -> Pixels {
        if self.open { self.width } else { Pixels::ZERO }
    }

    pub fn toggle(&mut self, cx: &mut Context<Self>) {
        self.open = !self.open;
        cx.notify();
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
            .when(!self.open, |this| this.hidden())
            .relative()
            .w(self.width)
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
                    .children(NAV.into_iter().enumerate().map(
                        |(index, (label, icon, destination))| {
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
                                .child(Label::new(label).text_color(muted))
                                .when_some(destination, |this, destination| {
                                    this.on_click(cx.listener(move |_, _, _, cx| {
                                        cx.emit(SidebarEvent::Navigate(destination));
                                    }))
                                })
                        },
                    )),
            )
            .child(
                div().px_5().py_2().child(
                    Label::new("PLAYLISTS")
                        .text_color(muted)
                        .text_size(px(10.))
                        .font_weight(FontWeight::SEMIBOLD),
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
                        Skeleton::new()
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
                        let muted = cx.theme().muted_foreground;
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
                                                .text_color(muted)
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
            .child(
                div()
                    .id("sidebar-resize-handle")
                    .absolute()
                    .top_0()
                    .right(px(-4.))
                    .w(px(8.))
                    .h_full()
                    .cursor_col_resize()
                    .on_drag_move(cx.listener(
                        |this, event: &DragMoveEvent<SidebarResize>, _, cx| {
                            let resize = event.drag(cx);
                            this.width = (resize.start_width + event.event.position.x
                                - resize.start_x.get())
                            .clamp(MIN_WIDTH, MAX_WIDTH);
                            cx.notify();
                        },
                    ))
                    .on_drag(
                        SidebarResize {
                            start_width: self.width,
                            start_x: Cell::new(Pixels::ZERO),
                        },
                        |resize, _, window, cx| {
                            resize.start_x.set(window.mouse_position().x);
                            cx.new(|_| Empty)
                        },
                    ),
            )
    }
}
