// SPDX-License-Identifier: GPL-3.0-or-later

use ui::{
    ActiveTheme as _, Button, Card, DraggedPin, Edge, Panel, Pin, PinKind, Pinnable as _, Popup,
    SNUG, Scrollbar, Scroller, Shield, Side, Tabs, Text, drop_gap, drop_marker,
};

use gpui::prelude::*;
use gpui::{
    AnyElement, App, Context, DragMoveEvent, ElementId, Entity, Hsla, MouseButton, MouseDownEvent,
    Pixels, Point, Render, ScrollHandle,
};
use gpui::{Window, div, px};
use router::{Destination, LibraryTab, Navigation, NavigationEvent, SettingsTab, navigate};
use state::{AppSettings, Origin, Playback, PlaybackState, Session, Sonora};

use crate::shared::menu::{ItemMenu, pin_menu};

const NAV: [(&str, &str, Option<Destination>); 4] = [
    ("nav-home", "icons/house.svg", Some(Destination::Home)),
    ("nav-search", "icons/search.svg", Some(Destination::Search)),
    (
        "nav-library",
        "icons/library-big.svg",
        Some(Destination::Library(LibraryTab::Songs)),
    ),
    (
        "nav-settings",
        "icons/settings.svg",
        Some(Destination::Settings(SettingsTab::General)),
    ),
];

const LIBRARY_TABS: [(&str, LibraryTab); 5] = [
    ("nav-songs", LibraryTab::Songs),
    ("nav-albums", LibraryTab::Albums),
    ("nav-playlists", LibraryTab::Playlists),
    ("nav-artists", LibraryTab::Artists),
    ("nav-local", LibraryTab::Local),
];

const SETTINGS_TABS: [(&str, SettingsTab); 4] = [
    ("settings-tab-general", SettingsTab::General),
    ("settings-tab-appearance", SettingsTab::Appearance),
    ("settings-tab-playback", SettingsTab::Playback),
    ("settings-tab-about", SettingsTab::About),
];

const MIN_WIDTH: Pixels = px(160.);
const MAX_WIDTH: Pixels = px(400.);
const HINT_HEIGHT: Pixels = px(42.);

pub(crate) struct SidebarLeft {
    settings: Entity<AppSettings>,
    session: Entity<Session>,
    trail: Entity<Navigation>,
    at: Destination,
    width: Pixels,
    open: bool,
    cramped: bool,
    forced: Option<bool>,
    library_open: bool,
    settings_open: bool,
    dropping: bool,
    drop_gap: Option<usize>,
    playback: Entity<Playback>,
    track_menu: ItemMenu,
    context_menu: Option<(Pin, Point<Pixels>)>,
    scrollbar: Entity<Scrollbar>,
}

impl SidebarLeft {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let settings = Sonora::global(cx).settings.clone();
        let session = Sonora::global(cx).session.clone();
        let playback = Sonora::global(cx).playback.clone();
        let playlist_scrollbar = cx.new(|_| {
            Scrollbar::new(ScrollHandle::new())
                .always_visible()
                .track_inset(px(4.))
        });
        let scrollbar = cx.new(|_| Scrollbar::new(ScrollHandle::new()));
        let width = px(settings.read(cx).sidebar_width()).clamp(MIN_WIDTH, MAX_WIDTH);
        let open = settings.read(cx).sidebar_open();
        let trail = router::trail(cx);

        cx.observe(&session, |_, _, cx| cx.notify()).detach();
        cx.observe(&settings, |_, _, cx| cx.notify()).detach();
        cx.observe(&trail, |_, _, cx| cx.notify()).detach();
        cx.subscribe(&trail, |this, _, _: &NavigationEvent, cx| {
            this.dismiss(cx);
            cx.notify();
        })
        .detach();

        let at = trail.read(cx).current();
        let library_open = matches!(at, Destination::Library(_));
        let settings_open = matches!(at, Destination::Settings(_));

        Self {
            settings,
            session,
            trail,
            at,
            width,
            open,
            forced: None,
            cramped: false,
            library_open,
            settings_open,
            dropping: false,
            drop_gap: None,
            playback,
            track_menu: ItemMenu::new(playlist_scrollbar),
            context_menu: None,
            scrollbar,
        }
    }

    fn follow(&mut self, current: &Destination) {
        if self.at == *current {
            return;
        }
        self.at = current.clone();
        (self.library_open, self.settings_open) = expanded(current);
    }

    fn dismiss_menu(&mut self, cx: &mut Context<Self>) {
        self.track_menu.reset(cx);
        self.context_menu = None;
        cx.notify();
    }

    fn menu(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        let (pin, position) = self.context_menu.clone()?;
        let menu = pin_menu(&pin, &self.track_menu, self.playback.clone(), cx);

        Some(
            Popup::new(position, menu)
                .on_close(cx.listener(|this, _, _, cx| this.dismiss_menu(cx))),
        )
    }

    pub fn is_open(&self) -> bool {
        self.forced.unwrap_or(self.open && !self.cramped)
    }

    pub fn overlays(&self) -> bool {
        self.cramped && self.is_open()
    }

    fn dismiss(&mut self, cx: &mut Context<Self>) {
        if !self.overlays() {
            return;
        }
        self.forced = Some(false);
        cx.notify();
    }

    pub fn occupied_width(&self) -> Pixels {
        match self.is_open() && !self.overlays() {
            true => self.width,
            false => Pixels::ZERO,
        }
    }

    pub fn toggle(&mut self, cx: &mut Context<Self>) {
        match self.cramped {
            true => self.forced = Some(!self.is_open()),
            false => {
                self.open = !self.open;
                self.persist(cx);
            }
        }
        cx.notify();
    }

    fn ceiling(&self, window: &Window, cx: &Context<Self>) -> Pixels {
        let reserved = match self.overlays() {
            true => Pixels::ZERO,
            false => SNUG + super::Chrome::sidebar_right(cx),
        };

        super::cap(MIN_WIDTH, MAX_WIDTH, reserved, window)
    }

    pub fn adapt(&mut self, window: &Window, cx: &mut Context<Self>) {
        self.width = ui::snapped(self.width, window);

        let taken = self.width + super::Chrome::sidebar_right(cx);
        let space_left = window.viewport_size().width - taken;
        let cramped = space_left < SNUG;
        if cramped != self.cramped {
            self.cramped = cramped;
            self.forced = None;
        }
    }

    fn pins(&self, window: &Window, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let pinned = self.settings.read(cx).pinned().to_vec();
        if pinned.is_empty() && !self.dropping {
            return Vec::new();
        }

        let count = pinned.len();
        let mut rows = vec![super::section_label("nav-pinned", window, cx).into_any_element()];
        rows.extend(
            pinned
                .into_iter()
                .enumerate()
                .map(|(index, pin)| self.pin_row(index, pin, count, cx)),
        );
        if count == 0 {
            rows.push(hint(cx));
        }

        rows
    }

    fn pin_row(&self, index: usize, pin: Pin, count: usize, cx: &mut Context<Self>) -> AnyElement {
        let theme = *cx.theme();
        let accent = theme.sidebar_accent;
        let destination = Destination::from(&pin);
        let active = destination == self.trail.read(cx).current();
        let opened = pin.clone();
        let edge = match self.drop_gap {
            Some(gap) if gap == index => Some(Edge::Above),
            Some(gap) if gap == count && index + 1 == count => Some(Edge::Below),
            _ => None,
        };

        let origin = origin_of(&pin);
        let playing = matches!(
            self.playback.read(cx).playing_from(&origin),
            Some(PlaybackState::Playing)
        );

        let card = Card::new(("pinned", index), pin.label())
            .cover(pin.cover.clone())
            .fallback(pin.kind.icon())
            .when(pin.kind.round(), Card::circle)
            .play(
                playing,
                cx.listener(move |this, _, _, cx| {
                    this.playback
                        .update(cx, |playback, cx| playback.toggle_origin(&origin, cx));
                }),
            )
            .tint(match active {
                true => theme.foreground,
                false => theme.muted_foreground,
            })
            .meta(i18n::lookup(pin.kind.key(), None))
            .when(active, |card| card.bg(accent))
            .hover(move |style| style.bg(accent))
            .press(move |_, _, cx| navigate(destination.clone(), cx))
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                    window.prevent_default();
                    this.track_menu.reset(cx);
                    this.context_menu = Some((opened.clone(), event.position));
                    cx.stop_propagation();
                    cx.notify();
                }),
            )
            .pin(pin)
            .on_drag_move(
                cx.listener(move |this, event: &DragMoveEvent<DraggedPin>, _, cx| {
                    let Some(gap) = drop_gap(event.bounds, event.event.position, index) else {
                        return;
                    };
                    let dragged = event.drag(cx).pin.clone();
                    let from = this
                        .settings
                        .read(cx)
                        .pinned()
                        .iter()
                        .position(|it| it.same(&dragged));
                    let gap = match from {
                        Some(from) if gap == from || gap == from + 1 => None,
                        _ => Some(gap),
                    };
                    if this.drop_gap != gap {
                        this.drop_gap = gap;
                        cx.notify();
                    }
                }),
            );

        div()
            .id(("pinned-slot", index))
            .relative()
            .min_w_0()
            .child(card)
            .when_some(edge, |this, edge| this.child(drop_marker(edge, cx)))
            .into_any_element()
    }

    fn persist(&self, cx: &mut Context<Self>) {
        let width = self.width / px(1.);
        let open = self.open;
        self.settings
            .update(cx, |settings, cx| settings.set_sidebar(width, open, cx));
    }
}

impl Render for SidebarLeft {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.theme();
        let sidebar_accent = theme.sidebar_accent;
        let foreground = theme.foreground;
        let muted = theme.muted_foreground;
        let sidebar_bg = theme.sidebar;
        let sidebar_border = theme.sidebar_border;

        let current = self.trail.read(cx).current();
        self.follow(&current);
        let authenticated = self.session.read(cx).authenticated();
        let has_local = self.session.read(cx).local_client().is_some();
        self.adapt(window, cx);

        if !cx.has_active_drag() {
            self.dropping = false;
            self.drop_gap = None;
        }

        let mut rows: Vec<AnyElement> = Vec::new();
        for (index, (key, icon, destination)) in NAV.into_iter().enumerate() {
            if matches!(destination, Some(Destination::Library(_))) {
                if !authenticated && !has_local {
                    continue;
                }
                let inside = matches!(current, Destination::Library(_));
                let text = if inside { foreground } else { muted };
                let default_tab = if authenticated {
                    LibraryTab::Songs
                } else {
                    LibraryTab::Local
                };
                let destination = destination.map(|_| Destination::Library(default_tab));
                let link_destination = if inside { None } else { destination };
                let target = link_destination.unwrap_or(current.clone());

                rows.push(
                    nav_row(index, key, text, sidebar_accent)
                        .icon(icon)
                        .trailing(chevron(self.library_open))
                        .on_click(cx.listener(move |this, _, _, cx| match inside {
                            true => {
                                this.library_open = !this.library_open;
                                cx.notify();
                            }
                            false => navigate(target.clone(), cx),
                        }))
                        .into_any_element(),
                );

                if self.library_open {
                    rows.push(
                        Tabs::new()
                            .items(LIBRARY_TABS.into_iter().map(|(name, tab)| {
                                let chosen = current == Destination::Library(tab);
                                let tint = if chosen { foreground } else { muted };

                                nav_row(name, name, tint, sidebar_accent)
                                    .flex_1()
                                    .when(chosen, |button| button.bg(sidebar_accent))
                                    .on_click(move |_, _, cx| {
                                        navigate(Destination::Library(tab), cx)
                                    })
                            }))
                            .into_any_element(),
                    );
                }
                continue;
            }

            if matches!(destination, Some(Destination::Settings(_))) {
                let inside = matches!(current, Destination::Settings(_));
                let text = if inside { foreground } else { muted };
                let link_destination = if inside { None } else { destination };
                let target = link_destination.unwrap_or(current.clone());

                rows.push(
                    nav_row(index, key, text, sidebar_accent)
                        .icon(icon)
                        .trailing(chevron(self.settings_open))
                        .on_click(cx.listener(move |this, _, _, cx| match inside {
                            true => {
                                this.settings_open = !this.settings_open;
                                cx.notify();
                            }
                            false => navigate(target.clone(), cx),
                        }))
                        .into_any_element(),
                );

                if self.settings_open {
                    rows.push(
                        Tabs::new()
                            .items(SETTINGS_TABS.into_iter().map(|(name, tab)| {
                                let chosen = current == Destination::Settings(tab);
                                let tint = if chosen { foreground } else { muted };

                                nav_row(name, name, tint, sidebar_accent)
                                    .flex_1()
                                    .when(chosen, |button| button.bg(sidebar_accent))
                                    .on_click(move |_, _, cx| {
                                        navigate(Destination::Settings(tab), cx)
                                    })
                            }))
                            .into_any_element(),
                    );
                }
                continue;
            }

            let active = destination
                .as_ref()
                .is_some_and(|it| it.same_section(&current));
            let text = if active { foreground } else { muted };

            rows.push(
                nav_row(index, key, text, sidebar_accent)
                    .icon(icon)
                    .when(active, |button| button.bg(sidebar_accent))
                    .when_some(destination, |button, destination| {
                        button.on_click(move |_, _, cx| navigate(destination.clone(), cx))
                    })
                    .into_any_element(),
            );
        }

        rows.extend(self.pins(window, cx));

        let overlaid = self.overlays();
        let panel = Panel::new("sidebar-left", Side::Left, self.width)
            .limits(MIN_WIDTH, MAX_WIDTH)
            .reach(self.ceiling(window, cx))
            .on_resize(cx.listener(|this, width: &Pixels, _, cx| {
                this.width = *width;
                this.persist(cx);
                cx.notify();
            }))
            .on_drag_move(cx.listener(|this, _: &DragMoveEvent<DraggedPin>, _, cx| {
                let settled = this.drop_gap.take().is_some() || !this.dropping;
                this.dropping = true;
                if settled {
                    cx.notify();
                }
            }))
            .on_drop(cx.listener(|this, dragged: &DraggedPin, _, cx| {
                let gap = this.drop_gap.take();
                this.dropping = false;
                let pin = dragged.pin.clone();
                this.settings
                    .update(cx, |settings, cx| settings.pin(pin, gap, cx));
                cx.notify();
            }))
            .when(!self.is_open(), |this| this.hidden())
            .when(!theme.transparent, |this| this.bg(sidebar_bg))
            .border_color(sidebar_border)
            .when(overlaid, |this| {
                this.occlude().absolute().left_0().top_0().bottom_0()
            })
            .child(
                Scroller::new("sidebar-left-rows", &self.scrollbar)
                    .flex()
                    .flex_col()
                    .gap_1()
                    .flex_1()
                    .min_h_0()
                    .p_3()
                    .children(rows),
            )
            .children(self.menu(cx));

        match overlaid {
            false => panel.into_any_element(),
            true => div()
                .absolute()
                .top_0()
                .left_0()
                .right_0()
                .bottom_0()
                .child(
                    Shield::new("sidebar-shield")
                        .absolute()
                        .top_0()
                        .left_0()
                        .right_0()
                        .bottom_0()
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _: &MouseDownEvent, _, cx| this.dismiss(cx)),
                        ),
                )
                .child(panel)
                .into_any_element(),
        }
    }
}

fn origin_of(pin: &Pin) -> Origin {
    match pin.kind {
        PinKind::Album => Origin::Album(pin.id.clone()),
        PinKind::Playlist => Origin::Playlist(pin.id.clone()),
        PinKind::Artist => Origin::Artist(pin.id.clone()),
        PinKind::Song => Origin::Radio(pin.id.clone()),
    }
}

fn hint(cx: &App) -> AnyElement {
    let theme = *cx.theme();

    div()
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .h(HINT_HEIGHT)
        .mx_2()
        .px_2()
        .rounded(theme.radius)
        .border_1()
        .border_dashed()
        .border_color(theme.sidebar_border)
        .text_size(theme.text(Text::Small))
        .text_color(theme.muted_foreground)
        .text_center()
        .child(i18n::lookup("nav-pin-hint", None))
        .into_any_element()
}

fn expanded(current: &Destination) -> (bool, bool) {
    (
        matches!(current, Destination::Library(_)),
        matches!(current, Destination::Settings(_)),
    )
}

fn chevron(open: bool) -> &'static str {
    match open {
        true => "icons/chevron-down.svg",
        false => "icons/chevron-right.svg",
    }
}

fn nav_row(id: impl Into<ElementId>, key: &'static str, tint: Hsla, accent: Hsla) -> Button {
    Button::new(id)
        .ghost()
        .label(i18n::lookup(key, None))
        .tint(tint)
        .gap_2p5()
        .justify_start()
        .hover(move |style| style.bg(accent))
        .active(move |style| style.bg(accent))
}

#[cfg(test)]
mod tests {
    use router::{Destination, LibraryTab, SettingsTab};

    use super::expanded;

    #[test]
    fn a_section_expands_only_where_it_leads() {
        assert_eq!(
            expanded(&Destination::Library(LibraryTab::Albums)),
            (true, false)
        );
        assert_eq!(
            expanded(&Destination::Settings(SettingsTab::General)),
            (false, true)
        );
    }

    #[test]
    fn leaving_through_content_collapses_both() {
        let away = [
            Destination::Home,
            Destination::Search,
            Destination::Album("id".into()),
            Destination::Playlist("id".into()),
            Destination::Artist("id".into()),
            Destination::Song("id".into()),
        ];

        for destination in away {
            assert_eq!(expanded(&destination), (false, false), "{destination:?}");
        }
    }
}
