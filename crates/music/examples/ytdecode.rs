// SPDX-License-Identifier: GPL-3.0-or-later

use std::io::Cursor;

use rodio::Source as _;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let video_id = std::env::args().nth(1).unwrap_or("4D7u5KF7SP8".to_string());
    let api = ytmusic::YtMusic::anonymous();
    let format = api.best_audio(&video_id).await?;
    println!(
        "itag={} codec={} bitrate={}",
        format.itag, format.codec, format.bitrate
    );
    let data = api.download(&format).await?;
    println!("downloaded {} bytes", data.len());
    let length = data.len() as u64;
    let decoder = rodio::Decoder::builder()
        .with_data(Cursor::new(data))
        .with_byte_len(length)
        .with_seekable(true)
        .build()?;
    let rate = decoder.sample_rate();
    let channels = decoder.channels();
    let samples: usize = decoder.count();
    println!(
        "decoded {} samples, {}ch @ {} Hz (~{}s)",
        samples,
        channels,
        rate,
        samples as u64 / (rate as u64 * channels as u64),
    );
    Ok(())
}
