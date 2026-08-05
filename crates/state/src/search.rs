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
const FLOOR: u32 = 20;
const EVIDENCE: u32 = 10;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Song,
    Artist,
    Album,
}

pub struct ArtistHit {
    pub name: String,
    pub id: Option<String>,
    pub cover: Option<String>,
    pub tracks: usize,
}

pub struct AlbumHit {
    pub id: String,
    pub name: String,
    pub artists: String,
    pub cover: Option<String>,
}

pub enum Hit<'a> {
    Song(&'a Track),
    Artist(&'a ArtistHit),
    Album(&'a AlbumHit),
}

#[derive(Default)]
struct Ranked {
    songs: Vec<Track>,
    artists: Vec<ArtistHit>,
    albums: Vec<AlbumHit>,
    best: Option<Kind>,
}

pub struct Search {
    query: String,
    served: Option<String>,
    found: Vec<Track>,
    ranked: Ranked,
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
                this.found.clear();
                this.ranked = Ranked::default();
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
            found: Vec::new(),
            ranked: Ranked::default(),
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

    pub fn songs(&self) -> &[Track] {
        &self.ranked.songs
    }

    pub fn artists(&self) -> &[ArtistHit] {
        &self.ranked.artists
    }

    pub fn albums(&self) -> &[AlbumHit] {
        &self.ranked.albums
    }

    pub fn best(&self) -> Option<Hit<'_>> {
        match self.ranked.best? {
            Kind::Song => self.ranked.songs.first().map(Hit::Song),
            Kind::Artist => self.ranked.artists.first().map(Hit::Artist),
            Kind::Album => self.ranked.albums.first().map(Hit::Album),
        }
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
            self.found.clear();
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
            let found = join(io.spawn(async move { client.search(&asked).await })).await;

            this.update(cx, |this, cx| {
                if this.query != query {
                    return;
                }
                this.loading = false;
                this.served = Some(query);
                match found {
                    Ok(tracks) => this.found = tracks,
                    Err(error) => {
                        this.found.clear();
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
            self.ranked = Ranked::default();
            cx.notify();
            return;
        }

        self.ranked = {
            let library = self.library.read(cx);
            let (owned, albums) = match library.state() {
                LibraryState::Ready { tracks, albums, .. } => {
                    (tracks.as_slice(), albums.as_slice())
                }
                _ => (&[][..], &[][..]),
            };
            rank(owned, albums, &self.found, query)
        };
        cx.notify();
    }
}

fn rank(owned: &[Track], albums: &[Album], found: &[Track], query: &str) -> Ranked {
    let terms: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();

    let mut songs: Vec<(u32, &Track)> = tagged(owned, found)
        .filter(|(track, remote)| !(*remote && kept(owned, track)))
        .filter_map(|(track, remote)| {
            let fields = [
                (TITLE, track.name.as_str()),
                (ARTIST, track.artists.as_str()),
                (ALBUM, track.album.as_str()),
            ];
            weigh(&fields, &terms)
                .or(remote.then_some(FLOOR))
                .map(|score| (score, track))
        })
        .collect();
    songs.sort_by(|(left, _), (right, _)| right.cmp(left));

    let mut artists = artists(owned, found, albums, &terms);
    artists.sort_by(|(left, one), (right, two)| right.cmp(left).then(two.tracks.cmp(&one.tracks)));

    let mut hits = album_hits(albums, owned, found, &terms);
    hits.sort_by(|(left, _), (right, _)| right.cmp(left));

    let best = best(&songs, &artists, &hits, &terms);

    Ranked {
        songs: songs
            .into_iter()
            .take(LIMIT)
            .map(|(_, track)| track.clone())
            .collect(),
        artists: artists.into_iter().take(LIMIT).map(|(_, it)| it).collect(),
        albums: hits.into_iter().take(LIMIT).map(|(_, it)| it).collect(),
        best,
    }
}

fn kept(owned: &[Track], track: &Track) -> bool {
    track
        .id
        .as_ref()
        .is_some_and(|id| owned.iter().any(|kept| kept.id.as_ref() == Some(id)))
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

fn weigh(fields: &[(u32, &str)], terms: &[String]) -> Option<u32> {
    terms.iter().try_fold(0, |total, term| {
        let best = fields
            .iter()
            .map(|(weight, value)| weight * score(value, term))
            .max()
            .unwrap_or(0);
        (best > 0).then_some(total + best)
    })
}

fn named(value: &str, terms: &[String]) -> Option<u32> {
    weigh(&[(NAME, value)], terms)
}

fn tagged<'a>(owned: &'a [Track], found: &'a [Track]) -> impl Iterator<Item = (&'a Track, bool)> {
    owned
        .iter()
        .map(|track| (track, false))
        .chain(found.iter().map(|track| (track, true)))
}

fn album_hits(
    albums: &[Album],
    owned: &[Track],
    found: &[Track],
    terms: &[String],
) -> Vec<(u32, AlbumHit)> {
    let saved = albums
        .iter()
        .map(|album| (&album.id, &album.name, &album.artists, &album.cover, false));
    let derived = tagged(owned, found).filter_map(|(track, remote)| {
        let id = track.album_id.as_ref()?;
        Some((id, &track.album, &track.artists, &track.cover, remote))
    });

    let mut hits: Vec<(u32, AlbumHit)> = Vec::new();
    for (id, name, artists, cover, remote) in saved.chain(derived) {
        let fields = [(TITLE, name.as_str()), (ARTIST, artists.as_str())];
        let Some(score) = weigh(&fields, terms).or(remote.then_some(FLOOR)) else {
            continue;
        };
        if hits.iter().all(|(_, hit)| hit.id != *id) {
            hits.push((
                score,
                AlbumHit {
                    id: id.clone(),
                    name: name.clone(),
                    artists: artists.clone(),
                    cover: cover.clone(),
                },
            ));
        }
    }

    hits
}

fn artists(
    owned: &[Track],
    found: &[Track],
    albums: &[Album],
    terms: &[String],
) -> Vec<(u32, ArtistHit)> {
    let mut collected: Vec<ArtistHit> = Vec::new();

    let mut record = |name: &str, id: Option<String>, cover: Option<String>, remote: bool| {
        if !remote && named(name, terms).is_none() {
            return;
        }
        match collected.iter_mut().find(|artist| artist.name == name) {
            Some(artist) => {
                artist.tracks += 1;
                artist.id = artist.id.take().or(id);
                artist.cover = artist.cover.take().or(cover);
            }
            None => collected.push(ArtistHit {
                name: name.to_owned(),
                id,
                cover,
                tracks: 1,
            }),
        }
    };

    for (track, remote) in tagged(owned, found) {
        for name in track.artists.split(", ") {
            let primary = track.artists.starts_with(name);
            let id = primary.then(|| track.artist_id.clone()).flatten();
            record(name, id, track.cover.clone(), remote);
        }
    }
    for album in albums {
        for name in album.artists.split(", ") {
            record(name, None, album.cover.clone(), false);
        }
    }

    collected
        .into_iter()
        .map(|artist| {
            let score = named(&artist.name, terms).unwrap_or(FLOOR);
            (score + (artist.tracks as u32).min(EVIDENCE), artist)
        })
        .collect()
}

fn best(
    songs: &[(u32, &Track)],
    artists: &[(u32, ArtistHit)],
    albums: &[(u32, AlbumHit)],
    terms: &[String],
) -> Option<Kind> {
    let song = songs
        .first()
        .map(|(_, track)| (named(&track.name, terms).unwrap_or(0), Kind::Song));
    let artist = artists.first().map(|(score, _)| (*score, Kind::Artist));
    let album = albums
        .first()
        .map(|(_, hit)| (named(&hit.name, terms).unwrap_or(0), Kind::Album));

    [artist, album, song]
        .into_iter()
        .flatten()
        .max_by_key(|(score, _)| *score)
        .map(|(_, kind)| kind)
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
        }
    }

    fn rhapsody() -> Track {
        track("Bohemian Rhapsody", "Queen", "A Night at the Opera")
    }

    fn dora() -> Track {
        track("Дорога", "Дора", "Дорадура")
    }

    fn titles(ranked: &Ranked) -> Vec<&str> {
        ranked
            .songs
            .iter()
            .map(|track| track.name.as_str())
            .collect()
    }

    #[test]
    fn whole_string_query_matches() {
        let ranked = rank(&[rhapsody()], &[], &[], "bohemian rhapsody");
        assert_eq!(titles(&ranked), ["Bohemian Rhapsody"]);
    }

    #[test]
    fn title_and_artist_terms_match_in_any_order() {
        let ranked = rank(&[rhapsody()], &[], &[], "queen bohemian");
        assert_eq!(titles(&ranked), ["Bohemian Rhapsody"]);
    }

    #[test]
    fn every_term_must_match() {
        let ranked = rank(&[rhapsody()], &[], &[], "queen zeppelin");
        assert!(ranked.songs.is_empty());
        assert!(ranked.artists.is_empty());
        assert!(ranked.albums.is_empty());
    }

    #[test]
    fn unmatched_catalog_entries_survive() {
        let ranked = rank(&[], &[], &[dora()], "dora");
        assert_eq!(titles(&ranked), ["Дорога"]);
        assert_eq!(ranked.artists.len(), 1);
        assert_eq!(ranked.albums.len(), 1);
    }

    #[test]
    fn literal_match_outranks_the_floor() {
        let ranked = rank(&[], &[], &[dora(), rhapsody()], "queen");
        assert_eq!(titles(&ranked), ["Bohemian Rhapsody", "Дорога"]);
    }

    #[test]
    fn saved_track_is_listed_once() {
        let saved = rhapsody();
        let ranked = rank(&[saved.clone()], &[], &[saved], "queen");
        assert_eq!(titles(&ranked), ["Bohemian Rhapsody"]);
    }
}
