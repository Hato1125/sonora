use librespot_core::SpotifyUri;
use librespot_playback::config::{AudioFormat, PlayerConfig};
use librespot_playback::mixer::NoOpVolume;
use librespot_playback::player::{Player, PlayerEvent};

use std::sync::Arc;
use std::time::Duration;

use gpui::{Context, Entity, EventEmitter, Task};
use spotify::Track;

use crate::sink::{BlazingSink, Flush, Volume};
use crate::{Session, SessionEvent};

const POSITION_INTERVAL: Duration = Duration::from_millis(500);
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
    player: Option<Arc<Player>>,
    session: Entity<Session>,
    level: f32,
    volume: Volume,
    flush: Flush,
    normalisation: bool,
    task: Option<Task<()>>,
}

impl EventEmitter<PlaybackEvent> for Playback {}

impl Playback {
    pub fn new(session: Entity<Session>, cx: &mut Context<Self>) -> Self {
        cx.subscribe(&session, |this, session, event, cx| match event {
            SessionEvent::SignedIn => {
                let Some(librespot) = session.read(cx).librespot() else {
                    return;
                };
                this.spawn_player(librespot, cx);
            }
            SessionEvent::SignedOut => this.teardown(cx),
        })
        .detach();
        Self {
            state: PlaybackState::Idle,
            position: Duration::ZERO,
            track: None,
            player: None,
            session,
            level: INITIAL_LEVEL,
            volume: Volume::new(gain(INITIAL_LEVEL)),
            flush: Flush::default(),
            normalisation: true,
            task: None,
        }
    }

    pub fn play(&mut self, track: &Track, cx: &mut Context<Self>) {
        let Some(player) = self.player.clone() else {
            return;
        };
        let Some(id) = track.id.as_deref() else {
            return self.failed(format!("{} has no track id", track.name), cx);
        };
        let Ok(uri) = SpotifyUri::from_uri(&format!("spotify:track:{id}")) else {
            return self.failed(format!("{id} is not a track uri"), cx);
        };

        self.flush.request();
        player.load(uri, true, 0);
        self.track = Some(track.clone());
        self.state = PlaybackState::Loading;
        self.position = Duration::ZERO;
        cx.notify();
    }

    pub fn resume(&mut self, cx: &mut Context<Self>) {
        if let Some(player) = self.player.as_ref() {
            player.play();
            cx.notify();
        }
    }

    pub fn pause(&mut self, cx: &mut Context<Self>) {
        if let Some(player) = self.player.as_ref() {
            player.pause();
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
        if let Some(player) = self.player.as_ref() {
            self.flush.request();
            player.seek(position.as_millis() as u32);
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
        self.volume.set(gain(self.level));
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

        if self.player.is_some() {
            let session = self.session.read(cx).librespot();
            if let Some(session) = session {
                self.spawn_player(session, cx);
                return;
            }
        }
        cx.notify();
    }

    fn spawn_player(&mut self, session: librespot_core::Session, cx: &mut Context<Self>) {
        let config = PlayerConfig {
            position_update_interval: Some(POSITION_INTERVAL),
            normalisation: self.normalisation,
            ..Default::default()
        };
        let flush = self.flush.clone();
        let volume = self.volume.clone();
        let player = Player::new(config, session, Box::new(NoOpVolume), move || {
            BlazingSink::boxed(AudioFormat::default(), flush, volume)
        });

        self.listen(&player, cx);
        self.player = Some(player);
        self.state = PlaybackState::Idle;
        self.position = Duration::ZERO;
        cx.notify();
    }

    fn listen(&mut self, player: &Arc<Player>, cx: &mut Context<Self>) {
        let mut events = player.get_player_event_channel();
        self.task = Some(cx.spawn(async move |this, cx| {
            while let Some(event) = events.recv().await {
                if this.update(cx, |this, cx| this.apply(event, cx)).is_err() {
                    break;
                }
            }
        }));
    }

    fn apply(&mut self, event: PlayerEvent, cx: &mut Context<Self>) {
        match event {
            PlayerEvent::Loading { position_ms, .. } => {
                self.state = PlaybackState::Loading;
                self.position = Duration::from_millis(position_ms as u64);
            }
            PlayerEvent::Playing { position_ms, .. } => {
                let started = self.state != PlaybackState::Playing;
                self.state = PlaybackState::Playing;
                self.position = Duration::from_millis(position_ms as u64);
                if started {
                    cx.emit(PlaybackEvent::StartedPlayback);
                }
            }
            PlayerEvent::Paused { position_ms, .. } => {
                self.state = PlaybackState::Paused;
                self.position = Duration::from_millis(position_ms as u64);
            }
            PlayerEvent::PositionChanged { position_ms, .. }
            | PlayerEvent::PositionCorrection { position_ms, .. } => {
                self.position = Duration::from_millis(position_ms as u64);
            }
            PlayerEvent::Stopped { .. } | PlayerEvent::EndOfTrack { .. } => {
                self.state = PlaybackState::Idle;
                self.position = Duration::ZERO;
                self.track = None;
                cx.emit(PlaybackEvent::EndedPlayback);
            }
            PlayerEvent::Unavailable { .. } => {
                return self.failed("Spotify could not serve this track".to_owned(), cx);
            }
            _ => return,
        }
        cx.notify();
    }

    fn teardown(&mut self, cx: &mut Context<Self>) {
        self.task = None;
        self.player = None;
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
