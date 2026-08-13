use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;
use gpui::{Context, Entity, EventEmitter, Task};
use music::{
    MusicApi, PlaybackConfig, PlaybackEvent as BackendEvent, PlaybackEvents, PlaybackFactory,
    Player, Track,
};

type Fetch = Pin<Box<dyn Future<Output = Result<Vec<Track>>> + Send>>;

#[derive(Clone, Copy, PartialEq)]
enum Start {
    Pick,
    Skip,
    Burst,
    Segue,
}

impl Start {
    fn debounce(self) -> Duration {
        match self {
            Self::Burst => SKIP_DEBOUNCE,
            Self::Pick | Self::Skip | Self::Segue => Duration::ZERO,
        }
    }
}

#[derive(Clone, Copy)]
enum QueuePlacement {
    Next,
    End,
}

use crate::queue::Queue;
use serde::{Deserialize, Serialize};

use crate::{AppSettings, Io, Note, Session, SessionEvent, Toasts, join};

const POSITION_INTERVAL: Duration = Duration::from_millis(500);
const PRELOAD_BEFORE_END: Duration = Duration::from_secs(10);
const SKIP_DEBOUNCE: Duration = Duration::from_millis(250);
const KEY_COOLDOWN: Duration = Duration::from_secs(6);
const RESUME_STEP: Duration = Duration::from_secs(5);
const TAPER_DB: f32 = 50.;
const SIMILAR_LIMIT: usize = 20;

fn gain(level: f32) -> f32 {
    match level.clamp(0., 1.) {
        level if level <= 0. => 0.,
        level => 10f32.powf(TAPER_DB * (level - 1.) / 20.),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlaybackState {
    Idle,
    Playing,
    Paused,
    Loading,
    Failed(String),
}

pub enum PlaybackEvent {
    StartedPlayback,
    EndedPlayback,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Repeat {
    #[default]
    Off,
    All,
    One,
}

#[derive(Clone, PartialEq, Eq)]
pub enum Origin {
    Album(String),
    Playlist(String),
    Artist(String),
    Radio(String),
}

impl Origin {
    fn id(&self) -> &str {
        match self {
            Origin::Album(id) | Origin::Playlist(id) | Origin::Artist(id) | Origin::Radio(id) => id,
        }
    }
}

pub struct Playback {
    state: PlaybackState,
    origin: Option<Origin>,
    position: Duration,
    track: Option<Track>,
    engine: Option<Box<dyn Player>>,
    local_engine: Option<Box<dyn Player>>,
    session: Entity<Session>,
    queue: Entity<Queue>,
    settings: Entity<AppSettings>,
    level: f32,
    normalisation: bool,
    gapless: bool,
    repeat: Repeat,
    radio: bool,
    seeded: Option<String>,
    task: Option<Task<()>>,
    local_task: Option<Task<()>>,
    load: Option<Task<()>>,
    fetch: Option<Task<()>>,
    enqueue: Option<Task<()>>,
    suggest: Option<Task<()>>,
    preloaded: Option<String>,
    skipped: Option<Instant>,
    blocked_until: Option<Instant>,
    refused: bool,
    armed: Option<Duration>,
    settle: Option<Duration>,
    stored: Duration,
}

impl EventEmitter<PlaybackEvent> for Playback {}

impl Playback {
    pub fn new(
        session: Entity<Session>,
        queue: Entity<Queue>,
        settings: Entity<AppSettings>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.subscribe(&session, |this, session, event, cx| match event {
            SessionEvent::SignedIn => {
                let Some(playback) = session.read(cx).playback() else {
                    return;
                };
                this.start_engine(playback, cx);
                this.adopt(cx);
            }
            SessionEvent::SignedOut => this.teardown(cx),
            SessionEvent::LocalChanged => {
                if this.local_engine.is_none()
                    && let Some(playback) = session.read(cx).local_playback()
                {
                    this.start_local_engine(playback, cx);
                }
            }
        })
        .detach();
        cx.observe(&queue, |this, _, cx| this.suggest_similar(cx))
            .detach();

        let level = settings.read(cx).volume();
        let normalisation = settings.read(cx).normalisation();
        let gapless = settings.read(cx).gapless();
        let repeat = settings.read(cx).repeat();

        Self {
            state: PlaybackState::Idle,
            origin: None,
            position: Duration::ZERO,
            track: None,
            engine: None,
            local_engine: None,
            session,
            queue,
            settings,
            level,
            normalisation,
            gapless,
            repeat,
            radio: false,
            seeded: None,
            task: None,
            local_task: None,
            load: None,
            fetch: None,
            enqueue: None,
            suggest: None,
            preloaded: None,
            skipped: None,
            blocked_until: None,
            refused: false,
            armed: None,
            settle: None,
            stored: Duration::ZERO,
        }
    }

    pub fn play(&mut self, track: &Track, cx: &mut Context<Self>) {
        self.load_after(track, Start::Pick, cx);
    }

    fn engine_for(&self, id: &str) -> Option<&dyn Player> {
        match music::is_local_id(id) {
            true => self.local_engine.as_deref(),
            false => self.engine.as_deref(),
        }
    }

    fn active_engine(&self) -> Option<&dyn Player> {
        let id = self.track.as_ref()?.id.as_deref()?;
        self.engine_for(id)
    }

    fn silence_other(&self, id: &str) {
        let other = match music::is_local_id(id) {
            true => self.engine.as_deref(),
            false => self.local_engine.as_deref(),
        };
        if let Some(engine) = other {
            engine.pause();
        }
    }

    pub fn preload(&mut self, track: &Track) {
        let Some(id) = track.id.as_deref() else {
            return;
        };
        if self.engine_for(id).is_none() {
            return;
        }
        if !track.playable || self.track.as_ref().and_then(|track| track.id.as_deref()) == Some(id)
        {
            return;
        }
        if self.preloaded.as_deref() == Some(id) {
            return;
        }
        self.preloaded = Some(id.to_owned());
        let Some(engine) = self.engine_for(id) else {
            return;
        };
        if let Err(error) = engine.preload(id) {
            self.preloaded = None;
            log::warn!("playback: cannot preload {}: {error:#}", track.name);
        }
    }

    fn load_after(&mut self, track: &Track, start: Start, cx: &mut Context<Self>) {
        if self.refused {
            return self.refuse(cx);
        }
        let Some(id) = track.id.clone() else {
            return self.failed(format!("{} has no track id", track.name), cx);
        };
        if !track.playable {
            return self.failed(format!("{} is not available to stream", track.name), cx);
        }
        if self.engine_for(&id).is_none() {
            return;
        }
        self.silence_other(&id);

        self.track = Some(track.clone());
        self.state = PlaybackState::Loading;
        self.position = Duration::ZERO;
        self.preloaded = None;
        self.armed = None;
        self.settle = None;
        cx.notify();

        let wait = self
            .blocked_until
            .and_then(|until| until.checked_duration_since(Instant::now()))
            .unwrap_or_default()
            .max(start.debounce());

        self.load = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(wait).await;
            this.update(cx, |this, cx| {
                let Some(engine) = this.engine_for(&id) else {
                    return;
                };
                if let Err(error) = engine.load(&id, start == Start::Segue) {
                    this.failed(format!("{error:#}"), cx);
                }
            })
            .ok();
        }));
    }

    pub fn start(&mut self, tracks: Vec<Track>, index: usize, cx: &mut Context<Self>) {
        self.fetch = None;
        self.begin(tracks, index, None, cx);
    }

    pub fn play_radio(&mut self, seed: &Track, cx: &mut Context<Self>) {
        let Some(id) = seed.id.clone() else {
            return self.failed(format!("{} has no track id", seed.name), cx);
        };
        if !seed.playable {
            return self.failed(format!("{} is not available to stream", seed.name), cx);
        }

        let origin = Origin::Radio(id.clone());
        let seed = seed.clone();
        self.gather(origin, cx, move |client| {
            Box::pin(async move {
                let mut tracks = client.track_radio(&id).await?;
                tracks.retain(|track| track.id != seed.id && track.playable);
                fastrand::shuffle(&mut tracks);
                tracks.insert(0, seed);
                Ok(tracks)
            })
        });
    }

    pub fn play_track(&mut self, track_id: &str, cx: &mut Context<Self>) {
        let origin = Origin::Radio(track_id.to_owned());
        let id = track_id.to_owned();
        self.gather(origin, cx, move |client| {
            Box::pin(async move {
                let mut tracks = client.track_radio(&id).await?;
                tracks.retain(|track| track.playable);
                Ok(tracks)
            })
        });
    }

    pub fn enqueue(&mut self, track: Track, cx: &mut Context<Self>) {
        if self.queue.read(cx).current().is_none() {
            self.begin(vec![track], 0, None, cx);
            return;
        }
        let name = track.name.clone();
        self.queue.update(cx, |queue, cx| queue.append(track, cx));
        Toasts::about(Note::Done, "toast-queued-track", name, cx);
    }

    pub fn play_next(&mut self, track: Track, cx: &mut Context<Self>) {
        if self.queue.read(cx).current().is_none() {
            self.begin(vec![track], 0, None, cx);
            return;
        }
        let name = track.name.clone();
        self.queue.update(cx, |queue, cx| queue.prepend(track, cx));
        Toasts::about(Note::Done, "toast-next-track", name, cx);
    }

    pub fn enqueue_all(&mut self, tracks: Vec<Track>, cx: &mut Context<Self>) {
        if tracks.is_empty() {
            return;
        }
        if self.queue.read(cx).current().is_none() {
            self.begin(tracks, 0, None, cx);
            return;
        }
        self.queue
            .update(cx, |queue, cx| queue.append_all(tracks, cx));
    }

    pub fn play_next_all(&mut self, tracks: Vec<Track>, cx: &mut Context<Self>) {
        if tracks.is_empty() {
            return;
        }
        if self.queue.read(cx).current().is_none() {
            self.begin(tracks, 0, None, cx);
            return;
        }
        self.queue
            .update(cx, |queue, cx| queue.prepend_all(tracks, cx));
    }

    pub fn enqueue_album(&mut self, album: &str, cx: &mut Context<Self>) {
        let id = album.to_owned();
        let album = album.to_owned();
        self.enqueue_from("album", &id, QueuePlacement::End, cx, move |client| {
            Box::pin(async move { client.album_tracks(&album).await })
        });
    }

    pub fn play_album_next(&mut self, album: &str, cx: &mut Context<Self>) {
        let id = album.to_owned();
        let album = album.to_owned();
        self.enqueue_from("album", &id, QueuePlacement::Next, cx, move |client| {
            Box::pin(async move { client.album_tracks(&album).await })
        });
    }

    pub fn play_album(&mut self, album: &str, cx: &mut Context<Self>) {
        let origin = Origin::Album(album.to_owned());
        let album = album.to_owned();
        self.gather(origin, cx, move |client| {
            Box::pin(async move { client.album_tracks(&album).await })
        });
    }

    pub fn play_artist(&mut self, artist: &str, cx: &mut Context<Self>) {
        let origin = Origin::Artist(artist.to_owned());
        let artist = artist.to_owned();
        self.gather(origin, cx, move |client| {
            Box::pin(async move { client.artist(&artist).await.map(|found| found.top_tracks) })
        });
    }

    pub fn enqueue_playlist(&mut self, playlist: &str, cx: &mut Context<Self>) {
        let id = playlist.to_owned();
        let playlist = playlist.to_owned();
        self.enqueue_from("playlist", &id, QueuePlacement::End, cx, move |client| {
            Box::pin(async move { client.playlist_tracks(&playlist).await })
        });
    }

    pub fn play_playlist_next(&mut self, playlist: &str, cx: &mut Context<Self>) {
        let id = playlist.to_owned();
        let playlist = playlist.to_owned();
        self.enqueue_from("playlist", &id, QueuePlacement::Next, cx, move |client| {
            Box::pin(async move { client.playlist_tracks(&playlist).await })
        });
    }

    pub fn play_playlist(&mut self, playlist: &str, cx: &mut Context<Self>) {
        let origin = Origin::Playlist(playlist.to_owned());
        let playlist = playlist.to_owned();
        self.gather(origin, cx, move |client| {
            Box::pin(async move { client.playlist_tracks(&playlist).await })
        });
    }

    fn client_for(&self, id: &str, cx: &Context<Self>) -> Option<Arc<dyn MusicApi>> {
        let session = self.session.read(cx);
        match music::is_local_id(id) {
            true => session.local_client(),
            false => session.client(),
        }
    }

    fn enqueue_from<F>(
        &mut self,
        source: &'static str,
        id: &str,
        placement: QueuePlacement,
        cx: &mut Context<Self>,
        tracks: F,
    ) where
        F: FnOnce(Arc<dyn MusicApi>) -> Fetch + Send + 'static,
    {
        if self.enqueue.is_some() {
            return;
        }
        let Some(client) = self.client_for(id, cx) else {
            return;
        };
        let io = Io::global(cx);
        self.enqueue = Some(cx.spawn(async move |this, cx| {
            let loaded = join(io.spawn(async move { tracks(client).await })).await;
            this.update(cx, |this, cx| {
                this.enqueue = None;
                match loaded {
                    Ok(tracks) => {
                        let queued = this.queue.read(cx).current().is_some();
                        match placement {
                            QueuePlacement::Next => this.play_next_all(tracks, cx),
                            QueuePlacement::End => this.enqueue_all(tracks, cx),
                        }
                        if queued {
                            let key = match placement {
                                QueuePlacement::Next => format!("toast-next-{source}"),
                                QueuePlacement::End => format!("toast-queued-{source}"),
                            };
                            Toasts::show(Note::Done, key, cx);
                        }
                    }
                    Err(error) => {
                        log::error!("playback: cannot enqueue {source}: {error:#}");
                        Toasts::show(Note::Failed, "toast-queue-failed", cx);
                    }
                }
            })
            .ok();
        }));
    }

    pub fn origin(&self) -> Option<&Origin> {
        self.origin.as_ref()
    }

    pub fn playing_from(&self, origin: &Origin) -> Option<PlaybackState> {
        (self.origin.as_ref() == Some(origin)).then(|| self.state.clone())
    }

    fn begin(
        &mut self,
        tracks: Vec<Track>,
        index: usize,
        origin: Option<Origin>,
        cx: &mut Context<Self>,
    ) {
        let Some(track) = self
            .queue
            .update(cx, |queue, cx| queue.start(tracks, index, cx))
        else {
            return;
        };
        self.origin = origin;
        self.play(&track, cx);
    }

    fn gather<F>(&mut self, origin: Origin, cx: &mut Context<Self>, tracks: F)
    where
        F: FnOnce(Arc<dyn MusicApi>) -> Fetch + Send + 'static,
    {
        let Some(client) = self.client_for(origin.id(), cx) else {
            return;
        };

        let io = Io::global(cx);
        if !self.has_active_playback() {
            self.state = PlaybackState::Loading;
            cx.notify();
        }

        self.fetch = Some(cx.spawn(async move |this, cx| {
            let loaded = join(io.spawn(async move { tracks(client).await })).await;

            this.update(cx, |this, cx| match loaded {
                Ok(tracks) => this.begin(tracks, 0, Some(origin), cx),
                Err(error) if this.has_active_playback() => {
                    log::error!("playback: cannot load context: {error:#}");
                }
                Err(error) => this.failed(format!("{error:#}"), cx),
            })
            .ok();
        }));
    }

    pub fn next(&mut self, cx: &mut Context<Self>) {
        self.fetch = None;
        let start = self.burst();
        self.follow_queue(start, cx);
    }

    fn burst(&mut self) -> Start {
        let now = Instant::now();
        let rapid = self
            .skipped
            .replace(now)
            .is_some_and(|last| now.duration_since(last) < SKIP_DEBOUNCE);
        match rapid {
            true => Start::Burst,
            false => Start::Skip,
        }
    }

    fn preload_next(&mut self, position: Duration, cx: &Context<Self>) {
        let Some(duration) = self.track.as_ref().map(|track| track.duration) else {
            return;
        };
        if duration.is_zero()
            || self.state != PlaybackState::Playing
            || duration.saturating_sub(position) > PRELOAD_BEFORE_END
        {
            return;
        }

        let next = match self.repeat != Repeat::One {
            true => self.queue.read(cx).upcoming().next().cloned(),
            false => None,
        };
        let Some(next) = next else {
            self.preloaded = None;
            return;
        };

        self.preload(&next);
    }

    pub fn radio(&self) -> bool {
        self.radio
    }

    pub fn toggle_radio(&mut self, cx: &mut Context<Self>) {
        self.radio = !self.radio;
        match self.radio {
            true => self.suggest_similar(cx),
            false => self.forget_similar(cx),
        }
    }

    pub fn play_similar(&mut self, index: usize, cx: &mut Context<Self>) {
        self.fetch = None;
        let Some(track) = self
            .queue
            .update(cx, |queue, cx| queue.play_similar(index, cx))
        else {
            return;
        };
        self.load_after(&track, Start::Pick, cx);
    }

    fn forget_similar(&mut self, cx: &mut Context<Self>) {
        self.seeded = None;
        self.suggest = None;
        self.queue.update(cx, |queue, cx| queue.clear_similar(cx));
        cx.notify();
    }

    fn seed(&self, cx: &Context<Self>) -> Option<Track> {
        let queue = self.queue.read(cx);
        queue.upcoming().last().or_else(|| queue.current()).cloned()
    }

    fn suggest_similar(&mut self, cx: &mut Context<Self>) {
        if !self.radio || self.queue.read(cx).similar().len() > 0 {
            return;
        }
        let Some(id) = self.seed(cx).and_then(|seed| seed.id) else {
            return self.forget_similar(cx);
        };
        if self.seeded.as_deref() == Some(id.as_str()) {
            return;
        }
        let Some(client) = self.client_for(&id, cx) else {
            return self.forget_similar(cx);
        };

        let queued = self.queue.read(cx).ids();
        self.seeded = Some(id.clone());
        let io = Io::global(cx);
        self.suggest = Some(cx.spawn(async move |this, cx| {
            let loaded = join(io.spawn(async move {
                let mut tracks = client.track_radio(&id).await?;
                tracks.retain(|track| {
                    track.playable
                        && track
                            .id
                            .as_ref()
                            .is_some_and(|id| !queued.contains(id.as_str()))
                });
                fastrand::shuffle(&mut tracks);
                tracks.truncate(SIMILAR_LIMIT);
                anyhow::Ok(tracks)
            }))
            .await;

            this.update(cx, |this, cx| match loaded {
                Ok(_) if !this.radio => {}
                Ok(tracks) => this.queue.update(cx, |queue, cx| queue.suggest(tracks, cx)),
                Err(error) => log::warn!("playback: cannot load similar tracks: {error:#}"),
            })
            .ok();
        }));
    }

    pub fn repeat(&self) -> Repeat {
        self.repeat
    }

    pub fn cycle_repeat(&mut self, cx: &mut Context<Self>) {
        self.repeat = match self.repeat {
            Repeat::Off => Repeat::All,
            Repeat::All => Repeat::One,
            Repeat::One => Repeat::Off,
        };
        let repeat = self.repeat;
        self.settings
            .update(cx, |settings, cx| settings.set_repeat(repeat, cx));
        cx.notify();
    }

    fn advance(&mut self, ended: Option<Track>, cx: &mut Context<Self>) {
        match self.repeat {
            Repeat::One => match ended {
                Some(track) => self.load_after(&track, Start::Segue, cx),
                None => self.segue_queue(cx),
            },
            Repeat::All if !self.queue.read(cx).has_next() => {
                self.fetch = None;
                if let Some(track) = self.queue.update(cx, |queue, cx| queue.rewind(cx)) {
                    self.load_after(&track, Start::Segue, cx);
                }
            }
            _ if self.radio && !self.queue.read(cx).has_next() => {
                match ended.or_else(|| self.track.clone()) {
                    Some(seed) => self.extend_radio(&seed, cx),
                    None => self.segue_queue(cx),
                }
            }
            _ => self.segue_queue(cx),
        }
    }

    fn segue_queue(&mut self, cx: &mut Context<Self>) {
        self.fetch = None;
        self.follow_queue(Start::Segue, cx);
    }

    fn extend_radio(&mut self, seed: &Track, cx: &mut Context<Self>) {
        let Some(id) = seed.id.clone() else {
            return self.next(cx);
        };
        let Some(client) = self.client_for(&id, cx) else {
            return self.next(cx);
        };

        let io = Io::global(cx);
        let heard = seed.id.clone();
        self.fetch = Some(cx.spawn(async move |this, cx| {
            let loaded = join(io.spawn(async move {
                let mut tracks = client.track_radio(&id).await?;
                tracks.retain(|track| track.id != heard && track.playable);
                fastrand::shuffle(&mut tracks);
                anyhow::Ok(tracks)
            }))
            .await;

            this.update(cx, |this, cx| match loaded {
                Ok(tracks) if !tracks.is_empty() => {
                    this.queue.update(cx, |queue, cx| {
                        for track in tracks {
                            queue.append(track, cx);
                        }
                    });
                    this.follow_queue(Start::Segue, cx);
                }
                Ok(_) => log::warn!("playback: radio returned no tracks"),
                Err(error) => log::warn!("playback: cannot extend radio: {error:#}"),
            })
            .ok();
        }));
    }

    fn follow_queue(&mut self, start: Start, cx: &mut Context<Self>) {
        let Some(track) = self.queue.update(cx, |queue, cx| queue.next(cx)) else {
            return;
        };
        self.load_after(&track, start, cx);
    }

    pub fn previous(&mut self, cx: &mut Context<Self>) {
        self.fetch = None;
        let start = self.burst();
        let Some(track) = self.queue.update(cx, |queue, cx| queue.previous(cx)) else {
            return;
        };
        self.load_after(&track, start, cx);
    }

    pub fn play_past(&mut self, index: usize, cx: &mut Context<Self>) {
        self.fetch = None;
        let Some(track) = self
            .queue
            .update(cx, |queue, cx| queue.play_past(index, cx))
        else {
            return;
        };
        self.load_after(&track, Start::Pick, cx);
    }

    pub fn play_upcoming(&mut self, index: usize, cx: &mut Context<Self>) {
        self.fetch = None;
        let Some(track) = self
            .queue
            .update(cx, |queue, cx| queue.play_upcoming(index, cx))
        else {
            return;
        };
        self.load_after(&track, Start::Pick, cx);
    }

    pub fn resume(&mut self, cx: &mut Context<Self>) {
        if let Some(at) = self.armed {
            return self.rearm(at, cx);
        }
        if let Some(engine) = self.active_engine() {
            engine.play();
            cx.notify();
        }
    }

    fn rearm(&mut self, at: Duration, cx: &mut Context<Self>) {
        let Some(track) = self.track.clone() else {
            return;
        };
        if self.active_engine().is_none() {
            return;
        }
        self.load_after(&track, Start::Pick, cx);
        self.settle = Some(at);
    }

    fn adopt(&mut self, cx: &mut Context<Self>) {
        if self.track.is_some() || !self.queue.read(cx).is_empty() {
            return;
        }
        let Some(slug) = self.session.read(cx).provider_slug() else {
            return;
        };
        let Some(resume) = self
            .settings
            .read(cx)
            .resume()
            .filter(|resume| resume.provider == slug)
            .cloned()
        else {
            return;
        };

        let at = Duration::from_secs_f32(resume.position.max(0.));
        let Some(track) = self.queue.update(cx, |queue, cx| queue.revive(resume, cx)) else {
            return;
        };
        self.track = Some(track);
        self.state = PlaybackState::Paused;
        self.position = at;
        self.stored = at;
        self.armed = Some(at);
        cx.notify();
    }

    fn remember(&mut self, force: bool, cx: &mut Context<Self>) {
        let position = self.position;
        if !force && position.abs_diff(self.stored) < RESUME_STEP {
            return;
        }
        self.stored = position;
        let seconds = position.as_secs_f32();
        self.settings
            .update(cx, |settings, cx| settings.set_resume_position(seconds, cx));
    }

    pub fn pause(&mut self, cx: &mut Context<Self>) {
        if let Some(engine) = self.active_engine() {
            engine.pause();
            cx.notify();
        }
    }

    pub fn toggle_play(&mut self, cx: &mut Context<Self>) {
        if self.state == PlaybackState::Playing {
            self.pause(cx);
        } else {
            self.resume(cx);
        }
    }

    pub fn toggle_origin(&mut self, origin: &Origin, cx: &mut Context<Self>) {
        match self.playing_from(origin) {
            Some(PlaybackState::Playing) => self.pause(cx),
            Some(PlaybackState::Paused) => self.resume(cx),
            _ => match origin {
                Origin::Album(id) => self.play_album(id, cx),
                Origin::Playlist(id) => self.play_playlist(id, cx),
                Origin::Artist(id) => self.play_artist(id, cx),
                Origin::Radio(id) => self.play_track(id, cx),
            },
        }
    }

    pub fn seek(&mut self, position: Duration, cx: &mut Context<Self>) {
        if self.armed.is_some() {
            self.armed = Some(position);
            self.position = position;
            self.remember(true, cx);
            cx.notify();
            return;
        }
        if let Some(engine) = self.active_engine() {
            engine.seek(position);
            self.position = position;
            cx.notify();
        }
    }

    pub fn seek_fraction(&mut self, fraction: f32, cx: &mut Context<Self>) {
        let Some(total) = self
            .track
            .as_ref()
            .map(|track| track.duration)
            .filter(|total| !total.is_zero())
        else {
            return;
        };

        let position = Duration::from_secs_f32(total.as_secs_f32() * fraction.clamp(0., 1.));
        self.seek(position, cx);
    }

    pub fn state(&self) -> &PlaybackState {
        &self.state
    }

    pub fn position(&self) -> Duration {
        self.position
    }

    pub fn track(&self) -> Option<&Track> {
        self.track.as_ref()
    }

    pub fn progress(&self) -> f32 {
        let Some(total) = self.track.as_ref().map(|track| track.duration) else {
            return 0.;
        };
        if total.is_zero() {
            return 0.;
        }
        (self.position.as_secs_f32() / total.as_secs_f32()).clamp(0., 1.)
    }

    pub fn is_loading(&self) -> bool {
        matches!(self.state, PlaybackState::Loading)
    }

    fn has_active_playback(&self) -> bool {
        self.track.is_some()
            && matches!(
                self.state,
                PlaybackState::Playing | PlaybackState::Paused | PlaybackState::Loading
            )
    }

    pub fn volume(&self) -> f32 {
        self.level
    }

    pub fn set_volume(&mut self, level: f32, cx: &mut Context<Self>) {
        self.level = level.clamp(0., 1.);
        self.settings
            .update(cx, |settings, cx| settings.set_volume(self.level, cx));
        let level = gain(self.level);
        if let Some(engine) = self.engine.as_ref() {
            engine.set_gain(level);
        }
        if let Some(engine) = self.local_engine.as_ref() {
            engine.set_gain(level);
        }
        cx.notify();
    }

    pub fn normalisation(&self) -> bool {
        self.normalisation
    }

    pub fn gapless(&self) -> bool {
        self.gapless
    }

    pub fn set_gapless(&mut self, on: bool, cx: &mut Context<Self>) {
        if self.gapless == on {
            return;
        }
        self.gapless = on;
        self.settings
            .update(cx, |settings, cx| settings.set_gapless(on, cx));
        self.restart_engine(cx);
    }

    pub fn set_normalisation(&mut self, on: bool, cx: &mut Context<Self>) {
        if self.normalisation == on {
            return;
        }
        self.normalisation = on;
        self.settings
            .update(cx, |settings, cx| settings.set_normalisation(on, cx));
        self.restart_engine(cx);
    }

    fn restart_engine(&mut self, cx: &mut Context<Self>) {
        if self.engine.is_some() {
            let playback = self.session.read(cx).playback();
            if let Some(playback) = playback {
                self.start_engine(playback, cx);
                return;
            }
        }
        cx.notify();
    }

    fn start_engine(&mut self, playback: Arc<dyn PlaybackFactory>, cx: &mut Context<Self>) {
        let config = PlaybackConfig {
            normalisation: self.normalisation,
            gapless: self.gapless,
            position_interval: POSITION_INTERVAL,
            gain: gain(self.level),
        };
        let (engine, events) = playback.start(config);

        self.listen(events, false, cx);
        self.engine = Some(engine);
        let local_active = self
            .track
            .as_ref()
            .and_then(|track| track.id.as_deref())
            .is_some_and(music::is_local_id);
        if !local_active {
            self.state = PlaybackState::Idle;
            self.position = Duration::ZERO;
        }
        cx.notify();
    }

    fn start_local_engine(&mut self, playback: Arc<dyn PlaybackFactory>, cx: &mut Context<Self>) {
        let config = PlaybackConfig {
            normalisation: self.normalisation,
            gapless: self.gapless,
            position_interval: POSITION_INTERVAL,
            gain: gain(self.level),
        };
        let (engine, events) = playback.start(config);

        self.listen(events, true, cx);
        self.local_engine = Some(engine);
    }

    fn listen(&mut self, mut events: Box<dyn PlaybackEvents>, local: bool, cx: &mut Context<Self>) {
        let task = Some(cx.spawn(async move |this, cx| {
            while let Some(event) = events.next().await {
                if this
                    .update(cx, |this, cx| this.apply(event, local, cx))
                    .is_err()
                {
                    break;
                }
            }
        }));
        match local {
            true => self.local_task = task,
            false => self.task = task,
        }
    }

    fn apply(&mut self, event: BackendEvent, local: bool, cx: &mut Context<Self>) {
        let active = self
            .track
            .as_ref()
            .and_then(|track| track.id.as_deref())
            .is_some_and(music::is_local_id);
        if local != active {
            return;
        }
        match event {
            BackendEvent::Loading(position) => {
                self.state = PlaybackState::Loading;
                self.position = position;
            }
            BackendEvent::Playing(position) => {
                let started = self.state != PlaybackState::Playing;
                self.state = PlaybackState::Playing;
                self.position = position;
                if let Some(at) = self.settle.take() {
                    self.seek(at, cx);
                }
                if started {
                    cx.emit(PlaybackEvent::StartedPlayback);
                }
            }
            BackendEvent::Paused(position) => {
                self.state = PlaybackState::Paused;
                self.position = position;
                self.remember(true, cx);
            }
            BackendEvent::Position(position) => {
                self.position = position;
                self.remember(false, cx);
                self.preload_next(position, cx);
            }
            BackendEvent::Length(duration) => {
                if let Some(track) = self.track.as_mut()
                    && !duration.is_zero()
                    && track.duration != duration
                {
                    track.duration = duration;
                }
            }
            BackendEvent::Ended => {
                let ended = self.track.take();
                self.state = PlaybackState::Idle;
                self.position = Duration::ZERO;
                cx.emit(PlaybackEvent::EndedPlayback);
                self.advance(ended, cx);
            }
            BackendEvent::Unavailable => {
                let failed = self.track.take();
                let name = failed.map_or_else(|| "?".to_owned(), |track| track.name);
                log::warn!(
                    "playback: {name} failed to load, backing off {}s",
                    KEY_COOLDOWN.as_secs()
                );
                self.blocked_until = Some(Instant::now() + KEY_COOLDOWN);
                self.state = PlaybackState::Idle;
                self.position = Duration::ZERO;
                Toasts::about(Note::Failed, "toast-track-unplayable", name, cx);
                cx.emit(PlaybackEvent::EndedPlayback);
            }
            BackendEvent::Refused => {
                self.refuse(cx);
                cx.emit(PlaybackEvent::EndedPlayback);
            }
        }
        cx.notify();
    }

    fn teardown(&mut self, cx: &mut Context<Self>) {
        self.task = None;
        self.engine = None;

        let local_active = self
            .track
            .as_ref()
            .and_then(|track| track.id.as_deref())
            .is_some_and(music::is_local_id);
        if !local_active {
            self.load = None;
            self.fetch = None;
            self.enqueue = None;
            self.suggest = None;
            self.seeded = None;
            self.preloaded = None;
            self.skipped = None;
            self.blocked_until = None;
            self.refused = false;
            self.track = None;
            self.origin = None;
            self.state = PlaybackState::Idle;
            self.position = Duration::ZERO;
            self.armed = None;
            self.settle = None;
            self.stored = Duration::ZERO;
        }
        cx.notify();
    }

    fn failed(&mut self, problem: String, cx: &mut Context<Self>) {
        log::error!("playback: {problem}");
        self.state = PlaybackState::Failed(problem);
        cx.notify();
    }

    fn refuse(&mut self, cx: &mut Context<Self>) {
        let first = !self.refused;
        self.refused = true;
        self.track = None;
        self.blocked_until = None;
        self.state = PlaybackState::Failed(
            "spotify denied an audio key for this account; nothing will play in this session"
                .to_owned(),
        );
        if first {
            log::error!(
                "playback: spotify denied an audio key for this account; nothing will play in \
                 this session"
            );
        }
        Toasts::show(Note::Failed, "toast-keys-refused", cx);
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::gain;

    #[test]
    fn never_amplifies_past_unity() {
        assert_eq!(gain(1.), 1.);
        for step in 0..=100 {
            assert!(gain(step as f32 / 100.) <= 1.);
        }
    }

    #[test]
    fn silences_a_closed_slider() {
        assert_eq!(gain(0.), 0.);
        assert_eq!(gain(-1.), 0.);
        assert_eq!(gain(2.), 1.);
    }

    #[test]
    fn rises_with_the_slider() {
        let mut last = gain(0.);
        for step in 1..=100 {
            let next = gain(step as f32 / 100.);
            assert!(next > last, "gain fell at {step}");
            last = next;
        }
    }

    #[test]
    fn halves_the_slider_to_the_taper_midpoint() {
        let expected = 10f32.powf(-super::TAPER_DB / 40.);
        assert!((gain(0.5) - expected).abs() < 1e-6);
    }
}
