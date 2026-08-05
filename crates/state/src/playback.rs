use std::time::{Duration, Instant};

use audio::{AudioEvent, AudioEvents, Engine, EngineConfig};
use gpui::{Context, Entity, EventEmitter, Task};
use spotify::Track;

use crate::queue::Queue;
use crate::{Session, SessionEvent};

const POSITION_INTERVAL: Duration = Duration::from_millis(500);
const LOAD_DEBOUNCE: Duration = Duration::from_millis(250);
const KEY_COOLDOWN: Duration = Duration::from_secs(6);
const TAPER_DB: f32 = 60.;
const CEILING_DB: f32 = 9.;
const INITIAL_LEVEL: f32 = 0.7;

fn gain(level: f32) -> f32 {
    let level = level.clamp(0., 1.);
    if level <= 0. {
        return 0.;
    }
    10f32.powf((CEILING_DB - TAPER_DB * (1. - level)) / 20.)
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

pub struct Playback {
    state: PlaybackState,
    position: Duration,
    track: Option<Track>,
    engine: Option<Engine>,
    session: Entity<Session>,
    queue: Entity<Queue>,
    level: f32,
    normalisation: bool,
    task: Option<Task<()>>,
    load: Option<Task<()>>,
    blocked_until: Option<Instant>,
}

impl EventEmitter<PlaybackEvent> for Playback {}

impl Playback {
    pub fn new(session: Entity<Session>, queue: Entity<Queue>, cx: &mut Context<Self>) -> Self {
        cx.subscribe(&session, |this, session, event, cx| match event {
            SessionEvent::SignedIn => {
                let Some(librespot) = session.read(cx).librespot() else {
                    return;
                };
                this.start_engine(librespot, cx);
            }
            SessionEvent::SignedOut => this.teardown(cx),
        })
        .detach();

        Self {
            state: PlaybackState::Idle,
            position: Duration::ZERO,
            track: None,
            engine: None,
            session,
            queue,
            level: INITIAL_LEVEL,
            normalisation: true,
            task: None,
            load: None,
            blocked_until: None,
        }
    }

    pub fn play(&mut self, track: &Track, cx: &mut Context<Self>) {
        if self.engine.is_none() {
            return;
        }
        let Some(id) = track.id.clone() else {
            return self.failed(format!("{} has no track id", track.name), cx);
        };
        if !track.playable {
            return self.failed(format!("{} is not available to stream", track.name), cx);
        }

        self.track = Some(track.clone());
        self.state = PlaybackState::Loading;
        self.position = Duration::ZERO;
        cx.notify();

        let wait = self
            .blocked_until
            .and_then(|until| until.checked_duration_since(Instant::now()))
            .unwrap_or(LOAD_DEBOUNCE)
            .max(LOAD_DEBOUNCE);

        self.load = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(wait).await;
            this.update(cx, |this, cx| {
                let Some(engine) = this.engine.as_ref() else {
                    return;
                };
                if let Err(error) = engine.load(&id) {
                    this.failed(format!("{error:#}"), cx);
                }
            })
            .ok();
        }));
    }

    pub fn start(&mut self, tracks: Vec<Track>, index: usize, cx: &mut Context<Self>) {
        let Some(track) = self
            .queue
            .update(cx, |queue, cx| queue.start(tracks, index, cx))
        else {
            return;
        };
        self.play(&track, cx);
    }

    pub fn next(&mut self, cx: &mut Context<Self>) {
        let Some(track) = self.queue.update(cx, |queue, cx| queue.next(cx)) else {
            return;
        };
        self.play(&track, cx);
    }

    pub fn previous(&mut self, cx: &mut Context<Self>) {
        let Some(track) = self.queue.update(cx, |queue, cx| queue.previous(cx)) else {
            return;
        };
        self.play(&track, cx);
    }

    pub fn resume(&mut self, cx: &mut Context<Self>) {
        if let Some(engine) = self.engine.as_ref() {
            engine.play();
            cx.notify();
        }
    }

    pub fn pause(&mut self, cx: &mut Context<Self>) {
        if let Some(engine) = self.engine.as_ref() {
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

    pub fn seek(&mut self, position: Duration, cx: &mut Context<Self>) {
        if let Some(engine) = self.engine.as_ref() {
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

    pub fn volume(&self) -> f32 {
        self.level
    }

    pub fn set_volume(&mut self, level: f32, cx: &mut Context<Self>) {
        self.level = level.clamp(0., 1.);
        if let Some(engine) = self.engine.as_ref() {
            engine.set_gain(gain(self.level));
        }
        cx.notify();
    }

    pub fn normalisation(&self) -> bool {
        self.normalisation
    }

    pub fn set_normalisation(&mut self, on: bool, cx: &mut Context<Self>) {
        if self.normalisation == on {
            return;
        }
        self.normalisation = on;

        if self.engine.is_some() {
            let session = self.session.read(cx).librespot();
            if let Some(session) = session {
                self.start_engine(session, cx);
                return;
            }
        }
        cx.notify();
    }

    fn start_engine(&mut self, session: librespot_core::Session, cx: &mut Context<Self>) {
        let config = EngineConfig {
            normalisation: self.normalisation,
            position_interval: POSITION_INTERVAL,
            gain: gain(self.level),
        };
        let (engine, events) = Engine::start(session, config);

        self.listen(events, cx);
        self.engine = Some(engine);
        self.state = PlaybackState::Idle;
        self.position = Duration::ZERO;
        cx.notify();
    }

    fn listen(&mut self, mut events: AudioEvents, cx: &mut Context<Self>) {
        self.task = Some(cx.spawn(async move |this, cx| {
            while let Some(event) = events.next().await {
                if this.update(cx, |this, cx| this.apply(event, cx)).is_err() {
                    break;
                }
            }
        }));
    }

    fn apply(&mut self, event: AudioEvent, cx: &mut Context<Self>) {
        match event {
            AudioEvent::Loading(position) => {
                self.state = PlaybackState::Loading;
                self.position = position;
            }
            AudioEvent::Playing(position) => {
                let started = self.state != PlaybackState::Playing;
                self.state = PlaybackState::Playing;
                self.position = position;
                if started {
                    cx.emit(PlaybackEvent::StartedPlayback);
                }
            }
            AudioEvent::Paused(position) => {
                self.state = PlaybackState::Paused;
                self.position = position;
            }
            AudioEvent::Position(position) => self.position = position,
            AudioEvent::Ended => {
                self.state = PlaybackState::Idle;
                self.position = Duration::ZERO;
                self.track = None;
                cx.emit(PlaybackEvent::EndedPlayback);
                self.next(cx);
            }
            AudioEvent::Unavailable => {
                let name = self.track.as_ref().map(|track| track.name.as_str());
                log::warn!(
                    "playback: {} failed to load, backing off {}s",
                    name.unwrap_or("?"),
                    KEY_COOLDOWN.as_secs()
                );
                self.blocked_until = Some(Instant::now() + KEY_COOLDOWN);
                self.state = PlaybackState::Idle;
                self.position = Duration::ZERO;
                self.track = None;
                cx.emit(PlaybackEvent::EndedPlayback);
            }
        }
        cx.notify();
    }

    fn teardown(&mut self, cx: &mut Context<Self>) {
        self.task = None;
        self.load = None;
        self.blocked_until = None;
        self.engine = None;
        self.track = None;
        self.state = PlaybackState::Idle;
        self.position = Duration::ZERO;
        cx.notify();
    }

    fn failed(&mut self, problem: String, cx: &mut Context<Self>) {
        log::error!("playback: {problem}");
        self.state = PlaybackState::Failed(problem);
        cx.notify();
    }
}
