// SPDX-License-Identifier: GPL-3.0-or-later

use gpui::prelude::*;
use gpui::{App, Context, Entity, Pixels, Render, SharedString, Window, div, px};
use input::{Dismiss, Input};
use ui::{Button, Menu, MenuItem, Toggle};

const WIDEST: Pixels = px(280.);
const MENU_WIDTH: Pixels = px(190.);
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

pub trait Tooled: 'static {
    fn toolbar(&self) -> Entity<Toolbar>;
}

pub struct Toolbar {
    input: Entity<Input>,
    apply: Option<Apply>,
    toggles: Option<Toggles>,
    switch: Option<Switch>,
    open: bool,
    picker: bool,
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
            open: false,
            picker: false,
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

impl Render for Toolbar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_1()
            .min_w_0()
            .items_center()
            .justify_end()
            .gap_1()
            .on_action(cx.listener(|this, _: &Dismiss, _, cx| this.close(cx)))
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
