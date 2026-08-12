use std::collections::HashMap;
use std::sync::Arc;

use gpui::{Context, Entity, Task};
use music::{LyricsHit, LyricsProvider, LyricsQuery, Track};
use tokio::task::JoinSet;

use crate::{Io, Playback, join};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LyricsState {
    Idle,
    Loading,
    Ready,
    Missing,
    Failed(String),
}

pub struct Lyrics {
    state: LyricsState,
    hits: Vec<LyricsHit>,
    chosen: usize,
    following: Option<String>,
    cache: HashMap<String, Vec<LyricsHit>>,
    providers: Vec<Arc<dyn LyricsProvider>>,
    playback: Entity<Playback>,
    io: Io,
    task: Option<Task<()>>,
}

impl Lyrics {
    pub fn new(
        playback: Entity<Playback>,
        providers: Vec<Arc<dyn LyricsProvider>>,
        io: Io,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&playback, |this, _, cx| this.follow(cx))
            .detach();
        Self {
            state: LyricsState::Idle,
            hits: Vec::new(),
            chosen: 0,
            following: None,
            cache: HashMap::new(),
            providers,
            playback,
            io,
            task: None,
        }
    }

    pub fn state(&self) -> &LyricsState {
        &self.state
    }

    pub fn following(&self) -> Option<&str> {
        self.following.as_deref()
    }

    pub fn hits(&self) -> &[LyricsHit] {
        &self.hits
    }

    pub fn current(&self) -> Option<&LyricsHit> {
        self.hits.get(self.chosen)
    }

    pub fn choose(&mut self, index: usize, cx: &mut Context<Self>) {
        if index >= self.hits.len() || index == self.chosen {
            return;
        }
        self.chosen = index;
        cx.notify();
    }

    pub fn active_line(&self, cx: &Context<Self>) -> Option<usize> {
        let music::Lyrics::Synced { lines } = &self.current()?.lyrics else {
            return None;
        };
        music::lyrics::active(lines, self.playback.read(cx).position())
    }

    fn follow(&mut self, cx: &mut Context<Self>) {
        let track = self.playback.read(cx).track().cloned();
        let Some(track) = track else {
            return self.forget(cx);
        };
        let Some(id) = track.id.clone() else {
            return self.forget(cx);
        };
        if self.following.as_deref() == Some(id.as_str()) {
            return;
        }
        self.following = Some(id.clone());
        self.chosen = 0;

        if let Some(hits) = self.cache.get(&id) {
            self.hits = hits.clone();
            self.state = settled(&self.hits);
            cx.notify();
            return;
        }
        self.load(id, track, cx);
    }

    fn forget(&mut self, cx: &mut Context<Self>) {
        if self.following.is_none() {
            return;
        }
        self.task = None;
        self.following = None;
        self.hits.clear();
        self.chosen = 0;
        self.state = LyricsState::Idle;
        cx.notify();
    }

    fn load(&mut self, id: String, track: Track, cx: &mut Context<Self>) {
        if self.providers.is_empty() {
            self.state = LyricsState::Missing;
            cx.notify();
            return;
        }
        self.hits.clear();
        self.state = LyricsState::Loading;
        cx.notify();

        let query = query_for(&track);
        let providers = self.providers.clone();
        let io = self.io.clone();
        self.task = Some(cx.spawn(async move |this, cx| {
            let found = join(io.spawn(async move { gather(providers, query).await })).await;

            this.update(cx, |this, cx| {
                if this.following.as_deref() != Some(id.as_str()) {
                    return;
                }
                this.task = None;
                match found {
                    Ok(hits) => {
                        this.cache.insert(id, hits.clone());
                        this.hits = hits;
                        this.state = settled(&this.hits);
                    }
                    Err(error) => {
                        log::warn!("lyrics: cannot look up {}: {error:#}", track.name);
                        this.state = LyricsState::Failed(format!("{error:#}"));
                    }
                }
                cx.notify();
            })
            .ok();
        }));
    }
}

fn settled(hits: &[LyricsHit]) -> LyricsState {
    match hits.is_empty() {
        true => LyricsState::Missing,
        false => LyricsState::Ready,
    }
}

fn query_for(track: &Track) -> LyricsQuery {
    LyricsQuery {
        title: track.name.clone(),
        artist: track.artists.clone(),
        album: (!track.album.is_empty()).then(|| track.album.clone()),
        duration: track.duration,
    }
}

async fn gather(
    providers: Vec<Arc<dyn LyricsProvider>>,
    query: LyricsQuery,
) -> anyhow::Result<Vec<LyricsHit>> {
    let mut tasks = JoinSet::new();
    for provider in providers {
        let query = query.clone();
        tasks.spawn(async move {
            provider
                .search(&query)
                .await
                .inspect_err(|error| {
                    log::warn!("lyrics: {} did not answer: {error:#}", provider.name())
                })
                .unwrap_or_default()
        });
    }
    let mut hits = Vec::new();
    while let Some(found) = tasks.join_next().await {
        hits.extend(found.unwrap_or_default());
    }
    Ok(music::lyrics::rank(&query, hits))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    fn track(name: &str, album: &str) -> Track {
        Track {
            id: Some("x".to_owned()),
            name: name.to_owned(),
            playable: true,
            artists: "Spiritbox".to_owned(),
            artist_refs: Vec::new(),
            album: album.to_owned(),
            album_id: None,
            cover: None,
            duration: Duration::from_secs(263),
            added_at: None,
            playcount: None,
            popularity: 0,
            explicit: false,
            track_number: 1,
            disc_number: 1,
            tags: Vec::new(),
            languages: Vec::new(),
            credits: Vec::new(),
        }
    }

    #[test]
    fn a_query_carries_the_title_and_length() {
        let query = query_for(&track("Jaded", "Eternal Blue"));

        assert_eq!(query.title, "Jaded");
        assert_eq!(query.artist, "Spiritbox");
        assert_eq!(query.album.as_deref(), Some("Eternal Blue"));
        assert_eq!(query.duration, Duration::from_secs(263));
    }

    #[test]
    fn an_empty_album_is_left_out() {
        assert_eq!(query_for(&track("Jaded", "")).album, None);
    }

    #[test]
    fn nothing_found_reads_as_missing() {
        assert_eq!(settled(&[]), LyricsState::Missing);
    }
}
