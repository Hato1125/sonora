use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use cpal::traits::{DeviceTrait, HostTrait};
use librespot_playback::audio_backend::{Sink, SinkError, SinkResult};
use librespot_playback::config::AudioFormat;
use librespot_playback::convert::Converter;
use librespot_playback::decoder::AudioPacket;
use librespot_playback::{NUM_CHANNELS, SAMPLE_RATE};
use rodio::{OutputStream, OutputStreamBuilder};

const QUEUED_CHUNKS: usize = 26;
const DRAIN_POLL: std::time::Duration = std::time::Duration::from_millis(10);

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

pub struct BlazingSink {
    sink: rodio::Sink,
    _stream: OutputStream,
    flush: Flush,
}

impl BlazingSink {
    pub fn open(format: AudioFormat, flush: Flush) -> Result<Self, SinkError> {
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

        let sink = rodio::Sink::connect_new(stream.mixer());
        sink.pause();

        Ok(Self {
            sink,
            _stream: stream,
            flush,
        })
    }

    pub fn boxed(format: AudioFormat, flush: Flush) -> Box<dyn Sink> {
        match Self::open(format, flush) {
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
