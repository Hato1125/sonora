// SPDX-License-Identifier: GPL-3.0-or-later

use gpui::prelude::*;
use gpui::{App, Context, Entity, Pixels, Render, SharedString, WeakEntity, Window, div, px};
use input::{Dismiss, Input};
use ui::{
    ActiveTheme as _, Button, FlagAxis, Menu, MenuItem, RangeAxis, RangeScrubber, RangeState,
    Toggle, eyebrow,
};

const WIDEST: Pixels = px(280.);
const MENU_WIDTH: Pixels = px(190.);
const FILTER_WIDTH: Pixels = px(260.);
const MENU_DROP: Pixels = px(30.);

type Apply = Box<dyn Fn(&str, &mut App)>;
type Toggles = Box<dyn Fn(&App) -> Vec<Toggle>>;
type Switch = Box<dyn Fn(&'static str, &mut App)>;

pub trait Searchable: 'static {
    fn search(&mut self, query: &str, cx: &mut Context<Self>)
    where
        Self: Sized;

    fn hint() -> SharedString
    where
        Self: Sized,
    {
        "common-search".into()
    }
}

pub trait Columned: 'static {
    fn toggles(&self, cx: &App) -> Vec<Toggle>;

    fn toggle_column(&mut self, key: &'static str, cx: &mut Context<Self>)
    where
        Self: Sized;
}

pub trait Filterable: 'static {
    fn ranges(&self, cx: &App) -> Vec<RangeAxis>;

    fn flags(&self, cx: &App) -> Vec<FlagAxis>;

    fn set_range(&mut self, key: &'static str, value: (f32, f32), cx: &mut Context<Self>)
    where
        Self: Sized;

    fn set_flag(&mut self, key: &'static str, on: bool, cx: &mut Context<Self>)
    where
        Self: Sized;

    fn reset_filters(&mut self, cx: &mut Context<Self>)
    where
        Self: Sized;
}

trait Port {
    fn ranges(&self, cx: &App) -> Vec<RangeAxis>;
    fn flags(&self, cx: &App) -> Vec<FlagAxis>;
    fn set_range(&self, key: &'static str, value: (f32, f32), cx: &mut App);
    fn set_flag(&self, key: &'static str, on: bool, cx: &mut App);
    fn reset(&self, cx: &mut App);
}

struct Wire<V>(WeakEntity<V>);

impl<V: Filterable> Port for Wire<V> {
    fn ranges(&self, cx: &App) -> Vec<RangeAxis> {
        self.0
            .upgrade()
            .map(|view| view.read(cx).ranges(cx))
            .unwrap_or_default()
    }

    fn flags(&self, cx: &App) -> Vec<FlagAxis> {
        self.0
            .upgrade()
            .map(|view| view.read(cx).flags(cx))
            .unwrap_or_default()
    }

    fn set_range(&self, key: &'static str, value: (f32, f32), cx: &mut App) {
        self.0
            .update(cx, |view, cx| view.set_range(key, value, cx))
            .ok();
    }

    fn set_flag(&self, key: &'static str, on: bool, cx: &mut App) {
        self.0
            .update(cx, |view, cx| view.set_flag(key, on, cx))
            .ok();
    }

    fn reset(&self, cx: &mut App) {
        self.0.update(cx, |view, cx| view.reset_filters(cx)).ok();
    }
}

pub trait Tooled: 'static {
    fn toolbar(&self) -> Entity<Toolbar>;
}

pub struct Toolbar {
    input: Entity<Input>,
    apply: Option<Apply>,
    toggles: Option<Toggles>,
    switch: Option<Switch>,
    port: Option<Box<dyn Port>>,
    sliders: Vec<(&'static str, RangeState)>,
    open: bool,
    picker: bool,
    sifting: bool,
}

impl Toolbar {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| {
            Input::new("common-search", cx)
                .icon("icons/search.svg")
                .compact()
        });

        cx.observe(&input, |this, input, cx| {
            let query = input.read(cx).text().to_owned();
            if let Some(apply) = &this.apply {
                apply(&query, cx);
            }
            cx.notify();
        })
        .detach();

        Self {
            input,
            apply: None,
            toggles: None,
            switch: None,
            port: None,
            sliders: Vec::new(),
            open: false,
            picker: false,
            sifting: false,
        }
    }

    pub fn bind<V: Searchable>(&mut self, view: &Entity<V>, cx: &mut Context<Self>) {
        let target = view.downgrade();
        self.apply = Some(Box::new(move |query, cx| {
            let query = query.to_owned();
            target.update(cx, |view, cx| view.search(&query, cx)).ok();
        }));
        self.input
            .update(cx, |input, cx| input.set_hint(V::hint(), cx));
        cx.notify();
    }

    pub fn columns<V: Columned>(&mut self, view: &Entity<V>, cx: &mut Context<Self>) {
        let read = view.downgrade();
        let write = view.downgrade();

        self.toggles = Some(Box::new(move |cx| {
            read.upgrade()
                .map(|view| view.read(cx).toggles(cx))
                .unwrap_or_default()
        }));
        self.switch = Some(Box::new(move |key, cx| {
            write
                .update(cx, |view, cx| view.toggle_column(key, cx))
                .ok();
        }));

        cx.observe(view, |_, _, cx| cx.notify()).detach();
        cx.notify();
    }

    pub fn filters<V: Filterable>(&mut self, view: &Entity<V>, cx: &mut Context<Self>) {
        self.port = Some(Box::new(Wire(view.downgrade())));
        cx.observe(view, |_, _, cx| cx.notify()).detach();
        cx.notify();
    }

    pub fn focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.apply.is_none() {
            return;
        }

        self.open = true;
        self.input.update(cx, |input, cx| input.focus(window, cx));
        cx.notify();
    }

    fn toggle(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.open {
            true => self.close(cx),
            false => self.focus(window, cx),
        }
    }

    fn close(&mut self, cx: &mut Context<Self>) {
        self.open = false;
        self.input.update(cx, |input, cx| input.set_text("", cx));
        cx.notify();
    }

    fn menu(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let toggles = match self.toggles.as_ref() {
            Some(toggles) => toggles(cx),
            None => Vec::new(),
        };

        div()
            .relative()
            .flex()
            .flex_none()
            .items_center()
            .child(
                Button::new("columns-toggle")
                    .icon("icons/columns-3.svg")
                    .small()
                    .ghost()
                    .selected(self.picker)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.picker = !this.picker;
                        cx.notify();
                    })),
            )
            .when(self.picker, |this| {
                this.child(
                    Menu::new("columns-menu")
                        .top(MENU_DROP)
                        .right_0()
                        .w(MENU_WIDTH)
                        .on_dismiss(cx.listener(|this, _, _, cx| {
                            this.picker = false;
                            cx.notify();
                        }))
                        .items(toggles.into_iter().map(|toggle| {
                            let key = toggle.key;
                            MenuItem::new(key, toggle.label)
                                .selected(toggle.visible)
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    if let Some(switch) = this.switch.as_ref() {
                                        switch(key, cx);
                                    }
                                    cx.notify();
                                }))
                        })),
                )
            })
    }
}

impl Toolbar {
    fn slider(&mut self, key: &'static str) -> RangeState {
        if let Some((_, state)) = self.sliders.iter().find(|(known, _)| *known == key) {
            return state.clone();
        }

        let state = RangeState::new(key);
        self.sliders.push((key, state.clone()));
        state
    }

    fn sift(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.theme();
        let ranges = match self.port.as_ref() {
            Some(port) => port.ranges(cx),
            None => Vec::new(),
        };
        let flags = match self.port.as_ref() {
            Some(port) => port.flags(cx),
            None => Vec::new(),
        };
        let narrowed = ranges.iter().any(|axis| !axis.whole()) || flags.iter().any(|flag| flag.on);

        let states: Vec<RangeState> = ranges.iter().map(|axis| self.slider(axis.key)).collect();

        let sliders: Vec<MenuItem> = ranges
            .iter()
            .zip(states.iter())
            .map(|(axis, state)| {
                let key = axis.key;
                let unit = axis.unit;
                let copy = axis.clone();
                MenuItem::new(key, axis.label.clone()).content(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .py_1()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap_2()
                                .child(eyebrow(axis.label.clone(), cx))
                                .child(
                                    div()
                                        .text_size(theme.text(ui::Text::Small))
                                        .text_color(theme.muted_foreground)
                                        .child(format!(
                                            "{} - {}",
                                            unit.say(axis.value.0),
                                            unit.say(axis.value.1)
                                        )),
                                ),
                        )
                        .child(
                            RangeScrubber::new(state, axis.share())
                                .stops(axis.stops())
                                .colors(theme.progress_bar, theme.muted, theme.foreground)
                                .on_change(cx.listener(move |this, share: &(f32, f32), _, cx| {
                                    if let Some(port) = this.port.as_ref() {
                                        port.set_range(key, copy.at(*share), cx);
                                    }
                                    cx.notify();
                                })),
                        ),
                )
            })
            .collect();

        let switches: Vec<MenuItem> = flags
            .iter()
            .map(|flag| {
                let key = flag.key;
                let on = flag.on;
                MenuItem::new(key, flag.label.clone())
                    .selected(on)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(port) = this.port.as_ref() {
                            port.set_flag(key, !on, cx);
                        }
                        cx.notify();
                    }))
            })
            .collect();

        div()
            .relative()
            .flex()
            .flex_none()
            .items_center()
            .child(
                Button::new("filters-toggle")
                    .icon("icons/sliders-horizontal.svg")
                    .small()
                    .ghost()
                    .selected(self.sifting)
                    .tint(match narrowed {
                        true => theme.primary,
                        false => theme.muted_foreground,
                    })
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.sifting = !this.sifting;
                        cx.notify();
                    })),
            )
            .when(self.sifting, |this| {
                this.child(
                    Menu::new("filters-menu")
                        .top(MENU_DROP)
                        .right_0()
                        .w(FILTER_WIDTH)
                        .on_dismiss(cx.listener(|this, _, _, cx| {
                            this.sifting = false;
                            cx.notify();
                        }))
                        .items(sliders)
                        .items(switches)
                        .item(MenuItem::separator("filters-end"))
                        .item(
                            MenuItem::new("filters-reset", i18n::t!("filter-reset")).on_click(
                                cx.listener(|this, _, _, cx| {
                                    if let Some(port) = this.port.as_ref() {
                                        port.reset(cx);
                                    }
                                    cx.notify();
                                }),
                            ),
                        ),
                )
            })
    }
}

impl Render for Toolbar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let sift = match self.port.is_some() {
            true => Some(self.sift(cx).into_any_element()),
            false => None,
        };

        div()
            .flex()
            .flex_1()
            .min_w_0()
            .items_center()
            .justify_end()
            .gap_1()
            .on_action(cx.listener(|this, _: &Dismiss, _, cx| this.close(cx)))
            .children(sift)
            .when(self.toggles.is_some(), |this| this.child(self.menu(cx)))
            .when(self.apply.is_some(), |this| {
                this.when(self.open, |this| {
                    this.child(
                        div()
                            .flex()
                            .flex_1()
                            .min_w_0()
                            .max_w(WIDEST)
                            .child(self.input.clone()),
                    )
                })
                .child(
                    Button::new("search-toggle")
                        .icon(match self.open {
                            true => "icons/x.svg",
                            false => "icons/search.svg",
                        })
                        .small()
                        .ghost()
                        .on_click(cx.listener(|this, _, window, cx| this.toggle(window, cx))),
                )
            })
    }
}
