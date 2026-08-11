// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use anyhow::{Context as _, Result};
use cpal::traits::{DeviceTrait, HostTrait};
use rodio::source::SeekError;
use rodio::{OutputStream, OutputStreamBuilder, Source};

pub const RAMP: Duration = Duration::from_millis(25);

#[derive(Clone)]
pub struct Volume(Arc<AtomicU32>);

impl Volume {
    pub fn new(gain: f32) -> Self {
        Self(Arc::new(AtomicU32::new(gain.to_bits())))
    }

    pub fn set(&self, gain: f32) {
        self.0.store(gain.to_bits(), Ordering::Relaxed);
    }

    fn get(&self) -> f32 {
        f32::from_bits(self.0.load(Ordering::Relaxed))
    }
}

pub struct Wanted {
    pub channels: u16,
    pub sample_rate: u32,
    pub format: Box<dyn FnOnce(cpal::SampleFormat) -> cpal::SampleFormat>,
}

pub struct Output {
    sink: Arc<rodio::Sink>,
    volume: Volume,
    _stream: OutputStream,
}

impl Output {
    pub fn open(volume: Volume, wanted: Option<Wanted>) -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .context("no audio output device")?;

        log::info!(
            "sink: using {}",
            device.name().unwrap_or_else(|_| "unknown".to_owned())
        );

        let mut builder = OutputStreamBuilder::default().with_device(device.clone());
        if let Some(wanted) = wanted {
            let default = device
                .default_output_config()
                .map_err(|error| anyhow::anyhow!("cannot read the output config: {error}"))?;
            let config = device
                .supported_output_configs()
                .map_err(|error| anyhow::anyhow!("cannot list the output configs: {error}"))?
                .find(|config| config.channels() == wanted.channels)
                .and_then(|config| {
                    config
                        .try_with_sample_rate(cpal::SampleRate(wanted.sample_rate))
                        .or_else(|| config.try_with_sample_rate(default.sample_rate()))
                })
                .unwrap_or(default);
            let format = (wanted.format)(config.sample_format());
            builder = builder
                .with_config(&config.config())
                .with_sample_format(format);
        }
        let mut stream = builder
            .open_stream()
            .map_err(|error| anyhow::anyhow!("cannot open the audio output: {error}"))?;
        stream.log_on_drop(false);

        let applied = volume.get();
        let (sink, source) = rodio::Sink::new();
        stream
            .mixer()
            .add(SmoothGain::new(source, volume.clone(), applied, RAMP));

        Ok(Self {
            sink: Arc::new(sink),
            volume,
            _stream: stream,
        })
    }

    pub fn sink(&self) -> &Arc<rodio::Sink> {
        &self.sink
    }

    pub fn set_volume(&self, gain: f32) {
        self.volume.set(gain);
    }
}

pub struct SmoothGain<I> {
    input: I,
    volume: Volume,

    current: f32,
    target: f32,
    step: f32,

    frames_left: u32,
    ramp_frames: u32,

    channel: u16,
    channels: u16,
}

impl<I: Source> SmoothGain<I> {
    pub fn new(input: I, volume: Volume, initial: f32, duration: Duration) -> Self {
        let channels = input.channels();
        let ramp_frames = (duration.as_secs_f64() * input.sample_rate() as f64)
            .round()
            .max(1.0) as u32;

        Self {
            input,
            volume,
            current: initial,
            target: initial,
            step: 0.0,
            frames_left: 0,
            ramp_frames,
            channel: 0,
            channels,
        }
    }
}

impl<I: Source> Iterator for SmoothGain<I> {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let sample = self.input.next()?;

        if self.channel == 0 {
            let requested = self.volume.get().max(0.0);

            if requested.to_bits() != self.target.to_bits() {
                self.target = requested;
                self.frames_left = self.ramp_frames;
                self.step = (self.target - self.current) / self.ramp_frames as f32;
            }

            if self.frames_left > 0 {
                self.current += self.step;
                self.frames_left -= 1;

                if self.frames_left == 0 {
                    self.current = self.target;
                }
            }
        }

        let output = sample * self.current;

        self.channel += 1;
        if self.channel == self.channels {
            self.channel = 0;
        }

        Some(output)
    }
}

impl<I: Source> Source for SmoothGain<I> {
    fn current_span_len(&self) -> Option<usize> {
        self.input.current_span_len()
    }

    fn channels(&self) -> u16 {
        self.input.channels()
    }

    fn sample_rate(&self) -> u32 {
        self.input.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.input.total_duration()
    }

    fn try_seek(&mut self, position: Duration) -> Result<(), SeekError> {
        self.input.try_seek(position)
    }
}
