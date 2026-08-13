use std::collections::HashMap;

use gpui::{Context, Hsla, SharedString, Task};

#[derive(Default)]
pub(crate) struct Tints {
    found: HashMap<SharedString, Option<Hsla>>,
    pending: HashMap<SharedString, Task<()>>,
}

impl Tints {
    pub(crate) fn of(&mut self, cover: &str, cx: &mut Context<Self>) -> Option<Hsla> {
        let key = SharedString::from(cover.to_owned());
        if let Some(found) = self.found.get(&key) {
            return *found;
        }
        if self.pending.contains_key(&key) {
            return None;
        }

        let tint = ui::tint(key.clone(), cx);
        let asked = key.clone();
        let task = cx.spawn(async move |this, cx| {
            let tint = tint.await;

            this.update(cx, |this, cx| {
                this.pending.remove(&asked);
                this.found.insert(asked.clone(), tint);
                cx.notify();
            })
            .ok();
        });
        self.pending.insert(key, task);

        None
    }
}
