use gpui::prelude::*;
use gpui::{AnyView, App, Context, Entity, FocusHandle, Pixels, Render, SharedString};
use gpui::{Window, div};
use i18n::t;
use input::ToggleFullscreen;
use state::Playback;
use ui::{ActiveTheme as _, Artwork, Text};

use crate::chrome::TitleBarOptions;
use crate::shells::Shell;

const COVER: f32 = 2.4;

pub struct FullscreenView {
    playback: Entity<Playback>,
    focus: FocusHandle,
}

impl FullscreenView {
    pub fn new(playback: Entity<Playback>, cx: &mut Context<Self>) -> Self {
        cx.observe(&playback, |_, _, cx| cx.notify()).detach();

        Self {
            playback,
            focus: cx.focus_handle(),
        }
    }

    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        window.focus(&self.focus, cx);
    }
}

impl Shell for FullscreenView {
    fn title_bar(&self, _content: Option<AnyView>, _cx: &App) -> TitleBarOptions {
        TitleBarOptions {
            navigation: false,
            sidebar_open: false,
            sidebar_right: None,
            offset: Pixels::ZERO,
            content: None,
        }
    }
}

impl Render for FullscreenView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.theme();
        let cover = ui::snapped(theme.metrics.cover * COVER, window);
        let track = self.playback.read(cx).track().cloned();
        let title = match &track {
            Some(track) => SharedString::from(track.name.clone()),
            None => t!("player-nothing-playing"),
        };

        div()
            .id("fullscreen")
            .track_focus(&self.focus)
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .w_full()
            .items_center()
            .justify_center()
            .gap_6()
            .bg(theme.background)
            .on_click(|_, window, cx| window.dispatch_action(Box::new(ToggleFullscreen), cx))
            .child(Artwork::new(track.as_ref().and_then(|track| track.cover.clone())).size(cover))
            .child(
                div()
                    .max_w(cover)
                    .truncate()
                    .text_size(theme.text(Text::Title))
                    .child(title),
            )
    }
}
