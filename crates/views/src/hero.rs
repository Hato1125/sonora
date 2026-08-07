use gpui::prelude::*;
use gpui::{AnyElement, App, ElementId, Entity, FontWeight, SharedString, Window, div};
use spotify::Track;
use state::{Playback, PlaybackState};
use ui::{ActiveTheme as _, Button, Card, Text};

pub(crate) fn release_date_label(value: &str) -> String {
    let parts: Vec<_> = value.split('-').collect();
    if parts.len() != 3 {
        return value.to_owned();
    }
    let month = match parts[1] {
        "01" => "Jan",
        "02" => "Feb",
        "03" => "Mar",
        "04" => "Apr",
        "05" => "May",
        "06" => "Jun",
        "07" => "Jul",
        "08" => "Aug",
        "09" => "Sep",
        "10" => "Oct",
        "11" => "Nov",
        "12" => "Dec",
        _ => return value.to_owned(),
    };
    let day = parts[2].trim_start_matches('0');
    format!("{month} {day}, {}", parts[0])
}

#[derive(IntoElement, Default)]
pub(crate) struct HeroMetaStrip {
    items: Vec<AnyElement>,
}

impl HeroMetaStrip {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn text(mut self, text: impl Into<SharedString>) -> Self {
        self.items
            .push(div().flex_none().child(text.into()).into_any_element());
        self
    }

    pub(crate) fn item(mut self, item: impl IntoElement) -> Self {
        self.items.push(item.into_any_element());
        self
    }
}

impl RenderOnce for HeroMetaStrip {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = *cx.theme();
        let mut strip = div()
            .flex()
            .flex_wrap()
            .items_center()
            .min_w_0()
            .gap_1()
            .text_size(theme.text(Text::Small))
            .text_color(theme.muted_foreground);

        for (index, item) in self.items.into_iter().enumerate() {
            strip = strip.child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap_1()
                    .when(index > 0, |this| this.child("•"))
                    .child(item),
            );
        }

        strip
    }
}

#[derive(IntoElement)]
pub(crate) struct HeroPlayButton {
    id: ElementId,
    label: SharedString,
    tracks: Vec<Track>,
    playback: Entity<Playback>,
}

impl HeroPlayButton {
    pub(crate) fn new(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        tracks: Vec<Track>,
        playback: Entity<Playback>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            tracks,
            playback,
        }
    }
}

impl RenderOnce for HeroPlayButton {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let first_playable = self.tracks.iter().position(|track| track.playable);
        let state = {
            let playback = self.playback.read(cx);
            let current = playback.track().and_then(|track| track.id.as_deref());
            current
                .filter(|current| {
                    self.tracks
                        .iter()
                        .any(|track| track.id.as_deref() == Some(*current))
                })
                .map(|_| playback.state().clone())
        };
        let (label, icon, blocked) = match &state {
            Some(PlaybackState::Playing) => ("Pause".into(), "icons/pause.svg", false),
            Some(PlaybackState::Paused) => ("Resume".into(), "icons/play.svg", false),
            Some(PlaybackState::Loading) => ("Loading…".into(), "icons/play.svg", true),
            _ => (self.label, "icons/play.svg", false),
        };
        let disabled = first_playable.is_none() || blocked;
        let first_playable = first_playable.unwrap_or_default();
        let tracks = self.tracks;
        let playback = self.playback;

        div().flex().child(
            Button::new(self.id)
                .label(label)
                .icon(icon)
                .primary()
                .disabled(disabled)
                .on_click(move |_, _, cx| {
                    playback.update(cx, |playback, cx| match &state {
                        Some(PlaybackState::Playing) => playback.pause(cx),
                        Some(PlaybackState::Paused) => playback.resume(cx),
                        Some(PlaybackState::Loading) => {}
                        _ => playback.start(tracks.clone(), first_playable, cx),
                    });
                }),
        )
    }
}

#[derive(IntoElement)]
pub(crate) struct PageHero {
    id: ElementId,
    title: SharedString,
    cover: Option<String>,
    eyebrow: Option<SharedString>,
    meta: Option<AnyElement>,
    actions: Option<AnyElement>,
    circle: bool,
    explicit: bool,
}

impl PageHero {
    pub(crate) fn new(id: impl Into<ElementId>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            cover: None,
            eyebrow: None,
            meta: None,
            actions: None,
            circle: false,
            explicit: false,
        }
    }

    pub(crate) fn cover(mut self, cover: Option<String>) -> Self {
        self.cover = cover;
        self
    }

    pub(crate) fn eyebrow(mut self, eyebrow: impl Into<SharedString>) -> Self {
        self.eyebrow = Some(eyebrow.into());
        self
    }

    pub(crate) fn meta(mut self, meta: impl IntoElement) -> Self {
        self.meta = Some(meta.into_any_element());
        self
    }

    pub(crate) fn actions(mut self, actions: impl IntoElement) -> Self {
        self.actions = Some(actions.into_any_element());
        self
    }

    pub(crate) fn circle(mut self) -> Self {
        self.circle = true;
        self
    }

    pub(crate) fn explicit(mut self, explicit: bool) -> Self {
        self.explicit = explicit;
        self
    }
}

impl RenderOnce for PageHero {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = *cx.theme();

        Card::new(self.id, self.title)
            .art(theme.metrics.cover)
            .art_radius(theme.radius * 1.5)
            .match_art_height()
            .cover(self.cover)
            .size(Text::Display)
            .weight(FontWeight::BOLD)
            .explicit_gap(theme.metrics.pad * 1.5)
            .flat()
            .flex_none()
            .items_end()
            .gap_5()
            .px_0()
            .pb_6()
            .when_some(self.eyebrow, Card::eyebrow)
            .when_some(self.meta, Card::bare_meta)
            .when_some(self.actions, Card::footer)
            .when(self.circle, Card::circle)
            .when(self.explicit, Card::explicit)
    }
}
