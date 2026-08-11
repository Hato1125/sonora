// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use librespot_playback::audio_backend::{Sink, SinkError, SinkResult};
use librespot_playback::config::AudioFormat;
use librespot_playback::convert::Converter;
use librespot_playback::decoder::AudioPacket;
use librespot_playback::{NUM_CHANNELS, SAMPLE_RATE};

use crate::audio::{Output, Volume, Wanted};

const QUEUED_CHUNKS: usize = 26;
const DRAIN_POLL: Duration = Duration::from_millis(10);

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
    output: Output,
    flush: Flush,
}

impl BlazingSink {
    pub fn open(format: AudioFormat, flush: Flush, volume: Volume) -> Result<Self, SinkError> {
        let wanted = Wanted {
            channels: NUM_CHANNELS as u16,
            sample_rate: SAMPLE_RATE,
            format: Box::new(move |device| output_sample_format(format, device)),
        };
        let output = Output::open(volume, Some(wanted))
            .map_err(|error| SinkError::ConnectionRefused(error.to_string()))?;
        output.sink().pause();

        Ok(Self { output, flush })
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
        self.output.sink().play();
        Ok(())
    }

    fn stop(&mut self) -> SinkResult<()> {
        self.output.sink().pause();
        Ok(())
    }

    fn write(&mut self, packet: AudioPacket, converter: &mut Converter) -> SinkResult<()> {
        if self.flush.take() {
            self.output.sink().clear();
            self.output.sink().play();
        }

        let samples = packet
            .samples()
            .map_err(|error| SinkError::OnWrite(error.to_string()))?;
        let samples = converter.f64_to_f32(samples);

        self.output.sink().append(rodio::buffer::SamplesBuffer::new(
            NUM_CHANNELS as cpal::ChannelCount,
            SAMPLE_RATE,
            &*samples,
        ));

        while self.output.sink().len() > QUEUED_CHUNKS {
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

fn output_sample_format(input: AudioFormat, device: cpal::SampleFormat) -> cpal::SampleFormat {
    if cfg!(target_os = "windows") {
        device
    } else {
        match input {
            AudioFormat::F64 => cpal::SampleFormat::F64,
            AudioFormat::F32 => cpal::SampleFormat::F32,
            AudioFormat::S32 => cpal::SampleFormat::I32,
            AudioFormat::S24 | AudioFormat::S24_3 => cpal::SampleFormat::I24,
            AudioFormat::S16 => cpal::SampleFormat::I16,
        }
    }
}

#[cfg(test)]
mod tests {
    use librespot_playback::config::AudioFormat;

    use super::output_sample_format;

    #[test]
    fn chooses_the_sample_format_for_the_platform() {
        let selected = output_sample_format(AudioFormat::F32, cpal::SampleFormat::I16);

        #[cfg(target_os = "windows")]
        assert_eq!(selected, cpal::SampleFormat::I16);
        #[cfg(not(target_os = "windows"))]
        assert_eq!(selected, cpal::SampleFormat::F32);
    }
}
