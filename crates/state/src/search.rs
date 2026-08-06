use std::time::Duration;

use gpui::{Context, Entity, Task};
use spotify::{Album, Track};

use crate::{Io, Library, LibraryState, Session, SessionEvent, join};

const DEBOUNCE: Duration = Duration::from_millis(250);
const LIMIT: usize = 20;
const EXACT: u32 = 100;
const PREFIX: u32 = 80;
const WORD: u32 = 60;
const INSIDE: u32 = 40;
const TITLE: u32 = 3;
const ARTIST: u32 = 2;
const ALBUM: u32 = 1;
const NAME: u32 = 1;
const TOP: u32 = 95;
const TAIL: u32 = 35;
const DERIVED: u32 = 80;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Song,
    Artist,
    Album,
}

#[derive(Clone)]
pub struct ArtistHit {
    pub name: String,
    pub id: Option<String>,
    pub cover: Option<String>,
    pub saved: usize,
}

#[derive(Clone)]
pub struct AlbumHit {
    pub id: String,
    pub name: String,
    pub artists: String,
    pub cover: Option<String>,
}

#[derive(Clone)]
pub enum Hit {
    Song(Track),
    Artist(ArtistHit),
    Album(AlbumHit),
}

impl Hit {
    fn kind(&self) -> Kind {
        match self {
            Hit::Song(_) => Kind::Song,
            Hit::Artist(_) => Kind::Artist,
            Hit::Album(_) => Kind::Album,
        }
    }
}

struct Query {
    terms: Vec<String>,
    whole: String,
}

impl Query {
    fn new(query: &str) -> Self {
        let terms: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
        Self {
            whole: terms.join(" "),
            terms,
        }
    }
}

struct Scored {
    score: u32,
    popularity: u32,
    hit: Hit,
}

pub struct Search {
    query: String,
    served: Option<String>,
    catalog: Vec<Track>,
    hits: Vec<Hit>,
    loading: bool,
    error: Option<String>,
    session: Entity<Session>,
    library: Entity<Library>,
    io: Io,
    task: Option<Task<()>>,
}

impl Search {
    pub fn new(
        session: Entity<Session>,
        library: Entity<Library>,
        io: Io,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.subscribe(&session, |this, _, event, cx| match event {
            SessionEvent::SignedOut => {
                this.task = None;
                this.query.clear();
                this.served = None;
                this.catalog.clear();
                this.hits.clear();
                this.loading = false;
                cx.notify();
            }
            SessionEvent::SignedIn => {
                let pending = this.query.clone();
                this.query.clear();
                this.ask(&pending, cx);
            }
        })
        .detach();

        cx.observe(&library, |this, _, cx| this.rank(cx)).detach();

        Self {
            query: String::new(),
            served: None,
            catalog: Vec::new(),
            hits: Vec::new(),
            loading: false,
            error: None,
            session,
            library,
            io,
            task: None,
        }
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn hits(&self) -> &[Hit] {
        &self.hits
    }

    pub fn of(&self, kind: Kind) -> impl Iterator<Item = &Hit> {
        self.hits.iter().filter(move |hit| hit.kind() == kind)
    }

    pub fn best(&self) -> Option<&Hit> {
        self.hits.first()
    }

    pub fn queue(&self) -> Vec<Track> {
        self.hits
            .iter()
            .filter_map(|hit| match hit {
                Hit::Song(track) => Some(track.clone()),
                _ => None,
            })
            .collect()
    }

    pub fn is_loading(&self) -> bool {
        self.loading
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn ask(&mut self, query: &str, cx: &mut Context<Self>) {
        let query = query.trim().to_owned();
        let answered = self.loading || self.served.as_deref() == Some(query.as_str());
        if query == self.query && answered {
            return;
        }
        self.query = query.clone();
        self.error = None;

        if query.is_empty() {
            self.task = None;
            self.catalog.clear();
            self.served = Some(String::new());
            self.loading = false;
            self.rank(cx);
            return;
        }

        self.rank(cx);

        let Some(client) = self.session.read(cx).client() else {
            self.loading = false;
            cx.notify();
            return;
        };

        self.loading = true;
        cx.notify();

        let io = self.io.clone();
        self.task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(DEBOUNCE).await;

            let asked = query.clone();
            let catalog = join(io.spawn(async move { client.search(&asked).await })).await;

            this.update(cx, |this, cx| {
                if this.query != query {
                    return;
                }
                this.loading = false;
                this.served = Some(query);
                match catalog {
                    Ok(tracks) => this.catalog = tracks,
                    Err(error) => {
                        this.catalog.clear();
                        this.error = Some(format!("{error:#}"));
                    }
                }
                this.rank(cx);
            })
            .ok();
        }));
    }

    fn rank(&mut self, cx: &mut Context<Self>) {
        let query = self.query.trim();
        if query.is_empty() {
            self.hits.clear();
            cx.notify();
            return;
        }

        self.hits = {
            let held = self.library.read(cx);
            let (tracks, albums) = match held.state() {
                LibraryState::Ready { tracks, albums, .. } => {
                    (tracks.as_slice(), albums.as_slice())
                }
                _ => (&[][..], &[][..]),
            };
            rank(tracks, albums, &self.catalog, query)
        };
        cx.notify();
    }
}

fn rank(library: &[Track], albums: &[Album], catalog: &[Track], asked: &str) -> Vec<Hit> {
    let query = Query::new(asked);
    if query.terms.is_empty() {
        return Vec::new();
    }

    let mut all = songs(library, catalog, &query);
    all.extend(artists(library, catalog, albums, &query));
    all.extend(albums_of(albums, library, catalog, &query));
    order(&mut all);

    all.into_iter().map(|scored| scored.hit).collect()
}

fn songs(library: &[Track], catalog: &[Track], query: &Query) -> Vec<Scored> {
    let mut scored: Vec<Scored> = Vec::new();

    for (track, rank) in sources(library, catalog) {
        if rank.is_some() && kept(library, track) {
            continue;
        }

        let fields = [
            (TITLE, track.name.as_str()),
            (ARTIST, track.artists.as_str()),
            (ALBUM, track.album.as_str()),
        ];
        let Some(score) = fit(&fields, query).max(rank) else {
            continue;
        };

        scored.push(Scored {
            score,
            popularity: track.popularity,
            hit: Hit::Song(track.clone()),
        });
    }

    capped(scored)
}

fn albums_of(albums: &[Album], library: &[Track], catalog: &[Track], query: &Query) -> Vec<Scored> {
    let saved = albums.iter().map(|album| {
        (
            &album.id,
            &album.name,
            &album.artists,
            &album.cover,
            None,
            0,
        )
    });
    let derived = sources(library, catalog).filter_map(|(track, rank)| {
        let id = track.album_id.as_ref()?;
        Some((
            id,
            &track.album,
            &track.artists,
            &track.cover,
            rank,
            track.popularity,
        ))
    });

    let mut scored: Vec<Scored> = Vec::new();
    let mut seen: Vec<&String> = Vec::new();
    for (id, name, artists, cover, rank, popularity) in saved.chain(derived) {
        let fields = [(TITLE, name.as_str()), (ARTIST, artists.as_str())];
        let Some(score) = fit(&fields, query).max(inherited(rank)) else {
            continue;
        };
        if seen.contains(&id) {
            continue;
        }
        seen.push(id);

        scored.push(Scored {
            score,
            popularity,
            hit: Hit::Album(AlbumHit {
                id: id.clone(),
                name: name.clone(),
                artists: artists.clone(),
                cover: cover.clone(),
            }),
        });
    }

    capped(scored)
}

fn artists(library: &[Track], catalog: &[Track], albums: &[Album], query: &Query) -> Vec<Scored> {
    let mut tallies: Vec<(u32, u32, ArtistHit)> = Vec::new();

    let mut record = |name: &str, id, cover, score, popularity, mine| match tallies
        .iter_mut()
        .find(|(_, _, artist)| artist.name == name)
    {
        Some((best, top, artist)) => {
            *best = (*best).max(score);
            *top = (*top).max(popularity);
            artist.saved += mine;
            artist.id = artist.id.take().or(id);
            artist.cover = artist.cover.take().or(cover);
        }
        None => tallies.push((
            score,
            popularity,
            ArtistHit {
                name: name.to_owned(),
                id,
                cover,
                saved: mine,
            },
        )),
    };

    for (track, rank) in sources(library, catalog) {
        for name in track.artists.split(", ") {
            let Some(score) = named(name, query).max(inherited(rank)) else {
                continue;
            };
            let primary = track.artists.starts_with(name);
            let id = primary.then(|| track.artist_id.clone()).flatten();
            let mine = usize::from(rank.is_none());
            record(name, id, track.cover.clone(), score, track.popularity, mine);
        }
    }
    for album in albums {
        for name in album.artists.split(", ") {
            let Some(score) = named(name, query) else {
                continue;
            };
            record(name, None, album.cover.clone(), score, 0, 0);
        }
    }

    capped(
        tallies
            .into_iter()
            .map(|(score, popularity, artist)| Scored {
                score,
                popularity,
                hit: Hit::Artist(artist),
            })
            .collect(),
    )
}

fn sources<'a>(
    library: &'a [Track],
    catalog: &'a [Track],
) -> impl Iterator<Item = (&'a Track, Option<u32>)> {
    let total = catalog.len();

    library.iter().map(|track| (track, None)).chain(
        catalog
            .iter()
            .enumerate()
            .map(move |(at, track)| (track, Some(placed(at, total)))),
    )
}

fn placed(at: usize, total: usize) -> u32 {
    if total <= 1 {
        return TOP;
    }
    let last = total - 1;
    TOP - (TOP - TAIL) * at.min(last) as u32 / last as u32
}

fn inherited(rank: Option<u32>) -> Option<u32> {
    rank.map(|score| score * DERIVED / 100)
}

fn order(scored: &mut [Scored]) {
    scored.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then(right.popularity.cmp(&left.popularity))
    });
}

fn capped(mut scored: Vec<Scored>) -> Vec<Scored> {
    order(&mut scored);
    scored.truncate(LIMIT);
    scored
}

fn kept(library: &[Track], track: &Track) -> bool {
    track
        .id
        .as_ref()
        .is_some_and(|id| library.iter().any(|kept| kept.id.as_ref() == Some(id)))
}

fn score(value: &str, term: &str) -> u32 {
    let value = value.trim().to_lowercase();
    if value == term {
        return EXACT;
    }
    if value.starts_with(term) {
        return PREFIX;
    }
    if value.split_whitespace().any(|word| word.starts_with(term)) {
        return WORD;
    }
    match value.contains(term) {
        true => INSIDE,
        false => 0,
    }
}

fn fit(fields: &[(u32, &str)], query: &Query) -> Option<u32> {
    let ceiling = fields.iter().map(|(weight, _)| *weight).max()? * EXACT;
    let best = |term: &str| {
        fields
            .iter()
            .map(|(weight, value)| weight * score(value, term))
            .max()
            .unwrap_or(0)
    };

    let spread = query.terms.iter().try_fold(0, |total, term| {
        let hit = best(term);
        (hit > 0).then_some(total + hit)
    })?;
    let mean = spread * 100 / (ceiling * query.terms.len() as u32);

    Some(mean.max(best(&query.whole) * 100 / ceiling).min(100))
}

fn named(value: &str, query: &Query) -> Option<u32> {
    fit(&[(NAME, value)], query)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(name: &str, artists: &str, album: &str) -> Track {
        Track {
            id: Some(format!("{name}:{artists}")),
            name: name.to_owned(),
            playable: true,
            artists: artists.to_owned(),
            artist_id: None,
            album: album.to_owned(),
            album_id: Some(album.to_owned()),
            cover: None,
            duration: Duration::ZERO,
            popularity: 0,
            explicit: false,
        }
    }

    fn liked(name: &str, artists: &str, album: &str, popularity: u32) -> Track {
        Track {
            popularity,
            ..track(name, artists, album)
        }
    }

    fn rhapsody() -> Track {
        track("Bohemian Rhapsody", "Queen", "A Night at the Opera")
    }

    fn dora() -> Track {
        track("Дорога", "Дора", "Дорадура")
    }

    fn titles(hits: &[Hit]) -> Vec<&str> {
        hits.iter()
            .filter_map(|hit| match hit {
                Hit::Song(track) => Some(track.name.as_str()),
                _ => None,
            })
            .collect()
    }

    fn count(hits: &[Hit], kind: Kind) -> usize {
        hits.iter().filter(|hit| hit.kind() == kind).count()
    }

    #[test]
    fn whole_string_query_matches() {
        let hits = rank(&[rhapsody()], &[], &[], "bohemian rhapsody");
        assert_eq!(titles(&hits), ["Bohemian Rhapsody"]);
    }

    #[test]
    fn title_and_artist_terms_match_in_any_order() {
        let hits = rank(&[rhapsody()], &[], &[], "queen bohemian");
        assert_eq!(titles(&hits), ["Bohemian Rhapsody"]);
    }

    #[test]
    fn every_term_must_match() {
        let hits = rank(&[rhapsody()], &[], &[], "queen zeppelin");
        assert!(hits.is_empty());
    }

    #[test]
    fn unmatched_catalog_entries_survive() {
        let hits = rank(&[], &[], &[dora()], "dora");
        assert_eq!(titles(&hits), ["Дорога"]);
        assert_eq!(count(&hits, Kind::Artist), 1);
        assert_eq!(count(&hits, Kind::Album), 1);
    }

    #[test]
    fn saved_track_is_listed_once() {
        let saved = rhapsody();
        let hits = rank(&[saved.clone()], &[], &[saved], "queen");
        assert_eq!(titles(&hits), ["Bohemian Rhapsody"]);
    }

    #[test]
    fn catalog_keeps_the_order_it_answered_with() {
        let catalog = [
            track("First", "Nobody", "One"),
            track("Second", "Nobody", "Two"),
            track("Third", "Nobody", "Three"),
        ];
        let hits = rank(&[], &[], &catalog, "lyric phrase");
        assert_eq!(titles(&hits), ["First", "Second", "Third"]);
    }

    #[test]
    fn library_exact_title_outranks_the_catalog_top() {
        let hits = rank(&[rhapsody()], &[], &[dora()], "bohemian rhapsody");
        assert_eq!(titles(&hits), ["Bohemian Rhapsody", "Дорога"]);
    }

    #[test]
    fn popularity_breaks_ties() {
        let library = [
            liked("Fever", "Bullet For My Valentine", "Fever", 10),
            liked("Fever", "Peggy Lee", "Black Coffee", 80),
        ];
        let hits = rank(&library, &[], &[], "fever");
        let Some(Hit::Song(first)) = hits.first() else {
            panic!("expected a song first");
        };
        assert_eq!(first.artists, "Peggy Lee");
    }

    #[test]
    fn library_songs_are_counted_for_artists() {
        let hits = rank(&[rhapsody()], &[], &[rhapsody()], "queen");
        let Some(Hit::Artist(artist)) = hits.iter().find(|hit| hit.kind() == Kind::Artist) else {
            panic!("expected an artist hit");
        };
        assert_eq!(artist.saved, 1);
    }
}
