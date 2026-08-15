use std::time::Duration;

use crate::{LyricsHit, LyricsLine, LyricsQuery};

const CLOSE_ENOUGH: u64 = 3;

pub fn rank(query: &LyricsQuery, mut hits: Vec<LyricsHit>) -> Vec<LyricsHit> {
    hits.sort_by_key(|hit| std::cmp::Reverse(score(query, hit)));
    hits
}

fn score(query: &LyricsQuery, hit: &LyricsHit) -> u32 {
    let mut score = 0;
    if let Some(duration) = hit.duration {
        let drift = duration.as_secs().abs_diff(query.duration.as_secs());
        if drift <= CLOSE_ENOUGH {
            score += 100 - (drift as u32) * 10;
        }
    }
    if alike(&hit.title, &query.title) {
        score += 40;
    }
    if alike(&hit.artist, &query.artist) {
        score += 30;
    }
    if hit.lyrics.synced() {
        score += 20;
    }
    score
}

fn alike(left: &str, right: &str) -> bool {
    let trim = |text: &str| {
        text.to_lowercase()
            .chars()
            .filter(|letter| letter.is_alphanumeric())
            .collect::<String>()
    };
    let (left, right) = (trim(left), trim(right));
    !left.is_empty() && (left == right || left.contains(&right) || right.contains(&left))
}

pub fn active(lines: &[LyricsLine], at: Duration) -> Option<usize> {
    lines
        .iter()
        .rposition(|line| line.start <= at)
        .filter(|index| match lines[*index].end {
            Some(end) => at < end,
            None => true,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Lyrics;

    fn hit(title: &str, artist: &str, seconds: u64, synced: bool) -> LyricsHit {
        LyricsHit {
            source: "test",
            lyrics: match synced {
                true => Lyrics::Synced {
                    lines: Vec::new().into(),
                },
                false => Lyrics::Plain("la".to_owned()),
            },
            title: title.to_owned(),
            artist: artist.to_owned(),
            duration: Some(Duration::from_secs(seconds)),
        }
    }

    fn query() -> LyricsQuery {
        LyricsQuery {
            title: "Jaded".to_owned(),
            artist: "Spiritbox".to_owned(),
            album: None,
            duration: Duration::from_secs(263),
        }
    }

    #[test]
    fn the_closest_duration_wins() {
        let hits = vec![
            hit("Jaded", "Spiritbox", 200, true),
            hit("Jaded", "Spiritbox", 263, false),
        ];
        let ranked = rank(&query(), hits);
        assert_eq!(ranked[0].duration, Some(Duration::from_secs(263)));
    }

    #[test]
    fn synced_breaks_a_tie() {
        let hits = vec![
            hit("Jaded", "Spiritbox", 263, false),
            hit("Jaded", "Spiritbox", 263, true),
        ];
        let ranked = rank(&query(), hits);
        assert!(ranked[0].lyrics.synced());
    }

    #[test]
    fn a_wrong_track_falls_behind() {
        let hits = vec![
            hit("Something Else", "Nobody", 263, true),
            hit("Jaded", "Spiritbox", 261, false),
        ];
        let ranked = rank(&query(), hits);
        assert_eq!(ranked[0].title, "Jaded");
    }

    #[test]
    fn punctuation_does_not_break_a_match() {
        assert!(alike("Don't Stop", "dont stop"));
        assert!(alike("Jaded", "JADED"));
        assert!(!alike("Jaded", "Rotoscope"));
    }

    #[test]
    fn the_active_line_follows_the_clock() {
        let lines = vec![
            LyricsLine {
                start: Duration::from_secs(0),
                end: Some(Duration::from_secs(5)),
                text: "one".to_owned(),
                words: None,
            },
            LyricsLine {
                start: Duration::from_secs(5),
                end: Some(Duration::from_secs(9)),
                text: "two".to_owned(),
                words: None,
            },
        ];
        assert_eq!(active(&lines, Duration::from_secs(2)), Some(0));
        assert_eq!(active(&lines, Duration::from_secs(6)), Some(1));
        assert_eq!(active(&lines, Duration::from_secs(30)), None);
    }
}
