use std::cell::Cell;
use ui::ActiveTheme as _;

use gpui::prelude::*;
use gpui::{Context, DragMoveEvent, Empty, Entity, EventEmitter, Pixels, Render, SharedString};
use gpui::{Window, div, px, svg};
use state::{AppSettings, Spotty};

const NAV: [(&str, &str, Option<Destination>); 4] = [
    ("Home", "icons/house.svg", None),
    ("Search", "icons/search.svg", Some(Destination::Search)),
    (
        "Your Library",
        "icons/library-big.svg",
        Some(Destination::Library(LibraryTab::Songs)),
    ),
    (
        "Settings",
        "icons/settings.svg",
        Some(Destination::Settings),
    ),
];

const MIN_WIDTH: Pixels = px(130.);
const MAX_WIDTH: Pixels = px(400.);

struct SidebarResize {
    start_width: Pixels,
    start_x: Cell<Pixels>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LibraryTab {
    Songs,
    Albums,
    Playlists,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Destination {
    Library(LibraryTab),
    Album(SharedString),
    Playlist(SharedString),
    Search,
    Settings,
}

pub enum SidebarEvent {
    Navigate(Destination),
}

pub struct Sidebar {
    settings: Entity<AppSettings>,
    width: Pixels,
    open: bool,
}

impl EventEmitter<SidebarEvent> for Sidebar {}

impl Sidebar {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let settings = Spotty::global(cx).settings.clone();
        let width = px(settings.read(cx).sidebar_width()).clamp(MIN_WIDTH, MAX_WIDTH);
        let open = settings.read(cx).sidebar_open();
        Self {
            settings,
            width,
            open,
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
        self.persist(cx);
        cx.notify();
    }

    fn persist(&self, cx: &mut Context<Self>) {
        let width = self.width / px(1.);
        let open = self.open;
        self.settings
            .update(cx, |settings, cx| settings.set_sidebar(width, open, cx));
    }
}

impl Render for Sidebar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let sidebar_accent = theme.sidebar_accent;
        let muted = theme.muted_foreground;
        let sidebar_bg = theme.sidebar;
        let sidebar_border = theme.sidebar_border;

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
                                .child(svg().path(icon).size_4().flex_none().text_color(muted))
                                .child(div().text_color(muted).child(label))
                                .when_some(destination, |this, destination| {
                                    this.on_click(cx.listener(move |_, _, _, cx| {
                                        cx.emit(SidebarEvent::Navigate(destination.clone()));
                                    }))
                                })
                        },
                    )),
            )
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
                            this.persist(cx);
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
