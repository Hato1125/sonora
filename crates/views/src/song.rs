use gpui::prelude::*;
use gpui::{
    AnyElement, Context, Entity, FontWeight, Render, SharedString, Window, div, px, relative,
};
use router::{Destination, Link as _};
use spotify::{Credit, Track};
use state::{Playback, SongDetail};
use ui::{ActiveTheme as _, Artwork, Avatar, Button, Initials, Scrollbar, Text, clock};

use crate::cells;
use crate::hero::{HeroMetaStrip, HeroPlayButton, PageHero, release_date_label};

pub(crate) struct SongView {
    detail: Entity<SongDetail>,
    playback: Entity<Playback>,
    scrollbar: Entity<Scrollbar>,
}

impl SongView {
    fn language_label(code: &str) -> &str {
        match code {
            "ar" => "Arabic",
            "de" => "German",
            "en" => "English",
            "es" => "Spanish",
            "fr" => "French",
            "hi" => "Hindi",
            "it" => "Italian",
            "ja" => "Japanese",
            "ko" => "Korean",
            "pt" => "Portuguese",
            "ru" => "Russian",
            "tr" => "Turkish",
            "uk" => "Ukrainian",
            "zh" => "Chinese",
            _ => code,
        }
    }

    pub(crate) fn new(
        detail: Entity<SongDetail>,
        playback: Entity<Playback>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&detail, |this, _, cx| {
            this.scrollbar
                .read(cx)
                .scroll()
                .set_offset(gpui::Point::default());
            cx.notify();
        })
        .detach();
        cx.observe(&playback, |_, _, cx| cx.notify()).detach();
        Self {
            detail,
            playback,
            scrollbar: cx.new(|_| Scrollbar::new(gpui::ScrollHandle::new())),
        }
    }

    fn card(
        &self,
        title: &'static str,
        body: impl IntoElement,
        fill: bool,
        cx: &Context<Self>,
    ) -> AnyElement {
        let theme = *cx.theme();
        div()
            .flex()
            .flex_col()
            .when(fill, |this| this.h_full())
            .gap_4()
            .p_5()
            .rounded(theme.radius)
            .border_1()
            .border_color(theme.border)
            .bg(theme.secondary.opacity(0.45))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .text_size(theme.text(Text::Small))
                    .text_color(theme.muted_foreground)
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(div().flex_none().child(title))
                    .child(div().h(px(1.)).flex_1().bg(theme.border)),
            )
            .child(body)
            .into_any_element()
    }

    fn fact(
        label: impl Into<SharedString>,
        value: impl Into<SharedString>,
        cx: &Context<Self>,
    ) -> AnyElement {
        let theme = *cx.theme();
        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_4()
            .py_1()
            .child(div().text_color(theme.muted_foreground).child(label.into()))
            .child(
                div()
                    .text_right()
                    .font_weight(FontWeight::MEDIUM)
                    .child(value.into()),
            )
            .into_any_element()
    }

    fn hero(&self, track: &Track, cx: &Context<Self>) -> AnyElement {
        let theme = *cx.theme();
        let cover = self
            .detail
            .read(cx)
            .album()
            .and_then(|album| album.album.cover_large.clone())
            .or_else(|| track.cover.clone());
        let album = self.detail.read(cx).album().map(|detail| &detail.album);
        let release = album
            .map(|album| release_date_label(&album.release_date))
            .filter(|release| !release.is_empty());

        let mut meta = HeroMetaStrip::new().item(cells::artist_links(
            "song-artists",
            track.artist_refs.clone(),
            track.artists.clone(),
            theme.muted_foreground,
        ));
        if let Some(release) = release {
            meta = meta.text(release);
        }
        meta = meta.text(clock(track.duration));

        let actions = div()
            .flex()
            .items_center()
            .gap_3()
            .child(HeroPlayButton::new(
                "play-song",
                "Play song",
                vec![track.clone()],
                self.playback.clone(),
            ))
            .when_some(track.album_id.clone(), |this, album_id| {
                this.child(
                    Button::new("open-album")
                        .label("View album")
                        .outline()
                        .on_click(move |_, _, cx| {
                            router::navigate(Destination::Album(album_id.clone().into()), cx);
                        }),
                )
            });

        PageHero::new("song-hero", track.name.clone())
            .cover(cover)
            .eyebrow("SONG")
            .meta(meta)
            .actions(actions)
            .explicit(track.explicit)
            .into_any_element()
    }

    fn overview(&self, track: &Track, cx: &Context<Self>) -> AnyElement {
        let album = self.detail.read(cx).album();
        let album_name = album
            .map(|detail| detail.album.name.clone())
            .unwrap_or_else(|| track.album.clone());
        let release = album
            .map(|detail| detail.album.release_date.clone())
            .unwrap_or_default();
        let release = if release.is_empty() {
            "Unknown".to_owned()
        } else {
            release_date_label(&release)
        };
        let label = album
            .map(|detail| detail.album.label.clone())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "Not provided".to_owned());
        let number = match (track.disc_number, track.track_number) {
            (disc, number) if disc > 1 => format!("Disc {disc}, track {number}"),
            (_, number) if number > 0 => format!("Track {number}"),
            _ => "Not provided".to_owned(),
        };
        let streams = self
            .detail
            .read(cx)
            .playcount()
            .map(count)
            .unwrap_or_else(|| "Not available".to_owned());
        self.card(
            "ABOUT THIS SONG",
            div()
                .flex()
                .flex_col()
                .child(Self::fact("Album", album_name, cx))
                .child(Self::fact("Released", release, cx))
                .child(Self::fact("Streams", streams, cx))
                .child(Self::fact("Position", number, cx))
                .child(Self::fact("Label", label, cx))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .pt_2()
                        .child(Self::fact(
                            "Popularity",
                            format!("{} / 100", track.popularity),
                            cx,
                        ))
                        .child(
                            div()
                                .w_full()
                                .h(px(4.))
                                .rounded_full()
                                .bg(cx.theme().muted)
                                .child(
                                    div()
                                        .h_full()
                                        .w(relative(track.popularity.min(100) as f32 / 100.))
                                        .rounded_full()
                                        .bg(cx.theme().progress_bar),
                                ),
                        ),
                ),
            true,
            cx,
        )
    }

    fn credits(&self, track: &Track, cx: &Context<Self>) -> AnyElement {
        let theme = *cx.theme();
        let rows: Vec<_> = if track.credits.is_empty() {
            track
                .artist_refs
                .iter()
                .map(|artist| Credit {
                    name: artist.name.clone(),
                    role: "Performed by".to_owned(),
                    id: artist.id.clone(),
                })
                .collect()
        } else {
            track.credits.clone()
        };
        let portraits = self.detail.read(cx).portraits().clone();
        self.card(
            "CREDITS",
            div()
                .flex()
                .flex_col()
                .gap_3()
                .children(rows.into_iter().enumerate().map(|(index, credit)| {
                    let portrait = credit.id.as_ref().and_then(|id| portraits.get(id)).cloned();
                    let avatar = match portrait {
                        Some(portrait) => Avatar::new(Some(portrait))
                            .size(theme.metrics.thumb)
                            .into_any_element(),
                        None => Initials::new(credit.name.clone(), theme.metrics.thumb)
                            .into_any_element(),
                    };
                    let row = div().flex().items_center().gap_3().child(avatar).child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_0p5()
                            .child(div().font_weight(FontWeight::MEDIUM).child(credit.name))
                            .child(
                                div()
                                    .text_size(theme.text(Text::Small))
                                    .text_color(theme.muted_foreground)
                                    .child(credit.role),
                            ),
                    );

                    match credit.id {
                        Some(id) => row
                            .id(("song-credit", index))
                            .cursor_pointer()
                            .hover(|style| style.bg(theme.secondary_hover))
                            .rounded(theme.radius)
                            .link(Destination::Artist(id.into()))
                            .into_any_element(),
                        None => row.into_any_element(),
                    }
                })),
            false,
            cx,
        )
    }

    fn discovery(&self, track: &Track, cx: &Context<Self>) -> AnyElement {
        let theme = *cx.theme();
        let tags = track.tags.clone();
        let languages = if track.languages.is_empty() {
            "Not provided".to_owned()
        } else {
            track
                .languages
                .iter()
                .map(|language| Self::language_label(language))
                .collect::<Vec<_>>()
                .join(", ")
        };
        self.card(
            "GENRES & DETAILS",
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(div().flex().flex_wrap().gap_2().when_else(
                    tags.is_empty(),
                    |this| this.child(Self::fact("Genres", "Not available", cx)),
                    |this| {
                        this.child(
                            div()
                                .flex()
                                .flex_wrap()
                                .justify_end()
                                .w_full()
                                .gap_2()
                                .py_1()
                                .children(tags.into_iter().map(|tag| {
                                    div()
                                        .px_3()
                                        .py_1()
                                        .rounded_full()
                                        .bg(theme.secondary)
                                        .border_1()
                                        .border_color(theme.border)
                                        .text_size(theme.text(Text::Small))
                                        .child(tag)
                                })),
                        )
                    },
                ))
                .child(Self::fact("Language", languages, cx))
                .child(Self::fact(
                    "Content",
                    if track.explicit { "Explicit" } else { "Clean" },
                    cx,
                )),
            false,
            cx,
        )
    }

    fn artist_profile(&self, cx: &Context<Self>) -> Option<AnyElement> {
        let theme = *cx.theme();
        let detail = self.detail.read(cx);
        let artist = detail.artist()?;
        let artist_id = detail.track()?.artist_refs.first()?.id.clone()?;
        let bio = artist
            .biography
            .clone()
            .unwrap_or_else(|| "Explore the artist's popular songs and releases.".to_owned());
        let bio = if bio.chars().count() > 360 {
            format!("{}…", bio.chars().take(360).collect::<String>())
        } else {
            bio
        };
        Some(
            div()
                .id("song-artist-profile")
                .flex()
                .items_center()
                .gap_5()
                .p_5()
                .rounded(theme.radius)
                .border_1()
                .border_color(theme.border)
                .cursor_pointer()
                .hover(|style| style.bg(theme.secondary))
                .link(Destination::Artist(artist_id.into()))
                .child(
                    Artwork::new(artist.cover_large.clone())
                        .size(px(88.))
                        .circle(),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_w_0()
                        .gap_2()
                        .child(
                            div()
                                .text_size(theme.text(Text::Small))
                                .text_color(theme.muted_foreground)
                                .font_weight(FontWeight::SEMIBOLD)
                                .child("ABOUT THE ARTIST"),
                        )
                        .child(
                            div()
                                .text_size(theme.text(Text::Title))
                                .font_weight(FontWeight::BOLD)
                                .child(artist.name.clone()),
                        )
                        .child(div().text_color(theme.muted_foreground).child(bio)),
                )
                .child(
                    div()
                        .flex_none()
                        .text_size(theme.text(Text::Small))
                        .font_weight(FontWeight::MEDIUM)
                        .child("View artist  →"),
                )
                .into_any_element(),
        )
    }
}

impl Render for SongView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.theme();
        let scroll = self.scrollbar.read(cx).scroll().clone();
        let (track, error, loading) = {
            let detail = self.detail.read(cx);
            (
                detail.track().cloned(),
                detail.error().map(str::to_owned),
                detail.is_loading(),
            )
        };

        div()
            .size_full()
            .child(
                div()
                    .id("song-page")
                    .size_full()
                    .overflow_y_scroll()
                    .track_scroll(&scroll)
                    .px(theme.metrics.inset)
                    .py(theme.metrics.inset)
                    .when(loading && track.is_none(), |this| {
                        this.child(
                            div()
                                .py_8()
                                .text_color(theme.muted_foreground)
                                .child("Loading song information…"),
                        )
                    })
                    .when_some(error, |this, error| {
                        this.child(div().pb_4().text_color(theme.danger).child(error))
                    })
                    .when_some(track, |this, track| {
                        this.child(self.hero(&track, cx))
                            .child(
                                div()
                                    .flex()
                                    .flex_wrap()
                                    .items_stretch()
                                    .gap_5()
                                    .child(
                                        div()
                                            .min_w(px(300.))
                                            .flex_1()
                                            .child(self.overview(&track, cx)),
                                    )
                                    .child(
                                        div()
                                            .min_w(px(300.))
                                            .flex_1()
                                            .flex()
                                            .flex_col()
                                            .gap_5()
                                            .child(self.credits(&track, cx))
                                            .child(self.discovery(&track, cx)),
                                    ),
                            )
                            .when_some(self.artist_profile(cx), |this, profile| {
                                this.child(div().pt_5().child(profile))
                            })
                            .when_some(
                                self.detail
                                    .read(cx)
                                    .album()
                                    .and_then(|album| album.album.copyrights.first())
                                    .cloned(),
                                |this, copyright| {
                                    let copyright = match copyright.starts_with(['©', '℗']) {
                                        true => copyright,
                                        false => format!("© {copyright}"),
                                    };
                                    this.child(
                                        div()
                                            .pt_5()
                                            .text_size(theme.text(Text::Tiny))
                                            .text_color(theme.muted_foreground)
                                            .child(copyright),
                                    )
                                },
                            )
                    }),
            )
            .child(self.scrollbar.clone())
    }
}

fn count(value: u64) -> String {
    let digits = value.to_string();
    let first = match digits.len() % 3 {
        0 => 3,
        remainder => remainder,
    };
    let mut grouped = digits[..first].to_owned();
    for chunk in digits.as_bytes()[first..].chunks(3) {
        grouped.push(',');
        grouped.push_str(std::str::from_utf8(chunk).unwrap_or_default());
    }
    grouped
}

#[cfg(test)]
mod tests {
    use super::count;

    #[test]
    fn groups_playcount() {
        assert_eq!(count(999), "999");
        assert_eq!(count(1_234), "1,234");
        assert_eq!(count(12_345_678), "12,345,678");
    }
}
