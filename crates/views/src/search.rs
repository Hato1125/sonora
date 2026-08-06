use gpui::prelude::*;
use gpui::{
    AnyElement, App, Context, ElementId, Entity, FontWeight, Hsla, Pixels, Render, ScrollHandle,
    SharedString, Window, div, px,
};
use input::Input;
use router::{Destination, navigate};
use state::{Hit, Kind, Playback, Search};
use ui::ActiveTheme as _;
use ui::{Artwork, Row, Scrollbar, Text, clock};
use workspace::Sidebar;

use crate::cells;

const READABLE: Pixels = px(1180.);
const COLUMNS: Pixels = px(720.);

enum Press {
    Song(usize),
    Album(String),
}

pub(crate) struct SearchView {
    input: Entity<Input>,
    search: Entity<Search>,
    playback: Entity<Playback>,
    sidebar: Entity<Sidebar>,
    songs: Entity<Scrollbar>,
    artists: Entity<Scrollbar>,
    albums: Entity<Scrollbar>,
    mixed: Entity<Scrollbar>,
}

impl SearchView {
    pub(crate) fn new(
        search: Entity<Search>,
        playback: Entity<Playback>,
        sidebar: Entity<Sidebar>,
        cx: &mut Context<Self>,
    ) -> Self {
        let input =
            cx.new(|cx| Input::new("What do you want to listen to?", cx).icon("icons/search.svg"));

        cx.observe(&input, |this, input, cx| {
            let query = input.read(cx).text().to_owned();
            this.search.update(cx, |search, cx| search.ask(&query, cx));
        })
        .detach();

        cx.observe(&search, |_, _, cx| cx.notify()).detach();
        cx.observe(&sidebar, |_, _, cx| cx.notify()).detach();
        cx.observe(&playback, |_, _, cx| cx.notify()).detach();

        let asked = input.read(cx).text().to_owned();
        search.update(cx, |search, cx| search.ask(&asked, cx));

        Self {
            input,
            search,
            playback,
            sidebar,
            songs: cx.new(|_| Scrollbar::new(ScrollHandle::new())),
            artists: cx.new(|_| Scrollbar::new(ScrollHandle::new())),
            albums: cx.new(|_| Scrollbar::new(ScrollHandle::new())),
            mixed: cx.new(|_| Scrollbar::new(ScrollHandle::new())),
        }
    }

    pub(crate) fn focus(&self, window: &mut Window, cx: &mut App) {
        self.input.update(cx, |input, cx| input.focus(window, cx));
    }

    fn play(&mut self, index: usize, cx: &mut Context<Self>) {
        let queued = self.search.read(cx).queue();
        if index >= queued.len() {
            return;
        }
        self.playback
            .update(cx, |playback, cx| playback.start(queued, index, cx));
    }

    fn press(
        &self,
        id: impl Into<ElementId>,
        target: Press,
        child: impl IntoElement,
        cx: &Context<Self>,
    ) -> AnyElement {
        div()
            .id(id)
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, _, cx| match &target {
                Press::Song(index) => this.play(*index, cx),
                Press::Album(id) => navigate(Destination::Album(id.clone().into()), cx),
            }))
            .child(child)
            .into_any_element()
    }

    fn playing(&self, cx: &Context<Self>) -> Option<String> {
        self.playback
            .read(cx)
            .track()
            .and_then(|current| current.id.clone())
    }

    fn row(&self, hit: &Hit, place: usize, compact: bool, cx: &Context<Self>) -> AnyElement {
        let theme = *cx.theme();
        let meta = meta(hit, compact);

        match hit {
            Hit::Song(track) => {
                let tint = match track.id.is_some() && track.id == self.playing(cx) {
                    true => theme.primary,
                    false => theme.foreground,
                };
                self.press(
                    ("song", place),
                    Press::Song(place),
                    Row::new(track.name.clone())
                        .cover(track.cover.clone())
                        .tint(tint)
                        .meta(meta)
                        .trailing(
                            div()
                                .flex_none()
                                .text_size(theme.text(Text::Small))
                                .text_color(theme.muted_foreground)
                                .child(clock(track.duration)),
                        ),
                    cx,
                )
            }
            Hit::Artist(artist) => Row::new(artist.name.clone())
                .cover(artist.cover.clone())
                .circle()
                .meta(meta)
                .into_any_element(),
            Hit::Album(album) => self.press(
                ElementId::Name(album.id.clone().into()),
                Press::Album(album.id.clone()),
                Row::new(album.name.clone())
                    .cover(album.cover.clone())
                    .meta(meta),
                cx,
            ),
        }
    }

    fn best(&self, cx: &Context<Self>) -> Option<AnyElement> {
        let theme = *cx.theme();
        let hit = self.search.read(cx).best()?;
        let (kind, title, target) = match hit {
            Hit::Song(track) => (Kind::Song, track.name.clone(), Some(Press::Song(0))),
            Hit::Artist(artist) => (Kind::Artist, artist.name.clone(), None),
            Hit::Album(album) => (
                Kind::Album,
                album.name.clone(),
                Some(Press::Album(album.id.clone())),
            ),
        };

        let card = div()
            .flex()
            .items_center()
            .gap_4()
            .p_3()
            .rounded(theme.radius)
            .bg(theme.secondary)
            .child(
                Artwork::new(cover(hit))
                    .size(theme.metrics.cover * 0.45)
                    .when(matches!(kind, Kind::Artist), Artwork::circle),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .min_w_0()
                    .child(
                        div()
                            .text_size(theme.text(Text::Small))
                            .text_color(theme.muted_foreground)
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(noun(kind).to_uppercase()),
                    )
                    .child(
                        div()
                            .text_size(theme.text(Text::Title))
                            .font_weight(FontWeight::BOLD)
                            .truncate()
                            .child(title),
                    )
                    .child(
                        div()
                            .text_color(theme.muted_foreground)
                            .truncate()
                            .child(meta(hit, false)),
                    ),
            );

        let card = match target {
            Some(target) => self.press("best", target, card, cx),
            None => card.into_any_element(),
        };

        Some(
            div()
                .flex()
                .flex_col()
                .flex_none()
                .gap_2()
                .child(heading(
                    theme.text(Text::Small),
                    theme.muted_foreground,
                    "Best match",
                ))
                .child(card)
                .into_any_element(),
        )
    }

    fn failure(&self, cx: &Context<Self>) -> Option<AnyElement> {
        let reason = self.search.read(cx).error()?.to_owned();

        Some(
            div()
                .flex_none()
                .text_color(cx.theme().danger)
                .child(reason)
                .into_any_element(),
        )
    }

    fn panel(
        &self,
        id: &'static str,
        bar: &Entity<Scrollbar>,
        rows: Vec<AnyElement>,
        cx: &Context<Self>,
    ) -> AnyElement {
        if rows.is_empty() {
            return div()
                .flex_none()
                .text_color(cx.theme().muted_foreground)
                .child("No matches")
                .into_any_element();
        }

        div()
            .relative()
            .flex_1()
            .min_h_0()
            .child(
                div()
                    .id(id)
                    .size_full()
                    .overflow_y_scroll()
                    .track_scroll(bar.read(cx).scroll())
                    .child(div().flex().flex_col().gap_1().pl_3().pr_3().children(rows)),
            )
            .child(bar.clone())
            .into_any_element()
    }

    fn section(
        &self,
        id: &'static str,
        bar: &Entity<Scrollbar>,
        title: &'static str,
        rows: Vec<AnyElement>,
        cx: &Context<Self>,
    ) -> AnyElement {
        let theme = *cx.theme();

        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .gap_1()
            .child(
                div().pl_3().child(heading(
                    theme.text(Text::Small),
                    theme.muted_foreground,
                    title,
                )),
            )
            .child(self.panel(id, bar, rows, cx))
            .into_any_element()
    }

    fn column(&self, kind: Kind, cx: &Context<Self>) -> AnyElement {
        let (id, bar, title) = match kind {
            Kind::Song => ("search-songs", &self.songs, "Songs"),
            Kind::Artist => ("search-artists", &self.artists, "Artists"),
            Kind::Album => ("search-albums", &self.albums, "Albums"),
        };

        let rows = self
            .search
            .read(cx)
            .of(kind)
            .enumerate()
            .map(|(place, hit)| self.row(hit, place, false, cx))
            .collect();

        self.section(id, bar, title, rows, cx)
    }

    fn everything(&self, cx: &Context<Self>) -> AnyElement {
        let mut place = 0;
        let rows = self
            .search
            .read(cx)
            .hits()
            .iter()
            .map(|hit| {
                let at = place;
                if matches!(hit, Hit::Song(_)) {
                    place += 1;
                }
                self.row(hit, at, true, cx)
            })
            .collect();

        self.section("search-all", &self.mixed, "Results", rows, cx)
    }
}

fn cover(hit: &Hit) -> Option<String> {
    match hit {
        Hit::Song(track) => track.cover.clone(),
        Hit::Artist(artist) => artist.cover.clone(),
        Hit::Album(album) => album.cover.clone(),
    }
}

fn meta(hit: &Hit, compact: bool) -> SharedString {
    match hit {
        Hit::Song(track) => tagged(Kind::Song, &track.artists, compact),
        Hit::Album(album) => tagged(Kind::Album, &album.artists, compact),
        Hit::Artist(artist) => match compact {
            true => SharedString::from(noun(Kind::Artist)),
            false => SharedString::from(held(artist.saved)),
        },
    }
}

fn tagged(kind: Kind, value: &str, compact: bool) -> SharedString {
    match compact {
        true => SharedString::from(format!("{} · {value}", noun(kind))),
        false => SharedString::from(value.to_owned()),
    }
}

fn held(saved: usize) -> String {
    match saved {
        0 => noun(Kind::Artist).to_owned(),
        1 => "1 song in Library".to_owned(),
        count => format!("{count} songs in Library"),
    }
}

fn noun(kind: Kind) -> &'static str {
    match kind {
        Kind::Song => "Song",
        Kind::Artist => "Artist",
        Kind::Album => "Album",
    }
}

fn heading(size: Pixels, color: Hsla, label: &'static str) -> AnyElement {
    div()
        .flex_none()
        .pb_1()
        .text_size(size)
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(color)
        .child(label.to_uppercase())
        .into_any_element()
}

fn divider(color: Hsla) -> impl IntoElement {
    div().flex_none().w(px(1.)).bg(color)
}

impl Render for SearchView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.theme();
        let pad = theme.metrics.inset;
        let sidebar = self.sidebar.read(cx).occupied_width();
        let room = cells::content_width(window, sidebar, pad * 2.);
        let stacked = room < COLUMNS;
        let inset = match room > READABLE {
            true => (room - READABLE) / 2.,
            false => Pixels::ZERO,
        };
        let asked = !self.search.read(cx).query().trim().is_empty();

        let results = match stacked {
            true => self.everything(cx),
            false => div()
                .flex()
                .flex_1()
                .min_h_0()
                .child(self.column(Kind::Song, cx))
                .child(divider(theme.border))
                .child(self.column(Kind::Artist, cx))
                .child(divider(theme.border))
                .child(self.column(Kind::Album, cx))
                .into_any_element(),
        };

        div()
            .flex()
            .flex_col()
            .size_full()
            .gap_6()
            .px(pad + inset)
            .pt(pad)
            .pb(pad)
            .child(div().flex().flex_none().child(self.input.clone()))
            .children(self.failure(cx))
            .children(self.best(cx))
            .when(asked, |this| this.child(results))
    }
}
