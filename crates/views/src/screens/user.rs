use gpui::prelude::*;
use gpui::{Context, Entity, Pixels, Render, ScrollHandle, SharedString, Window, div, px};
use i18n::t;
use state::{Playback, Profile};
use ui::{ActiveTheme as _, Card, Scrollbar, Scroller, heading, vacant};

use crate::chrome::Chrome;
use crate::shared::album_grid::CardGrid;
use crate::shared::cards;
use crate::shared::cells;
use crate::shared::hero::{HeroMetaStrip, PageHero};

const FALLBACK: &str = "icons/user.svg";
const PENDING: usize = 6;
const STEADY: Pixels = px(0.5);

pub(crate) struct UserView {
    profile: Entity<Profile>,
    playback: Entity<Playback>,
    scrollbar: Entity<Scrollbar>,
    width: Pixels,
}

impl UserView {
    pub(crate) fn new(
        profile: Entity<Profile>,
        playback: Entity<Playback>,
        cx: &mut Context<Self>,
    ) -> Self {
        let id = cx.entity_id();

        cx.observe(&profile, |this, _, cx| {
            this.scrollbar
                .read(cx)
                .scroll()
                .set_offset(gpui::Point::default());
            cx.notify();
        })
        .detach();
        cx.observe(&playback, |_, _, cx| cx.notify()).detach();

        let chrome = Chrome::entity(cx);
        cx.observe(&chrome, |_, _, cx| cx.notify()).detach();

        Self {
            profile,
            playback,
            scrollbar: cx.new(|_| Scrollbar::new(ScrollHandle::new()).watching(id)),
            width: Pixels::ZERO,
        }
    }

    fn header(&self, cx: &Context<Self>) -> impl IntoElement {
        let profile = self.profile.read(cx);
        let user = profile.user();
        let title = user
            .map(|user| SharedString::from(user.name.clone()))
            .or_else(|| profile.id().map(|id| SharedString::from(id.to_owned())))
            .unwrap_or_default();

        let mut strip = HeroMetaStrip::new();
        if let Some(followers) = user.and_then(|user| user.followers) {
            let value = cells::count(followers);
            strip = strip.text(t!("user-followers", count = followers, value = &value));
        }
        if let Some(following) = user.and_then(|user| user.following) {
            let value = cells::count(following);
            strip = strip.text(t!("user-following", count = following, value = &value));
        }

        PageHero::new("user-hero", title)
            .cover(user.and_then(|user| user.avatar.clone()))
            .circle()
            .fallback(FALLBACK)
            .eyebrow(t!("user-eyebrow"))
            .meta(strip)
    }

    fn playlists(&self, cx: &Context<Self>) -> impl IntoElement {
        let profile = self.profile.read(cx);
        let loading = profile.is_loading();
        let listed = profile.playlists();
        let layout = CardGrid::layout(self.width);

        let cards = match loading {
            true => (0..PENDING)
                .map(|place| {
                    Card::skeleton(("user-pending", place))
                        .tile(layout.card)
                        .into_any_element()
                })
                .collect(),
            false => listed
                .iter()
                .enumerate()
                .map(|(place, playlist)| {
                    cards::playlist_card(("user-playlist", place), playlist, &self.playback, cx)
                        .tile(layout.card)
                        .flat()
                        .into_any_element()
                })
                .collect::<Vec<_>>(),
        };
        let empty = cards.is_empty();

        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(heading(t!("user-playlists"), cx))
            .when(empty, |this| {
                this.child(vacant(t!("user-playlists-empty"), cx))
            })
            .when(!empty, |this| {
                this.child(
                    div()
                        .flex()
                        .flex_wrap()
                        .w_full()
                        .gap_x(layout.gap)
                        .gap_y_6()
                        .children(cards),
                )
            })
    }
}

impl Render for UserView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.theme();
        let pad = theme.metrics.inset;
        let room = cells::content_width(window, pad * 2., cx);
        if (room - self.width).abs() >= STEADY {
            self.width = room;
        }

        let error = self.profile.read(cx).error().map(str::to_owned);

        div().flex().flex_col().size_full().child(
            Scroller::new("user-page", &self.scrollbar).p(pad).child(
                div()
                    .flex()
                    .flex_col()
                    .gap_8()
                    .child(self.header(cx))
                    .children(error.map(|error| {
                        div()
                            .text_color(theme.danger)
                            .child(SharedString::from(error))
                    }))
                    .child(self.playlists(cx)),
            ),
        )
    }
}
