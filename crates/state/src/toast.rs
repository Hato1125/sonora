use std::time::Duration;

use gpui::{App, AppContext as _, Context, Entity, Global, SharedString};

const LINGER: Duration = Duration::from_secs(4);

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Done,
    Failed,
}

#[derive(Clone)]
pub struct Toast {
    pub id: usize,
    pub outcome: Outcome,
    pub key: SharedString,
    pub name: Option<SharedString>,
}

pub struct Toasts {
    shown: Vec<Toast>,
    next: usize,
}

struct Installed(Entity<Toasts>);

impl Global for Installed {}

impl Toasts {
    pub fn entity(cx: &mut App) -> Entity<Self> {
        if cx.try_global::<Installed>().is_none() {
            let toasts = cx.new(|_| Self {
                shown: Vec::new(),
                next: 0,
            });
            cx.set_global(Installed(toasts));
        }
        cx.global::<Installed>().0.clone()
    }

    pub fn show(outcome: Outcome, key: impl Into<SharedString>, cx: &mut App) {
        let toasts = Self::entity(cx);
        toasts.update(cx, |this, cx| this.push(outcome, key.into(), None, cx));
    }

    pub fn about(
        outcome: Outcome,
        key: impl Into<SharedString>,
        name: impl Into<SharedString>,
        cx: &mut App,
    ) {
        let toasts = Self::entity(cx);
        let name = Some(name.into());
        toasts.update(cx, |this, cx| this.push(outcome, key.into(), name, cx));
    }

    pub fn shown(&self) -> &[Toast] {
        &self.shown
    }

    pub fn dismiss(&mut self, id: usize, cx: &mut Context<Self>) {
        self.shown.retain(|toast| toast.id != id);
        cx.notify();
    }

    fn push(
        &mut self,
        outcome: Outcome,
        key: SharedString,
        name: Option<SharedString>,
        cx: &mut Context<Self>,
    ) {
        let showing = self
            .shown
            .iter()
            .any(|toast| toast.outcome == outcome && toast.key == key && toast.name == name);
        if showing {
            return;
        }

        let id = self.next;
        self.next += 1;
        self.shown.push(Toast {
            id,
            outcome,
            key,
            name,
        });
        cx.notify();

        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(LINGER).await;
            this.update(cx, |this, cx| this.dismiss(id, cx)).ok();
        })
        .detach();
    }
}
