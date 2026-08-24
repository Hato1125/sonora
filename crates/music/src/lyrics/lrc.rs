use std::time::Duration;

use crate::{LyricsLine, LyricsWord};

pub fn parse(lrc: &str) -> Vec<LyricsLine> {
    let mut lines: Vec<LyricsLine> = lrc.lines().flat_map(read).collect();
    lines.sort_by_key(|line| line.start);
    close(&mut lines);
    lines
}

pub fn close(lines: &mut [LyricsLine]) {
    for index in 0..lines.len().saturating_sub(1) {
        let next = lines[index + 1].start;
        if lines[index].end.is_none() {
            lines[index].end = Some(next);
        }
    }
    for line in lines.iter_mut() {
        let Some(end) = line.end else { continue };
        let Some(words) = line.words.as_mut() else {
            continue;
        };
        if let Some(last) = words.last_mut()
            && last.end <= last.start
        {
            last.end = end.max(last.start);
        }
    }
}

pub fn stamp_of(stamp: &str) -> Option<Duration> {
    let (minutes, rest) = stamp.split_once(':')?;
    let minutes: u64 = minutes.trim().parse().ok()?;
    let seconds: f64 = rest.replace(',', ".").parse().ok()?;
    if !seconds.is_finite() || seconds < 0. {
        return None;
    }
    Some(Duration::from_secs_f64(minutes as f64 * 60. + seconds))
}

struct Segment {
    at: Option<Duration>,
    text: String,
}

fn read(line: &str) -> Vec<LyricsLine> {
    let mut rest = line.trim();
    let mut stamps = Vec::new();
    while let Some(body) = rest.strip_prefix('[') {
        let Some((stamp, tail)) = body.split_once(']') else {
            break;
        };
        let Some(at) = stamp_of(stamp) else { break };
        stamps.push(at);
        rest = tail.trim_start();
    }

    let (text, words) = spoken(rest);
    stamps
        .into_iter()
        .map(|start| LyricsLine {
            start,
            end: None,
            text: text.clone(),
            words: words.clone().map(|words| shifted(words, start)),
        })
        .collect()
}

fn shifted(words: Vec<LyricsWord>, start: Duration) -> Vec<LyricsWord> {
    let Some(first) = words.first() else {
        return words;
    };
    let Some(drift) = start.checked_sub(first.start) else {
        return words;
    };
    match drift.is_zero() {
        true => words,
        false => words
            .into_iter()
            .map(|word| LyricsWord {
                start: word.start + drift,
                end: word.end + drift,
                text: word.text,
            })
            .collect(),
    }
}

fn spoken(body: &str) -> (String, Option<Vec<LyricsWord>>) {
    let segments = cut(body);
    let whole: String = segments
        .iter()
        .map(|segment| segment.text.as_str())
        .collect();
    let timed: Vec<&Segment> = segments
        .iter()
        .filter(|segment| segment.at.is_some())
        .collect();
    if timed.is_empty() {
        return (whole.trim().to_owned(), None);
    }

    let mut words = Vec::new();
    for (index, segment) in timed.iter().enumerate() {
        let start = segment.at.expect("a timed segment carries a stamp");
        if segment.text.is_empty() {
            if let Some(last) = words.last_mut() {
                let last: &mut LyricsWord = last;
                last.end = start.max(last.start);
            }
            continue;
        }
        let end = timed
            .get(index + 1)
            .and_then(|next| next.at)
            .filter(|next| *next > start)
            .unwrap_or(start);
        words.push(LyricsWord {
            start,
            end,
            text: segment.text.clone(),
        });
    }

    let whole = whole.trim_end().to_owned();
    (whole, (!words.is_empty()).then_some(words))
}

fn cut(body: &str) -> Vec<Segment> {
    let mut segments = vec![Segment {
        at: None,
        text: String::new(),
    }];
    let mut rest = body;
    while let Some(open) = rest.find('<') {
        let tail = &rest[open + 1..];
        let Some(shut) = tail.find('>') else { break };
        let last = segments.last_mut().expect("a segment is always open");
        match stamp_of(&tail[..shut]) {
            Some(at) => {
                last.text.push_str(&rest[..open]);
                segments.push(Segment {
                    at: Some(at),
                    text: String::new(),
                });
            }
            None => last.text.push_str(&rest[..open + shut + 2]),
        }
        rest = &tail[shut + 1..];
    }
    segments
        .last_mut()
        .expect("a segment is always open")
        .text
        .push_str(rest);
    segments
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
    fn reads_word_tags() {
        let lines = parse("[00:12.50]<00:12.50>I <00:12.80>see <00:13.10>trees\n[00:15.00]next");

        let words = lines[0].words.as_ref().expect("the line is worded");
        assert_eq!(words.len(), 3);
        assert_eq!(words[0].text, "I ");
        assert_eq!(words[1].start, Duration::from_millis(12_800));
        assert_eq!(words[2].end, Duration::from_secs(15));
        assert_eq!(lines[0].text, "I see trees");
    }

    #[test]
    fn a_bare_end_tag_closes_the_last_word() {
        let lines = parse("[00:01.00]<00:01.00>one <00:01.50>two<00:02.00>");

        let words = lines[0].words.as_ref().expect("the line is worded");
        assert_eq!(words.len(), 2);
        assert_eq!(words[1].end, Duration::from_secs(2));
        assert_eq!(lines[0].text, "one two");
    }

    #[test]
    fn angle_brackets_that_are_not_stamps_stay_in_the_text() {
        let lines = parse("[00:01.00]a <3 b");

        assert_eq!(lines[0].text, "a <3 b");
        assert!(lines[0].words.is_none());
    }

    #[test]
    fn one_line_can_carry_several_stamps() {
        let lines = parse("[00:01.00][00:31.00]chorus");

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].start, Duration::from_secs(1));
        assert_eq!(lines[1].start, Duration::from_secs(31));
        assert_eq!(lines[1].text, "chorus");
    }

    #[test]
    fn a_repeated_worded_line_moves_its_words_along() {
        let lines = parse("[00:01.00][00:31.00]<00:01.00>one <00:01.50>two");

        let words = lines[1].words.as_ref().expect("the line is worded");
        assert_eq!(words[0].start, Duration::from_secs(31));
        assert_eq!(words[1].start, Duration::from_millis(31_500));
    }
}
