use gpui::prelude::*;
use gpui::{
    AnyElement, App, Context, Div, ElementId, Entity, FontWeight, Hsla, Pixels, Render,
    ScrollHandle, SharedString, Window, div, px, svg,
};
use input::Input;
use router::{Destination, navigate};
use state::{Hit, Kind, Playback, Search};
use ui::ActiveTheme as _;
use ui::{Artwork, Scrollbar, Theme, clock};
use workspace::Sidebar;

use crate::cells;

const PADDING: Pixels = px(24.);
const READABLE: Pixels = px(1180.);
const COLUMNS: Pixels = px(720.);
const ROW: Pixels = px(52.);
const COVER: Pixels = px(36.);
const BEST_COVER: Pixels = px(84.);
const AVATAR: Pixels = px(18.);

enum Press {
    Song(usize),
    Album(String),
}

pub(crate) struct SearchView {
    input: Entity<Input>,
    search: Entity<Search>,
    playback: Entity<Playback>,
    sidebar: Entity<Sidebar>,
    scroll: ScrollHandle,
    scrollbar: Entity<Scrollbar>,
}

impl SearchView {
    pub(crate) fn new(
        search: Entity<Search>,
        playback: Entity<Playback>,
        sidebar: Entity<Sidebar>,
        cx: &mut Context<Self>,
    ) -> Self {
        let input = cx.new(|cx| Input::new("What do you want to listen to?", cx));

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

        let scroll = ScrollHandle::new();
        let scrollbar = cx.new(|_| Scrollbar::new(scroll.clone()));

        Self {
            input,
            search,
            playback,
            sidebar,
            scroll,
            scrollbar,
        }
    }

    pub(crate) fn focus(&self, window: &mut Window, cx: &mut App) {
        self.input.update(cx, |input, cx| input.focus(window, cx));
    }

    fn play(&mut self, index: usize, cx: &mut Context<Self>) {
        let queued = self.search.read(cx).songs().to_vec();
        if index >= queued.len() {
            return;
        }
        self.playback
            .update(cx, |playback, cx| playback.start(queued, index, cx));
    }

    fn open_album(&mut self, id: String, cx: &mut Context<Self>) {
        navigate(Destination::Album(id.into()), cx);
    }

    fn bar(&self, cx: &Context<Self>) -> AnyElement {
        let theme = cx.theme();

        div()
            .flex()
            .flex_none()
            .items_center()
            .gap_2()
            .h(px(40.))
            .px_3()
            .rounded(theme.radius)
            .bg(theme.secondary)
            .border_1()
            .border_color(theme.border)
            .child(
                svg()
                    .path("icons/search.svg")
                    .size_4()
                    .flex_none()
                    .text_color(theme.muted_foreground),
            )
            .child(self.input.clone())
            .into_any_element()
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

    fn best(&self, cx: &Context<Self>) -> Option<AnyElement> {
        let theme = *cx.theme();
        let (cover, kind, title, meta, press) = match self.search.read(cx).best()? {
            Hit::Song(track) => (
                track.cover.clone(),
                Kind::Song,
                SharedString::from(track.name.clone()),
                SharedString::from(track.artists.clone()),
                Some(Press::Song(0)),
            ),
            Hit::Artist(artist) => (
                artist.cover.clone(),
                Kind::Artist,
                SharedString::from(artist.name.clone()),
                SharedString::from(songs_label(artist.tracks)),
                None,
            ),
            Hit::Album(album) => (
                album.cover.clone(),
                Kind::Album,
                SharedString::from(album.name.clone()),
                SharedString::from(album.artists.clone()),
                Some(Press::Album(album.id.clone())),
            ),
        };

        let card = div()
            .flex()
            .items_center()
            .gap_4()
            .p_4()
            .rounded(theme.radius)
            .bg(theme.secondary)
            .child(Artwork::new(cover).size(BEST_COVER).rounded(cells::ROUNDED))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .min_w_0()
                    .gap_1()
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(theme.muted_foreground)
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(label(kind)),
                    )
                    .child(
                        div()
                            .text_size(px(22.))
                            .font_weight(FontWeight::BOLD)
                            .truncate()
                            .child(title),
                    )
                    .child(
                        div()
                            .text_color(theme.muted_foreground)
                            .truncate()
                            .child(meta),
                    ),
            );

        Some(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(heading("Best match", theme.muted_foreground))
                .child(self.pressable("best", card, press, cx))
                .into_any_element(),
        )
    }

    fn pressable(
        &self,
        id: impl Into<ElementId>,
        row: Div,
        press: Option<Press>,
        cx: &Context<Self>,
    ) -> AnyElement {
        let Some(press) = press else {
            return row.into_any_element();
        };

        row.id(id)
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, _, cx| match &press {
                Press::Song(index) => this.play(*index, cx),
                Press::Album(id) => this.open_album(id.clone(), cx),
            }))
            .into_any_element()
    }

    fn column(&self, kind: Kind, cx: &Context<Self>) -> AnyElement {
        let theme = *cx.theme();
        let search = self.search.read(cx);
        let playing = self
            .playback
            .read(cx)
            .track()
            .and_then(|current| current.id.clone());
        let rows: Vec<AnyElement> = match kind {
            Kind::Song => search
                .songs()
                .iter()
                .enumerate()
                .map(|(index, track)| {
                    let tint = match track.id.is_some() && track.id == playing {
                        true => theme.primary,
                        false => theme.foreground,
                    };
                    self.pressable(
                        ("song", index),
                        row(
                            theme,
                            track.cover.clone(),
                            cells::ROUNDED,
                            track.name.clone().into(),
                            tint,
                            track.artists.clone().into(),
                        )
                        .child(
                            div()
                                .flex_none()
                                .text_size(px(11.))
                                .text_color(theme.muted_foreground)
                                .child(clock(track.duration)),
                        ),
                        Some(Press::Song(index)),
                        cx,
                    )
                })
                .collect(),
            Kind::Artist => search
                .artists()
                .iter()
                .map(|artist| {
                    row(
                        theme,
                        artist.cover.clone(),
                        AVATAR,
                        artist.name.clone().into(),
                        theme.foreground,
                        songs_label(artist.tracks).into(),
                    )
                    .into_any_element()
                })
                .collect(),
            Kind::Album => search
                .albums()
                .iter()
                .enumerate()
                .map(|(index, album)| {
                    self.pressable(
                        ("album", index),
                        row(
                            theme,
                            album.cover.clone(),
                            cells::ROUNDED,
                            album.name.clone().into(),
                            theme.foreground,
                            album.artists.clone().into(),
                        ),
                        Some(Press::Album(album.id.clone())),
                        cx,
                    )
                })
                .collect(),
        };

        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .gap_1()
            .child(heading(heading_for(kind), theme.muted_foreground))
            .when(rows.is_empty(), |this| {
                this.child(
                    div()
                        .h(ROW)
                        .flex()
                        .items_center()
                        .text_color(theme.muted_foreground)
                        .child("No matches"),
                )
            })
            .children(rows)
            .into_any_element()
    }
}

fn row(
    theme: Theme,
    cover: Option<String>,
    rounded: Pixels,
    title: SharedString,
    tint: Hsla,
    meta: SharedString,
) -> Div {
    div()
        .flex()
        .items_center()
        .gap_3()
        .h(ROW)
        .px_2()
        .rounded(theme.radius)
        .hover(move |style| style.bg(theme.table_hover))
        .child(Artwork::new(cover).size(COVER).rounded(rounded))
        .child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_w_0()
                .child(div().truncate().text_color(tint).child(title))
                .child(
                    div()
                        .truncate()
                        .text_size(px(11.))
                        .text_color(theme.muted_foreground)
                        .child(meta),
                ),
        )
}

fn heading(label: impl Into<SharedString>, color: Hsla) -> AnyElement {
    div()
        .flex_none()
        .pb_1()
        .text_size(px(11.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(color)
        .child(label.into().to_uppercase())
        .into_any_element()
}

fn heading_for(kind: Kind) -> &'static str {
    match kind {
        Kind::Song => "Songs",
        Kind::Artist => "Artists",
        Kind::Album => "Albums",
    }
}

fn label(kind: Kind) -> &'static str {
    match kind {
        Kind::Song => "SONG",
        Kind::Artist => "ARTIST",
        Kind::Album => "ALBUM",
    }
}

fn songs_label(count: usize) -> String {
    match count {
        1 => "1 song".to_owned(),
        count => format!("{count} songs"),
    }
}

impl Render for SearchView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let sidebar = self.sidebar.read(cx).occupied_width();
        let room = cells::content_width(window, sidebar, PADDING * 2.);
        let stacked = room < COLUMNS;
        let inset = match room > READABLE {
            true => (room - READABLE) / 2.,
            false => Pixels::ZERO,
        };
        let border = cx.theme().border;
        let asked = !self.search.read(cx).query().trim().is_empty();

        let columns = match stacked {
            true => div()
                .flex()
                .flex_col()
                .gap_6()
                .child(self.column(Kind::Song, cx))
                .child(self.column(Kind::Artist, cx))
                .child(self.column(Kind::Album, cx)),
            false => div()
                .flex()
                .gap_6()
                .child(self.column(Kind::Song, cx))
                .child(divider(border))
                .child(self.column(Kind::Artist, cx))
                .child(divider(border))
                .child(self.column(Kind::Album, cx)),
        };

        div()
            .relative()
            .size_full()
            .child(
                div()
                    .id("search-page")
                    .size_full()
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .px(PADDING + inset)
                    .pt(PADDING)
                    .pb(PADDING)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_6()
                            .child(self.bar(cx))
                            .children(self.failure(cx))
                            .children(self.best(cx))
                            .when(asked, |this| this.child(columns)),
                    ),
            )
            .child(self.scrollbar.clone())
    }
}

fn divider(color: Hsla) -> Div {
    div().flex_none().w(px(1.)).bg(color)
}
