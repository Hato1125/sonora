// SPDX-License-Identifier: GPL-3.0-or-later

use std::io::Cursor;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context as _, Result};
use async_trait::async_trait;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use ytmusic::YtMusic;

use crate::{PlaybackConfig, PlaybackEvent, PlaybackEvents, PlaybackFactory, Player};

const NORMAL_CAP: f32 = 1.0;

enum Command {
    Load { id: String },
    Preload { id: String },
    Play,
    Pause,
    Seek(Duration),
    Gain(f32),
}

pub struct Factory {
    api: Arc<YtMusic>,
}

impl Factory {
    pub fn new(api: Arc<YtMusic>) -> Self {
        Self { api }
    }
}

impl PlaybackFactory for Factory {
    fn start(&self, config: PlaybackConfig) -> (Box<dyn Player>, Box<dyn PlaybackEvents>) {
        let (commands, command_rx) = unbounded_channel();
        let (events, event_rx) = unbounded_channel();
        let api = self.api.clone();
        let spawned = std::thread::Builder::new()
            .name("yt-playback".to_string())
            .spawn(move || run(api, config, command_rx, events));
        if let Err(error) = spawned {
            log::error!("playback: cannot spawn engine thread: {error}");
        }
        (Box::new(Engine { commands }), Box::new(Events(event_rx)))
    }
}

struct Engine {
    commands: UnboundedSender<Command>,
}

impl Player for Engine {
    fn load(&self, track_id: &str, _seamless: bool) -> Result<()> {
        self.commands
            .send(Command::Load {
                id: track_id.to_string(),
            })
            .context("cannot reach playback engine")
    }

    fn preload(&self, track_id: &str) -> Result<()> {
        self.commands
            .send(Command::Preload {
                id: track_id.to_string(),
            })
            .context("cannot reach playback engine")
    }

    fn play(&self) {
        self.commands.send(Command::Play).ok();
    }

    fn pause(&self) {
        self.commands.send(Command::Pause).ok();
    }

    fn seek(&self, position: Duration) {
        self.commands.send(Command::Seek(position)).ok();
    }

    fn set_gain(&self, gain: f32) {
        self.commands.send(Command::Gain(gain)).ok();
    }
}

pub struct Events(UnboundedReceiver<PlaybackEvent>);

#[async_trait]
impl PlaybackEvents for Events {
    async fn next(&mut self) -> Option<PlaybackEvent> {
        self.0.recv().await
    }
}

struct Loaded {
    data: Vec<u8>,
    loudness_db: Option<f32>,
    duration: Option<Duration>,
}

fn run(
    api: Arc<YtMusic>,
    config: PlaybackConfig,
    commands: UnboundedReceiver<Command>,
    events: UnboundedSender<PlaybackEvent>,
) {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            log::error!("playback: cannot build engine runtime: {error}");
            return;
        }
    };
    runtime.block_on(engine_loop(api, config, commands, events));
}

async fn engine_loop(
    api: Arc<YtMusic>,
    config: PlaybackConfig,
    mut commands: UnboundedReceiver<Command>,
    events: UnboundedSender<PlaybackEvent>,
) {
    let stream = match rodio::OutputStreamBuilder::open_default_stream() {
        Ok(stream) => stream,
        Err(error) => {
            log::error!("playback: cannot open audio output: {error}");
            return;
        }
    };
    let sink = rodio::Sink::connect_new(stream.mixer());
    sink.pause();

    let mut gain = config.gain;
    sink.set_volume(gain);
    let mut ticker = tokio::time::interval(config.position_interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut playing = false;
    let mut current: Option<(String, Option<Duration>)> = None;
    let mut queued: Option<(String, Loaded)> = None;
    let mut prev_len = 0usize;

    loop {
        tokio::select! {
            command = commands.recv() => {
                let Some(command) = command else { break };
                match command {
                    Command::Load { id } => {
                        if current.as_ref().is_some_and(|(cid, _)| *cid == id) {
                            playing = true;
                            if let Some((_, Some(length))) = &current {
                                events.send(PlaybackEvent::Length(*length)).ok();
                            }
                            events.send(PlaybackEvent::Playing(sink.get_pos())).ok();
                            continue;
                        }
                        events.send(PlaybackEvent::Loading(Duration::ZERO)).ok();
                        let reuse = queued.take().filter(|(qid, _)| *qid == id).map(|(_, l)| l);
                        let fetched = match reuse {
                            Some(loaded) => Ok(loaded),
                            None => fetch(&api, &id).await,
                        };
                        match fetched.and_then(|loaded| {
                            let length = loaded.duration;
                            let normal = normalisation(config.normalisation, loaded.loudness_db);
                            let source = decode(loaded.data, normal)?;
                            Ok((source, length))
                        }) {
                            Ok((source, length)) => {
                                sink.clear();
                                sink.append(source);
                                sink.play();
                                playing = true;
                                current = Some((id, length));
                                queued = None;
                                prev_len = sink.len();
                                if let Some(length) = length {
                                    events.send(PlaybackEvent::Length(length)).ok();
                                }
                                events.send(PlaybackEvent::Playing(Duration::ZERO)).ok();
                            }
                            Err(error) => {
                                log::warn!("playback: cannot load {id}: {error:#}");
                                playing = false;
                                current = None;
                                events.send(PlaybackEvent::Unavailable).ok();
                            }
                        }
                    }
                    Command::Preload { id } => {
                        let known = current.as_ref().is_some_and(|(cid, _)| *cid == id)
                            || queued.as_ref().is_some_and(|(qid, _)| *qid == id);
                        if known || current.is_none() {
                            continue;
                        }
                        match fetch(&api, &id).await {
                            Ok(loaded) => {
                                let normal =
                                    normalisation(config.normalisation, loaded.loudness_db);
                                match decode(loaded.data.clone(), normal) {
                                    Ok(source) => {
                                        sink.append(source);
                                        prev_len = sink.len();
                                        queued = Some((id, loaded));
                                    }
                                    Err(error) => {
                                        log::warn!("playback: cannot decode preload {id}: {error:#}")
                                    }
                                }
                            }
                            Err(error) => log::warn!("playback: cannot preload {id}: {error:#}"),
                        }
                    }
                    Command::Play => {
                        if current.is_some() {
                            sink.play();
                            playing = true;
                            events.send(PlaybackEvent::Playing(sink.get_pos())).ok();
                        }
                    }
                    Command::Pause => {
                        sink.pause();
                        playing = false;
                        events.send(PlaybackEvent::Paused(sink.get_pos())).ok();
                    }
                    Command::Seek(position) => {
                        if current.is_some() {
                            if let Err(error) = sink.try_seek(position) {
                                log::warn!("playback: cannot seek: {error}");
                            }
                            events.send(PlaybackEvent::Position(sink.get_pos())).ok();
                        }
                    }
                    Command::Gain(level) => {
                        gain = level;
                        sink.set_volume(gain);
                    }
                }
            }
            _ = ticker.tick() => {
                let len = sink.len();
                if current.is_some() && playing && len < prev_len {
                    events.send(PlaybackEvent::Ended).ok();
                    current = queued.take().map(|(id, loaded)| (id, loaded.duration));
                    playing = current.is_some();
                    if let Some((_, Some(length))) = &current {
                        events.send(PlaybackEvent::Length(*length)).ok();
                    }
                } else if playing {
                    events.send(PlaybackEvent::Position(sink.get_pos())).ok();
                }
                prev_len = len;
            }
        }
    }
}

async fn fetch(api: &YtMusic, id: &str) -> Result<Loaded> {
    let format = api.best_audio(id).await?;
    let duration = format.duration;
    let data = api.download(&format).await?;
    Ok(Loaded {
        data,
        loudness_db: format.loudness_db,
        duration,
    })
}

fn decode(data: Vec<u8>, gain: f32) -> Result<impl rodio::Source + Send + 'static> {
    use rodio::Source as _;
    let length = data.len() as u64;
    let decoder = rodio::Decoder::builder()
        .with_data(Cursor::new(data))
        .with_byte_len(length)
        .with_seekable(true)
        .build()
        .context("cannot decode audio")?;
    Ok(decoder.amplify(gain))
}

fn normalisation(enabled: bool, loudness_db: Option<f32>) -> f32 {
    match (enabled, loudness_db) {
        (true, Some(db)) => 10f32.powf(-db / 20.0).min(NORMAL_CAP),
        _ => 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalisation_attenuates_loud_tracks() {
        let factor = normalisation(true, Some(6.0));
        assert!(factor < 0.51 && factor > 0.49);
    }

    #[test]
    fn normalisation_never_boosts() {
        assert_eq!(normalisation(true, Some(-3.0)), NORMAL_CAP);
        assert_eq!(normalisation(false, Some(6.0)), 1.0);
        assert_eq!(normalisation(true, None), 1.0);
    }
}
