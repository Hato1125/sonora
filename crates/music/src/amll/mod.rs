use std::time::Duration;

use anyhow::{Context as _, Result};
use async_trait::async_trait;
use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

use crate::lyrics::lrc;
use crate::{Lyrics, LyricsHit, LyricsLine, LyricsProvider, LyricsQuery, LyricsWord};

const SOURCE: &str = "AMLL";
const ENDPOINT: &str = "https://raw.githubusercontent.com/amll-dev/amll-ttml-db/refs/heads/main";
const TRUST: u32 = 50;
const AGENT: &str = concat!(
    "sonora/",
    env!("CARGO_PKG_VERSION"),
    " (https://github.com/nolight132/sonora)"
);

pub struct AmllDb {
    http: reqwest::Client,
}

impl AmllDb {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
        }
    }
}

impl Default for AmllDb {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LyricsProvider for AmllDb {
    fn name(&self) -> &'static str {
        SOURCE
    }

    async fn search(&self, query: &LyricsQuery) -> Result<Vec<LyricsHit>> {
        let Some(id) = query.id_for("spotify") else {
            return Ok(Vec::new());
        };
        let response = self
            .http
            .get(format!("{ENDPOINT}/spotify-lyrics/{id}.ttml"))
            .header("User-Agent", AGENT)
            .send()
            .await
            .context("cannot reach the amll database")?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(Vec::new());
        }
        let status = response.status();
        if !status.is_success() {
            anyhow::bail!("the amll database answered with status {status}");
        }
        let ttml = response
            .text()
            .await
            .context("cannot read the amll response")?;
        Ok(hit(&ttml).into_iter().collect())
    }
}

struct Sheet {
    lines: Vec<LyricsLine>,
    title: String,
    artist: String,
    album: Option<String>,
    duration: Option<Duration>,
    writers: Vec<String>,
}

fn hit(ttml: &str) -> Option<LyricsHit> {
    let sheet = parse(ttml)?;
    (!sheet.lines.is_empty()).then(|| LyricsHit {
        source: SOURCE,
        trust: TRUST,
        instrumental: false,
        lyrics: Lyrics::Synced {
            lines: sheet.lines.into(),
        },
        title: sheet.title,
        artist: sheet.artist,
        album: sheet.album,
        duration: sheet.duration,
        writers: sheet.writers,
    })
}

fn parse(ttml: &str) -> Option<Sheet> {
    let mut reader = Reader::from_str(ttml);
    reader.config_mut().trim_text(false);

    let mut sheet = Sheet {
        lines: Vec::new(),
        title: String::new(),
        artist: String::new(),
        album: None,
        duration: None,
        writers: Vec::new(),
    };
    let mut line: Option<LyricsLine> = None;
    let mut word: Option<LyricsWord> = None;
    let mut writer: Option<String> = None;
    let mut hush = 0usize;

    loop {
        match reader.read_event().ok()? {
            Event::Eof => break,
            Event::Empty(tag) if tag.name().as_ref() == b"amll:meta" => {
                note(&mut sheet, &tag);
            }
            Event::Start(_) if hush > 0 => hush += 1,
            Event::End(_) if hush > 0 => hush -= 1,
            Event::Start(tag) => match tag.name().as_ref() {
                b"body" => {
                    sheet.duration = attr(&tag, b"dur").as_deref().and_then(clock_of);
                }
                b"p" => {
                    line = attr(&tag, b"begin")
                        .as_deref()
                        .and_then(clock_of)
                        .map(|start| LyricsLine {
                            start,
                            end: attr(&tag, b"end").as_deref().and_then(clock_of),
                            text: String::new(),
                            romanized: None,
                            words: None,
                            secondary: Vec::new(),
                        });
                }
                b"span" => {
                    if attr(&tag, b"ttm:role").is_some() {
                        hush += 1;
                        continue;
                    }
                    word = attr(&tag, b"begin")
                        .as_deref()
                        .and_then(clock_of)
                        .map(|start| LyricsWord {
                            start,
                            end: attr(&tag, b"end")
                                .as_deref()
                                .and_then(clock_of)
                                .unwrap_or(start),
                            text: String::new(),
                        });
                }
                b"songwriter" => writer = Some(String::new()),
                _ => {}
            },
            Event::Text(text) if hush == 0 => {
                let Ok(text) = text.xml_content() else {
                    continue;
                };
                match &mut writer {
                    Some(writer) => writer.push_str(&text),
                    None => spell(&text, &mut word, &mut line),
                }
            }
            Event::GeneralRef(name) if hush == 0 => {
                let Some(letter) = unref(name.as_ref()) else {
                    continue;
                };
                match &mut writer {
                    Some(writer) => writer.push_str(letter),
                    None => spell(letter, &mut word, &mut line),
                }
            }
            Event::End(tag) => match tag.name().as_ref() {
                b"span" => {
                    if let (Some(sung), Some(line)) = (word.take(), &mut line)
                        && !sung.text.is_empty()
                    {
                        line.words.get_or_insert_default().push(sung);
                    }
                }
                b"songwriter" => {
                    if let Some(named) = writer.take() {
                        let named = named.trim().to_owned();
                        if !named.is_empty() && !sheet.writers.contains(&named) {
                            sheet.writers.push(named);
                        }
                    }
                }
                b"p" => {
                    if let Some(mut done) = line.take() {
                        done.text = done.text.trim().to_owned();
                        if !done.text.is_empty() {
                            sheet.lines.push(done);
                        }
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }

    lrc::normalize(&mut sheet.lines);
    Some(sheet)
}

fn spell(text: &str, word: &mut Option<LyricsWord>, line: &mut Option<LyricsLine>) {
    match (word, line) {
        (Some(word), Some(line)) => {
            word.text.push_str(text);
            line.text.push_str(text);
        }
        (None, Some(line)) => line.text.push_str(text),
        _ => {}
    }
}

fn unref(name: &[u8]) -> Option<&'static str> {
    match name {
        b"amp" => Some("&"),
        b"lt" => Some("<"),
        b"gt" => Some(">"),
        b"quot" => Some("\""),
        b"apos" => Some("'"),
        _ => None,
    }
}

fn note(sheet: &mut Sheet, tag: &BytesStart) {
    let (Some(key), Some(value)) = (attr(tag, b"key"), attr(tag, b"value")) else {
        return;
    };
    match key.as_str() {
        "musicName" => sheet.title = value,
        "artists" if sheet.artist.is_empty() => sheet.artist = value,
        "album" => sheet.album = Some(value),
        _ => {}
    }
}

fn attr(tag: &BytesStart, name: &[u8]) -> Option<String> {
    tag.try_get_attribute(name)
        .ok()
        .flatten()
        .and_then(|found| found.unescape_value().ok())
        .map(|value| value.into_owned())
}

fn clock_of(stamp: &str) -> Option<Duration> {
    let mut parts = stamp.rsplit(':');
    let seconds: f64 = parts.next()?.parse().ok()?;
    if !seconds.is_finite() || seconds < 0. {
        return None;
    }
    let minutes: u64 = match parts.next() {
        Some(minutes) => minutes.parse().ok()?,
        None => 0,
    };
    let hours: u64 = match parts.next() {
        Some(hours) => hours.parse().ok()?,
        None => 0,
    };
    Some(Duration::from_secs_f64(
        (hours * 3600 + minutes * 60) as f64 + seconds,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<tt xmlns="http://www.w3.org/ns/ttml" xmlns:amll="http://www.example.com/ns/amll" xmlns:itunes="http://music.apple.com/lyric-ttml-internal" xmlns:ttm="http://www.w3.org/ns/ttml#metadata" itunes:timing="Word"><head><metadata><ttm:agent type="person" xml:id="v1"/><iTunesMetadata xmlns="http://music.apple.com/lyric-ttml-internal"><songwriters><songwriter>Abel Tesfaye</songwriter><songwriter>Max Martin</songwriter></songwriters></iTunesMetadata><amll:meta key="musicName" value="Blinding Lights"/><amll:meta key="artists" value="The Weeknd"/><amll:meta key="album" value="After Hours"/><amll:meta key="spotifyId" value="0VjIjW4GlUZAMYd2vXMi3b"/></metadata></head><body dur="3:14.571"><div begin="27.173" end="3:14.571"><p begin="27.173" end="28.516" itunes:key="L1" ttm:agent="v1"><span begin="27.173" end="27.407">I&apos;ve</span> <span begin="27.407" end="27.510">been</span> <span begin="27.510" end="27.899">tryna</span> <span begin="27.899" end="28.516">call</span><span ttm:role="x-translation" xml:lang="zh-CN">我一直心存向往</span></p><p begin="29.988" end="32.075" itunes:key="L2" ttm:agent="v1"><span begin="29.988" end="30.117">I&apos;ve</span> <span begin="30.117" end="30.236">been</span></p></div></body></tt>"#;

    #[test]
    fn reads_a_clock() {
        assert_eq!(clock_of("27.173"), Some(Duration::from_millis(27_173)));
        assert_eq!(clock_of("3:14.571"), Some(Duration::from_millis(194_571)));
        assert_eq!(clock_of("1:00:00"), Some(Duration::from_secs(3600)));
        assert_eq!(clock_of("bogus"), None);
    }

    #[test]
    fn parses_a_worded_sheet() {
        let hit = hit(SAMPLE).expect("the sample parses");

        assert_eq!(hit.title, "Blinding Lights");
        assert_eq!(hit.artist, "The Weeknd");
        assert_eq!(hit.album.as_deref(), Some("After Hours"));
        assert_eq!(hit.duration, Some(Duration::from_millis(194_571)));
        assert_eq!(
            hit.writers,
            vec!["Abel Tesfaye".to_owned(), "Max Martin".to_owned()]
        );

        let Lyrics::Synced { lines } = &hit.lyrics else {
            unreachable!()
        };
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].text, "I've been tryna call");
        let words = lines[0].words.as_ref().expect("the line is worded");
        assert_eq!(words.len(), 4);
        assert_eq!(words[0].text, "I've");
        assert_eq!(words[3].end, Duration::from_millis(28_516));
    }

    #[test]
    fn a_translation_span_stays_out_of_the_text() {
        let hit = hit(SAMPLE).expect("the sample parses");
        let Lyrics::Synced { lines } = &hit.lyrics else {
            unreachable!()
        };
        assert!(!lines[0].text.contains('向'));
    }
}
