use std::time::Duration;

use anyhow::{Context as _, Result};
use async_trait::async_trait;
use serde::Deserialize;

use crate::{Lyrics, LyricsHit, LyricsLine, LyricsProvider, LyricsQuery};

const SOURCE: &str = "LrcLib";
const ENDPOINT: &str = "https://lrclib.net/api/search";
const AGENT: &str = concat!(
    "sonora/",
    env!("CARGO_PKG_VERSION"),
    " (https://github.com/nolight132/sonora)"
);

pub struct LrcLib {
    http: reqwest::Client,
}

impl LrcLib {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
        }
    }
}

impl Default for LrcLib {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Deserialize)]
struct Found {
    #[serde(rename = "trackName")]
    track_name: Option<String>,
    #[serde(rename = "artistName")]
    artist_name: Option<String>,
    duration: Option<f64>,
    #[serde(rename = "plainLyrics")]
    plain: Option<String>,
    #[serde(rename = "syncedLyrics")]
    synced: Option<String>,
}

#[async_trait]
impl LyricsProvider for LrcLib {
    fn name(&self) -> &'static str {
        SOURCE
    }

    async fn search(&self, query: &LyricsQuery) -> Result<Vec<LyricsHit>> {
        let response = self
            .http
            .get(ENDPOINT)
            .query(&[
                ("track_name", query.title.as_str()),
                ("artist_name", query.artist.as_str()),
            ])
            .header("User-Agent", AGENT)
            .send()
            .await
            .context("cannot reach lrclib")?;
        let status = response.status();
        if !status.is_success() {
            anyhow::bail!("lrclib answered with status {status}");
        }
        let found: Vec<Found> = response
            .json()
            .await
            .context("cannot read the lrclib response")?;
        Ok(found.into_iter().filter_map(hit).collect())
    }
}

fn hit(found: Found) -> Option<LyricsHit> {
    let lyrics = found
        .synced
        .as_deref()
        .map(parse)
        .filter(|lines| !lines.is_empty())
        .map(|lines| Lyrics::Synced {
            lines: lines.into(),
        })
        .or_else(|| {
            found
                .plain
                .as_deref()
                .map(str::trim)
                .filter(|text| !text.is_empty())
                .map(|text| Lyrics::Plain(text.to_owned()))
        })?;

    Some(LyricsHit {
        source: SOURCE,
        lyrics,
        title: found.track_name.unwrap_or_default(),
        artist: found.artist_name.unwrap_or_default(),
        duration: found.duration.map(Duration::from_secs_f64),
    })
}

fn parse(lrc: &str) -> Vec<LyricsLine> {
    let mut lines: Vec<LyricsLine> = lrc
        .lines()
        .filter_map(|line| {
            let (stamp, text) = line.strip_prefix('[')?.split_once(']')?;
            Some(LyricsLine {
                start: stamp_of(stamp)?,
                end: None,
                text: text.trim().to_owned(),
                words: None,
            })
        })
        .collect();
    lines.sort_by_key(|line| line.start);
    for index in 0..lines.len().saturating_sub(1) {
        lines[index].end = Some(lines[index + 1].start);
    }
    lines
}

fn stamp_of(stamp: &str) -> Option<Duration> {
    let (minutes, rest) = stamp.split_once(':')?;
    let minutes: u64 = minutes.trim().parse().ok()?;
    let seconds: f64 = rest.replace(',', ".").parse().ok()?;
    Some(Duration::from_secs_f64(minutes as f64 * 60. + seconds))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_stamp() {
        assert_eq!(stamp_of("01:02.50"), Some(Duration::from_millis(62_500)));
        assert_eq!(stamp_of("00:09"), Some(Duration::from_secs(9)));
        assert_eq!(stamp_of("bogus"), None);
    }

    #[test]
    fn parses_lrc_and_closes_every_line() {
        let lines = parse("[00:10.00] first\n[00:14.50] second\n[bad] skipped\n");

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].text, "first");
        assert_eq!(lines[0].end, Some(Duration::from_millis(14_500)));
        assert_eq!(lines[1].end, None);
    }

    #[test]
    fn prefers_synced_over_plain() {
        let found = Found {
            track_name: Some("Jaded".to_owned()),
            artist_name: Some("Spiritbox".to_owned()),
            duration: Some(263.),
            plain: Some("plain".to_owned()),
            synced: Some("[00:01.00] synced".to_owned()),
        };

        assert!(hit(found).unwrap().lyrics.synced());
    }

    #[test]
    fn skips_an_entry_without_any_lyrics() {
        let found = Found {
            track_name: Some("Jaded".to_owned()),
            artist_name: Some("Spiritbox".to_owned()),
            duration: Some(263.),
            plain: Some("   ".to_owned()),
            synced: None,
        };

        assert!(hit(found).is_none());
    }
}
