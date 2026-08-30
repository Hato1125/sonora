use gpui::prelude::*;
use gpui::{App, Context, Entity, FocusHandle, Global, Render, Window, div};
use i18n::t;
use ui::{ActiveTheme as _, Button, Dismiss, FORM_CONTEXT, Modal, Submit};

#[derive(Clone, Copy)]
pub(crate) enum Kind {
    LibrarySongs(usize),
    PlaylistSongs(usize),
    History(usize),
    Albums(usize),
    Artists(usize),
    Playlists(usize),
}

impl Kind {
    fn title(&self) -> gpui::SharedString {
        match self {
            Self::PlaylistSongs(_) => t!("confirm-remove-playlist-title"),
            Self::History(_) => t!("confirm-remove-history-title"),
            Self::Artists(_) => t!("confirm-unfollow-title"),
            _ => t!("confirm-remove-library-title"),
        }
    }

    fn detail(&self) -> gpui::SharedString {
        match *self {
            Self::LibrarySongs(count) => t!("confirm-remove-songs", count = count),
            Self::PlaylistSongs(count) => t!("confirm-remove-playlist-songs", count = count),
            Self::History(count) => t!("confirm-remove-history-songs", count = count),
            Self::Albums(count) => t!("confirm-remove-albums", count = count),
            Self::Artists(count) => t!("confirm-unfollow-artists", count = count),
            Self::Playlists(count) => t!("confirm-remove-playlists", count = count),
        }
    }

    fn action(&self) -> gpui::SharedString {
        match self {
            Self::Artists(_) => t!("artist-unfollow"),
            _ => t!("common-delete"),
        }
    }
}

type Apply = Box<dyn FnOnce(&mut App)>;

struct Pending {
    kind: Kind,
    apply: Apply,
}

pub(crate) struct Confirm {
    pending: Option<Pending>,
    focus: FocusHandle,
    restore: Option<FocusHandle>,
    grab: bool,
}

struct Installed(Entity<Confirm>);

impl Global for Installed {}

impl Confirm {
    pub fn entity(cx: &mut App) -> Entity<Self> {
        if cx.try_global::<Installed>().is_none() {
            let confirm = cx.new(|cx| Self {
                pending: None,
                focus: cx.focus_handle(),
                restore: None,
                grab: false,
            });
            cx.set_global(Installed(confirm));
        }
        cx.global::<Installed>().0.clone()
    }

    pub fn ask(kind: Kind, apply: impl FnOnce(&mut App) + 'static, cx: &mut App) {
        let confirm = Self::entity(cx);
        confirm.update(cx, |this, cx| {
            this.pending = Some(Pending {
                kind,
                apply: Box::new(apply),
            });
            this.grab = true;
            cx.notify();
        });
    }

    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.pending = None;
        self.grab = false;
        if let Some(focus) = self.restore.take() {
            window.focus(&focus, cx);
        }
        cx.notify();
    }

    fn apply(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(pending) = self.pending.take() else {
            return;
        };
        self.grab = false;
        if let Some(focus) = self.restore.take() {
            window.focus(&focus, cx);
        }
        (pending.apply)(cx);
        cx.notify();
    }
}

impl Render for Confirm {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(pending) = self.pending.as_ref() else {
            return div().into_any_element();
        };
        if self.grab {
            self.restore = window.focused(cx);
            window.focus(&self.focus, cx);
            self.grab = false;
        }

        let title = pending.kind.title();
        let detail = pending.kind.detail();
        let action = pending.kind.action();
        let theme = *cx.theme();

        div()
            .absolute()
            .inset_0()
            .key_context(FORM_CONTEXT)
            .track_focus(&self.focus)
            .on_action(cx.listener(|this, _: &Dismiss, window, cx| {
                cx.stop_propagation();
                this.close(window, cx);
            }))
            .on_action(cx.listener(|this, _: &Submit, window, cx| {
                cx.stop_propagation();
                this.apply(window, cx);
            }))
            .child(
                Modal::new("confirm-remove", title)
                    .w(theme.metrics.cover * 2.8)
                    .detail(detail)
                    .action(
                        Button::new("cancel-confirm")
                            .ghost()
                            .label(t!("common-cancel"))
                            .on_click(cx.listener(|this, _, window, cx| this.close(window, cx))),
                    )
                    .action(
                        Button::new("apply-confirm")
                            .danger()
                            .label(action)
                            .on_click(cx.listener(|this, _, window, cx| this.apply(window, cx))),
                    )
                    .on_dismiss(cx.listener(|this, _, window, cx| this.close(window, cx))),
            )
            .into_any_element()
    }
}
