mod accounts;
mod browsers;

use gpui::prelude::*;
use gpui::{ElementId, Entity, Pixels};
use ui::{Input, Menu, Scrollbar};

pub(crate) use accounts::AccountPicker;
pub(crate) use browsers::BrowserPicker;

pub(crate) fn searchable_menu(
    id: impl Into<ElementId>,
    input: Entity<Input>,
    width: Pixels,
) -> Menu {
    Menu::new(id).w(width).header(input)
}

pub(crate) fn searchable_scroll_menu(
    id: impl Into<ElementId>,
    input: Entity<Input>,
    width: Pixels,
    height: Pixels,
    scrollbar: Entity<Scrollbar>,
) -> Menu {
    searchable_menu(id, input, width)
        .max_h(height)
        .scrollbar(scrollbar)
}

pub(crate) fn matches_query(id: &str, label: &str, query: &str) -> bool {
    query.is_empty() || label.to_lowercase().contains(query) || id.to_lowercase().contains(query)
}
