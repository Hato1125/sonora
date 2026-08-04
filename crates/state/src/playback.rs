use librespot_core::SpotifyUri;
use librespot_playback::audio_backend;
use librespot_playback::config::{AudioFormat, PlayerConfig};
use librespot_playback::mixer::{Mixer, MixerConfig, softmixer::SoftMixer};
use librespot_playback::player::{Player, PlayerEvent};

use std::sync::Arc;
use std::time::Duration;

use gpui::{Context, Entity, EventEmitter, Task};

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
    player: Option<Arc<Player>>,
    mixer: Option<SoftMixer>,
    session: Entity<Session>,
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
            player: None,
            mixer: None,
            session,
            task: None,
        }
    }

    pub fn play(&mut self, track_id: &str, cx: &mut Context<Self>) {
        let Some(player) = self.player.clone() else {
            return;
        };
        let Ok(uri) = SpotifyUri::from_uri(&format!("spotify:track:{track_id}")) else {
            return self.failed(format!("{track_id} is not a track uri"), cx);
        };
        player.load(uri, true, 0);
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

    pub fn seek(&mut self, position: Duration, cx: &mut Context<Self>) {
        if let Some(player) = self.player.as_ref() {
            player.seek(position.as_millis() as u32);
            cx.notify();
        }
    }

    pub fn state(&self) -> &PlaybackState {
        &self.state
    }

    pub fn position(&self) -> Duration {
        self.position
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
        let Some(backend) = audio_backend::find(None) else {
            return self.failed("no audio backend was compiled in".to_owned(), cx);
        };
        let mixer = match SoftMixer::open(MixerConfig::default()) {
            Ok(mixer) => mixer,
            Err(error) => return self.failed(format!("cannot open the mixer: {error}"), cx),
        };

        let config = PlayerConfig {
            position_update_interval: Some(POSITION_INTERVAL),
            ..Default::default()
        };
        let player = Player::new(config, session, mixer.get_soft_volume(), move || {
            backend(None, AudioFormat::default())
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
