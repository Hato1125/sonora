use librespot_core::SpotifyUri;
use librespot_playback::config::{AudioFormat, PlayerConfig};
use librespot_playback::mixer::{Mixer, MixerConfig, softmixer::SoftMixer};
use librespot_playback::player::{Player, PlayerEvent};

use std::sync::Arc;
use std::time::Duration;

use gpui::{Context, Entity, EventEmitter, Task};
use spotify::Track;

use crate::sink::{BlazingSink, Flush};
use crate::{Session, SessionEvent};

const POSITION_INTERVAL: Duration = Duration::from_millis(500);

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
    mixer: Option<SoftMixer>,
    flush: Flush,
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
            mixer: None,
            flush: Flush::default(),
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

    pub fn volume(&self) -> u16 {
        self.mixer.as_ref().map(|mixer| mixer.volume()).unwrap_or(0)
    }

    pub fn set_volume(&mut self, volume: u16, cx: &mut Context<Self>) {
        if let Some(mixer) = self.mixer.as_ref() {
            mixer.set_volume(volume);
            cx.notify();
        }
    }

    fn spawn_player(&mut self, session: librespot_core::Session, cx: &mut Context<Self>) {
        let mixer = match SoftMixer::open(MixerConfig::default()) {
            Ok(mixer) => mixer,
            Err(error) => return self.failed(format!("cannot open the mixer: {error}"), cx),
        };

        let config = PlayerConfig {
            position_update_interval: Some(POSITION_INTERVAL),
            ..Default::default()
        };
        let flush = self.flush.clone();
        let player = Player::new(config, session, mixer.get_soft_volume(), move || {
            BlazingSink::boxed(AudioFormat::default(), flush)
        });

        self.listen(&player, cx);
        self.mixer = Some(mixer);
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
        self.mixer = None;
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
