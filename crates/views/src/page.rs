// SPDX-License-Identifier: GPL-3.0-or-later

use gpui::{App, Entity, Pixels, ScrollHandle, Window, px};

use state::{AppSettings, Playback};
use ui::{GridState, Table, Viewport, scrolled};

use crate::cells;
use crate::tracks::{self, TrackSource};

const FRAME: Pixels = px(1.);

pub(crate) fn store(
    settings: &Entity<AppSettings>,
    table: &dyn Table,
    layout_key: &str,
    sort_key: &str,
    cx: &mut App,
) {
    let layout = table.layout(cx);
    let sorting = table.sorting(cx);

    settings.update(cx, |settings, cx| {
        settings.set_table(layout_key, layout, cx);
        settings.set_sorting(sort_key, sorting, cx);
    });
}

pub(crate) fn reserved(inset: Pixels) -> Pixels {
    inset * 2. + px(2.)
}

pub(crate) fn play(
    table: &Entity<GridState<TrackSource>>,
    playback: &Entity<Playback>,
    display: usize,
    cx: &mut App,
) {
    let queued = tracks::ordered(table, cx);
    playback.update(cx, |playback, cx| playback.start(queued, display, cx));
}

pub(crate) fn resize(
    table: &dyn Table,
    width: &mut Pixels,
    inset: Pixels,
    window: &Window,
    cx: &mut App,
) {
    let next = cells::content_width(window, reserved(inset), cx);
    if (next - *width).abs() < px(0.5) {
        return;
    }
    *width = next;
    table.set_width(next, cx);
}

pub(crate) fn viewport(scroll: &ScrollHandle, inset: Pixels, window: &Window) -> Viewport {
    let hero = scroll
        .bounds_for_item(0)
        .map(|bounds| bounds.size.height)
        .unwrap_or_default();
    let visible = scroll.bounds().size.height;

    Viewport {
        top: (scrolled(scroll) - inset - hero - FRAME).max(Pixels::ZERO),
        height: match visible > Pixels::ZERO {
            true => visible,
            false => window.viewport_size().height,
        },
    }
}
