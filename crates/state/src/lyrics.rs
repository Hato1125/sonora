use std::collections::HashMap;
use std::sync::Arc;

use gpui::{Context, Entity, Task};
use music::{LyricsHit, LyricsProvider, LyricsQuery, Track, TrackKey};
use tokio::task::JoinSet;

use crate::{Io, Playback, Session, join};

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
    session: Entity<Session>,
    io: Io,
    task: Option<Task<()>>,
}

impl Lyrics {
    pub fn new(
        playback: Entity<Playback>,
        session: Entity<Session>,
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
            session,
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
            self.state = state_for(&self.hits);
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

        let key = self
            .session
            .read(cx)
            .slug_for(&id)
            .map(|provider| TrackKey {
                provider,
                id: id.clone(),
            });
        let query = query_for(&track, key);
        let providers = self.providers.clone();
        let io = self.io.clone();
        self.task = Some(cx.spawn(async move |this, cx| {
            let (sender, mut incoming) = tokio::sync::mpsc::unbounded_channel();
            let query_for_rank = query.clone();
            let worker = io.spawn(async move { gather(providers, query, sender).await });
            let mut hits = Vec::new();
            let mut displayed = None;

            while let Some(mut found) = incoming.recv().await {
                hits.append(&mut found);
                let mut ranked = music::lyrics::rank(&query_for_rank, hits.clone());
                if displayed.is_none()
                    && let Some(karaoke) = ranked.iter().find(|hit| hit.lyrics.worded()).cloned()
                {
                    displayed = Some(karaoke);
                    keep_displayed_first(&mut ranked, displayed.as_ref());
                    this.update(cx, |this, cx| {
                        if this.following.as_deref() != Some(id.as_str()) {
                            return;
                        }
                        this.hits = ranked;
                        this.state = LyricsState::Ready;
                        cx.notify();
                    })
                    .ok();
                }
            }

            let found = join(worker).await;

            this.update(cx, |this, cx| {
                if this.following.as_deref() != Some(id.as_str()) {
                    return;
                }
                this.task = None;
                match found {
                    Ok(()) => {
                        let mut hits = music::lyrics::rank(&query_for_rank, hits);
                        keep_displayed_first(&mut hits, displayed.as_ref());
                        this.cache.insert(id, hits.clone());
                        this.hits = hits;
                        this.state = state_for(&this.hits);
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

fn keep_displayed_first(hits: &mut Vec<LyricsHit>, displayed: Option<&LyricsHit>) {
    let Some(index) = displayed.and_then(|displayed| hits.iter().position(|hit| hit == displayed))
    else {
        return;
    };
    let displayed = hits.remove(index);
    hits.insert(0, displayed);
}

fn state_for(hits: &[LyricsHit]) -> LyricsState {
    match hits.is_empty() {
        true => LyricsState::Missing,
        false => LyricsState::Ready,
    }
}

fn query_for(track: &Track, key: Option<TrackKey>) -> LyricsQuery {
    LyricsQuery {
        title: track.name.clone(),
        artist: track.artists.clone(),
        album: (!track.album.is_empty()).then(|| track.album.clone()),
        duration: track.duration,
        track: key,
    }
}

async fn gather(
    providers: Vec<Arc<dyn LyricsProvider>>,
    query: LyricsQuery,
    sender: tokio::sync::mpsc::UnboundedSender<Vec<LyricsHit>>,
) -> anyhow::Result<()> {
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
    while let Some(found) = tasks.join_next().await {
        sender.send(found.unwrap_or_default()).ok();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use music::{Lyrics as Sheet, LyricsHit, LyricsLine, LyricsWord};

    use super::keep_displayed_first;

    fn hit(source: &'static str, lyrics: Sheet) -> LyricsHit {
        LyricsHit {
            source,
            trust: 0,
            lyrics,
            title: "title".to_owned(),
            artist: "artist".to_owned(),
            album: None,
            duration: None,
            writers: Vec::new(),
        }
    }

    #[test]
    fn a_displayed_karaoke_result_stays_selected_after_final_ranking() {
        let plain = hit("plain", Sheet::plain("line"));
        let displayed = hit(
            "karaoke",
            Sheet::Synced {
                lines: vec![LyricsLine {
                    start: Duration::ZERO,
                    end: Some(Duration::from_secs(1)),
                    text: "line".to_owned(),
                    romanized: None,
                    words: Some(vec![LyricsWord {
                        start: Duration::ZERO,
                        end: Duration::from_secs(1),
                        text: "line".to_owned(),
                    }]),
                    secondary: Vec::new(),
                }]
                .into(),
            },
        );
        let mut final_ranking = vec![plain, displayed.clone()];

        keep_displayed_first(&mut final_ranking, Some(&displayed));

        assert_eq!(final_ranking.first(), Some(&displayed));
    }
}
