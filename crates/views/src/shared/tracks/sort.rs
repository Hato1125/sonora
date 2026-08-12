use std::cmp::Ordering;

use gpui::SharedString;
use music::Track;

use super::TrackField;

pub(super) fn compare(tracks: &[Track], field: TrackField, a: usize, b: usize) -> Ordering {
    let text = |index: usize, pick: fn(&Track) -> &String| {
        tracks
            .get(index)
            .map(|track| pick(track).to_lowercase())
            .unwrap_or_default()
    };

    match field {
        TrackField::Title => text(a, |track| &track.name).cmp(&text(b, |track| &track.name)),
        TrackField::Artists => {
            text(a, |track| &track.artists).cmp(&text(b, |track| &track.artists))
        }
        TrackField::Album => text(a, |track| &track.album).cmp(&text(b, |track| &track.album)),
        TrackField::AddedAt => tracks
            .get(a)
            .and_then(|track| track.added_at)
            .cmp(&tracks.get(b).and_then(|track| track.added_at)),
        TrackField::Plays => tracks
            .get(a)
            .and_then(|track| track.playcount)
            .cmp(&tracks.get(b).and_then(|track| track.playcount)),
        TrackField::Duration => tracks
            .get(a)
            .map(|track| track.duration)
            .cmp(&tracks.get(b).map(|track| track.duration)),
        TrackField::Index | TrackField::Cover => a.cmp(&b),
    }
}

pub(super) fn group(tracks: &[Track], field: TrackField, row: usize) -> Option<SharedString> {
    let track = tracks.get(row)?;

    match field {
        TrackField::Title => Some(initial(&track.name)),
        TrackField::Artists => Some(initial(&track.artists)),
        TrackField::Album => Some(initial(&track.album)),
        _ => None,
    }
}

pub(crate) fn initial(text: &str) -> SharedString {
    text.chars()
        .next()
        .filter(|first| first.is_alphabetic())
        .map(|first| SharedString::from(first.to_uppercase().collect::<String>()))
        .unwrap_or_else(|| SharedString::from("#"))
}

pub(super) fn hits(track: &Track, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    let haystack = format!("{} {} {}", track.name, track.artists, track.album);
    haystack.to_lowercase().contains(query)
}

#[cfg(test)]
mod tests {
    use super::initial;

    #[test]
    fn letters_bucket_under_their_uppercase_form() {
        assert_eq!(initial("bark at the moon"), "B");
        assert_eq!(initial("Bark at the Moon"), "B");
    }

    #[test]
    fn cyrillic_keeps_its_own_letter() {
        assert_eq!(initial("прощай"), "П");
        assert_eq!(initial("Ялта"), "Я");
    }

    #[test]
    fn digits_punctuation_and_emptiness_share_one_bucket() {
        assert_eq!(initial("99 Luftballons"), "#");
        assert_eq!(initial("!!!"), "#");
        assert_eq!(initial(" leading space"), "#");
        assert_eq!(initial(""), "#");
    }

    #[test]
    fn multi_char_uppercase_is_kept_whole() {
        assert_eq!(initial("ßeta"), "SS");
    }
}
