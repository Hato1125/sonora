use crate::LyricsLine;

const LABELS: &[&str] = &[
    "artist:",
    "title:",
    "album:",
    "song:",
    "by:",
    "lyrics:",
    "written by",
    "composed by",
    "arranged by",
    "produced by",
    "lyrics by",
    "music by",
    "mixed by",
    "作词",
    "作曲",
    "编曲",
    "作詞",
    "編曲",
    "制作",
];

const QUIET: &[&str] = &["纯音乐", "此歌曲为没有填词的纯音乐"];

pub(crate) fn instrumental(lines: &[LyricsLine]) -> bool {
    !lines.is_empty()
        && lines.iter().all(|line| {
            line.text.trim().is_empty() || QUIET.iter().any(|mark| line.text.contains(mark))
        })
}

pub(crate) fn headed(lines: &mut Vec<LyricsLine>, title: &str, artists: &[String]) -> bool {
    loop {
        while lines.first().is_some_and(|line| labelled(&line.text)) {
            lines.remove(0);
        }
        let Some(first) = lines.first().map(|line| line.text.clone()) else {
            return true;
        };
        let named = artists
            .iter()
            .any(|artist| artist.len() > 3 && loosely(&first, artist));
        if !named {
            return true;
        }
        let claimed = first
            .split(['-', '–', '—'])
            .map(str::trim)
            .filter(|part| !part.is_empty() && !artists.iter().any(|artist| loosely(part, artist)))
            .max_by_key(|part| part.len());
        if let Some(claimed) = claimed
            && !related(claimed, title)
        {
            return false;
        }
        lines.remove(0);
    }
}

fn related(claimed: &str, name: &str) -> bool {
    let words = |text: &str| {
        text.split(|letter: char| !letter.is_alphanumeric())
            .map(|word| word.to_lowercase())
            .filter(|word| word.chars().count() >= 4)
            .collect::<Vec<_>>()
    };
    let (left, right) = (words(claimed), words(name));
    if left.is_empty() || right.is_empty() {
        return true;
    }
    left.iter().any(|left| {
        right
            .iter()
            .any(|right| left.starts_with(right.as_str()) || right.starts_with(left.as_str()))
    })
}

fn labelled(text: &str) -> bool {
    let text = text.trim().to_ascii_lowercase();
    LABELS.iter().any(|label| text.starts_with(label))
}

fn loosely(haystack: &str, needle: &str) -> bool {
    let plain = |text: &str| {
        text.chars()
            .filter(|letter| letter.is_alphanumeric())
            .flat_map(char::to_lowercase)
            .collect::<String>()
    };
    let needle = plain(needle);
    !needle.is_empty() && plain(haystack).contains(&needle)
}
