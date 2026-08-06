use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait};
use librespot_playback::audio_backend::{Sink, SinkError, SinkResult};
use librespot_playback::config::AudioFormat;
use librespot_playback::convert::Converter;
use librespot_playback::decoder::AudioPacket;
use librespot_playback::{NUM_CHANNELS, SAMPLE_RATE};
use rodio::{OutputStream, OutputStreamBuilder};

const QUEUED_CHUNKS: usize = 26;
const DRAIN_POLL: Duration = Duration::from_millis(10);
const GAIN_RAMP_DURATION: Duration = Duration::from_millis(25);
const GAIN_RAMP_TICK: Duration = Duration::from_millis(4);

#[derive(Clone, Default)]
pub struct Flush(Arc<AtomicBool>);

impl Flush {
    pub fn request(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    fn take(&self) -> bool {
        self.0.swap(false, Ordering::Relaxed)
    }
}

#[derive(Clone)]
pub struct Volume(Arc<VolumeControl>);

struct VolumeControl {
    target: AtomicU32,
    changed: Mutex<()>,
    wake: Condvar,
}

impl Volume {
    pub fn new(gain: f32) -> Self {
        Self(Arc::new(VolumeControl {
            target: AtomicU32::new(gain.to_bits()),
            changed: Mutex::new(()),
            wake: Condvar::new(),
        }))
    }

    pub fn set(&self, gain: f32) {
        let _changed = self
            .0
            .changed
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        self.0.target.store(gain.to_bits(), Ordering::Release);
        self.0.wake.notify_all();
    }

    fn get(&self) -> f32 {
        f32::from_bits(self.0.target.load(Ordering::Acquire))
    }
}

struct GainRamp {
    stop: Arc<AtomicBool>,
    volume: Volume,
    worker: Option<JoinHandle<()>>,
}

impl GainRamp {
    fn start(sink: Arc<rodio::Sink>, volume: Volume, initial: f32) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = stop.clone();
        let worker_volume = volume.clone();
        let worker = std::thread::spawn(move || {
            let mut applied = initial;
            let mut target = initial;
            let mut from = initial;
            let mut started = Instant::now();
            let mut changed = worker_volume
                .0
                .changed
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());

            loop {
                if worker_stop.load(Ordering::Acquire) {
                    break;
                }

                let requested = worker_volume.get();
                let now = Instant::now();
                if requested != target {
                    from = applied;
                    target = requested;
                    started = now;
                }

                let next = gain_at(from, target, now.saturating_duration_since(started));
                if next != applied {
                    sink.set_volume(next);
                    applied = next;
                }

                if applied == target {
                    changed = worker_volume
                        .0
                        .wake
                        .wait(changed)
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                } else {
                    let waited = worker_volume
                        .0
                        .wake
                        .wait_timeout(changed, GAIN_RAMP_TICK)
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    changed = waited.0;
                }
            }
        });

        Self {
            stop,
            volume,
            worker: Some(worker),
        }
    }
}

impl Drop for GainRamp {
    fn drop(&mut self) {
        {
            let _changed = self
                .volume
                .0
                .changed
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            self.stop.store(true, Ordering::Release);
            self.volume.0.wake.notify_all();
        }
        if self
            .worker
            .take()
            .is_some_and(|worker| worker.join().is_err())
        {
            log::warn!("sink: volume ramp worker panicked");
        }
    }
}

fn gain_at(from: f32, target: f32, elapsed: Duration) -> f32 {
    if target == 0. || elapsed >= GAIN_RAMP_DURATION {
        return target;
    }
    let progress = elapsed.as_secs_f32() / GAIN_RAMP_DURATION.as_secs_f32();
    from + (target - from) * progress
}

pub struct BlazingSink {
    sink: Arc<rodio::Sink>,
    _gain_ramp: GainRamp,
    _stream: OutputStream,
    flush: Flush,
}

impl BlazingSink {
    pub fn open(format: AudioFormat, flush: Flush, volume: Volume) -> Result<Self, SinkError> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| SinkError::ConnectionRefused("no output device".to_owned()))?;

        log::info!(
            "sink: using {}",
            device.name().unwrap_or_else(|_| "unknown".to_owned())
        );

        let default = device
            .default_output_config()
            .map_err(|error| SinkError::InvalidParams(error.to_string()))?;
        let config = device
            .supported_output_configs()
            .map_err(|error| SinkError::InvalidParams(error.to_string()))?
            .find(|config| config.channels() == NUM_CHANNELS as cpal::ChannelCount)
            .and_then(|config| {
                config
                    .try_with_sample_rate(cpal::SampleRate(SAMPLE_RATE))
                    .or_else(|| config.try_with_sample_rate(default.sample_rate()))
            })
            .unwrap_or(default);

        let mut stream = OutputStreamBuilder::default()
            .with_device(device)
            .with_config(&config.config())
            .with_sample_format(sample_format(format))
            .open_stream()
            .map_err(|error| SinkError::ConnectionRefused(error.to_string()))?;
        stream.log_on_drop(false);

        let sink = Arc::new(rodio::Sink::connect_new(stream.mixer()));
        sink.pause();

        let applied = volume.get();
        sink.set_volume(applied);
        let gain_ramp = GainRamp::start(sink.clone(), volume, applied);

        Ok(Self {
            sink,
            _gain_ramp: gain_ramp,
            _stream: stream,
            flush,
        })
    }

    pub fn boxed(format: AudioFormat, flush: Flush, volume: Volume) -> Box<dyn Sink> {
        match Self::open(format, flush, volume) {
            Ok(sink) => Box::new(sink),
            Err(error) => {
                log::error!("sink: cannot open an output device: {error}");
                Box::new(Silence)
            }
        }
    }
}

impl Sink for BlazingSink {
    fn start(&mut self) -> SinkResult<()> {
        self.sink.play();
        Ok(())
    }

    fn stop(&mut self) -> SinkResult<()> {
        self.sink.pause();
        Ok(())
    }

    fn write(&mut self, packet: AudioPacket, converter: &mut Converter) -> SinkResult<()> {
        if self.flush.take() {
            self.sink.clear();
            self.sink.play();
        }

        let samples = packet
            .samples()
            .map_err(|error| SinkError::OnWrite(error.to_string()))?;
        let samples = converter.f64_to_f32(samples);

        self.sink.append(rodio::buffer::SamplesBuffer::new(
            NUM_CHANNELS as cpal::ChannelCount,
            SAMPLE_RATE,
            &*samples,
        ));

        while self.sink.len() > QUEUED_CHUNKS {
            std::thread::sleep(DRAIN_POLL);
        }
        Ok(())
    }
}

struct Silence;

impl Sink for Silence {
    fn write(&mut self, _packet: AudioPacket, _converter: &mut Converter) -> SinkResult<()> {
        Ok(())
    }
}

fn sample_format(format: AudioFormat) -> cpal::SampleFormat {
    match format {
        AudioFormat::F64 => cpal::SampleFormat::F64,
        AudioFormat::F32 => cpal::SampleFormat::F32,
        AudioFormat::S32 => cpal::SampleFormat::I32,
        AudioFormat::S24 | AudioFormat::S24_3 => cpal::SampleFormat::I24,
        AudioFormat::S16 => cpal::SampleFormat::I16,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gain_ramp_uses_elapsed_time() {
        assert_eq!(gain_at(0., 1., Duration::ZERO), 0.);
        assert_eq!(gain_at(0., 1., Duration::from_micros(12_500)), 0.5);
        assert_eq!(gain_at(0., 1., GAIN_RAMP_DURATION), 1.);
        assert_eq!(gain_at(0., 1., Duration::from_secs(1)), 1.);
    }

    #[test]
    fn gain_ramp_mutes_immediately() {
        assert_eq!(gain_at(2., 0., Duration::ZERO), 0.);
    }
}
