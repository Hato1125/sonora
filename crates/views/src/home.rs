use std::collections::HashSet;
use std::rc::Rc;

use gpui::prelude::*;
use gpui::{App, Context, Entity, Render, Window, div};
use spotify::Track;
use state::{Library, LibraryState, Playback};
use ui::ActiveTheme as _;
use workspace::Sidebar;

use crate::quick_picks::{QuickPicks, column_count, page_count};
use crate::tracks::{PlaybackStatus, playback_status};

const GROUP_SIZE: usize = 12;
const LIMIT: usize = GROUP_SIZE * 3;

pub(crate) struct HomeView {
    playback: Entity<Playback>,
    playback_status: PlaybackStatus,
    quick_picks: Rc<Vec<Track>>,
    quick_picks_columns: usize,
    quick_picks_page: usize,
    quick_picks_seed: u64,
    sidebar: Entity<Sidebar>,
}

impl HomeView {
    pub(crate) fn new(
        library: Entity<Library>,
        playback: Entity<Playback>,
        sidebar: Entity<Sidebar>,
        cx: &mut Context<Self>,
    ) -> Self {
        let quick_picks_seed = session_seed();
        let quick_picks = picks(&library, quick_picks_seed, cx);

        cx.observe(&library, |this, library, cx| {
            this.quick_picks = picks(&library, this.quick_picks_seed, cx);
            this.quick_picks_page = 0;
            cx.notify();
        })
        .detach();
        cx.observe(&sidebar, |_, _, cx| cx.notify()).detach();

        let current_playback = playback_status(&playback, cx);
        cx.observe(&playback, |this, playback, cx| {
            let current = playback_status(&playback, cx);
            if this.playback_status != current {
                this.playback_status = current;
                cx.notify();
            }
        })
        .detach();

        Self {
            playback,
            playback_status: current_playback,
            quick_picks,
            quick_picks_columns: 0,
            quick_picks_page: 0,
            quick_picks_seed,
            sidebar,
        }
    }
}

impl Render for HomeView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.theme();
        let available = window.viewport_size().width
            - self.sidebar.read(cx).occupied_width()
            - theme.metrics.inset * 2.;
        let columns = column_count(available);
        if self.quick_picks_columns != columns {
            self.quick_picks_columns = columns;
            self.quick_picks_page = 0;
        }

        let pages = page_count(self.quick_picks.len(), available);
        self.quick_picks_page = self.quick_picks_page.min(pages.saturating_sub(1));
        let page = self.quick_picks_page;

        div()
            .id("home-page")
            .size_full()
            .overflow_y_scroll()
            .p(theme.metrics.inset)
            .child(
                div().flex().flex_col().gap_6().child(
                    QuickPicks::new(
                        self.quick_picks.clone(),
                        self.playback.clone(),
                        self.playback_status.0.clone(),
                        available,
                        page,
                    )
                    .on_previous(cx.listener(|this, _, _, cx| {
                        this.quick_picks_page = this.quick_picks_page.saturating_sub(1);
                        cx.notify();
                    }))
                    .on_next(cx.listener(move |this, _, _, cx| {
                        this.quick_picks_page =
                            (this.quick_picks_page + 1).min(pages.saturating_sub(1));
                        cx.notify();
                    })),
                ),
            )
    }
}

fn picks(library: &Entity<Library>, seed: u64, cx: &App) -> Rc<Vec<Track>> {
    let tracks = match library.read(cx).state() {
        LibraryState::Ready { tracks, .. } => mixed_tracks(tracks, seed),
        _ => Vec::new(),
    };
    Rc::new(tracks)
}

fn mixed_tracks(tracks: &[Track], seed: u64) -> Vec<Track> {
    let mut random = fastrand::Rng::with_seed(seed);
    let mut selected = tracks
        .iter()
        .enumerate()
        .filter(|(_, track)| track.playable && track.id.is_some())
        .map(|(index, _)| index)
        .take(GROUP_SIZE)
        .collect::<Vec<_>>();
    let recent = selected.iter().copied().collect::<HashSet<_>>();
    let mut remaining = tracks
        .iter()
        .enumerate()
        .filter(|(index, track)| track.playable && track.id.is_some() && !recent.contains(index))
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    random.shuffle(&mut remaining);

    let random_count = GROUP_SIZE.min(remaining.len());
    selected.extend(remaining.drain(..random_count));

    let mut artists = selected
        .iter()
        .map(|index| artist_key(&tracks[*index]))
        .collect::<HashSet<_>>();
    let mut fallback = Vec::new();
    let mut diverse_count = 0;
    for index in remaining {
        if diverse_count < GROUP_SIZE && artists.insert(artist_key(&tracks[index])) {
            selected.push(index);
            diverse_count += 1;
        } else {
            fallback.push(index);
        }
    }

    selected.extend(fallback.into_iter().take(LIMIT - selected.len()));
    let mut mixed = selected
        .into_iter()
        .map(|index| tracks[index].clone())
        .collect::<Vec<_>>();
    random.shuffle(&mut mixed);
    mixed
}

fn artist_key(track: &Track) -> &str {
    track
        .artist_refs
        .first()
        .map(|artist| artist.id.as_deref().unwrap_or(&artist.name))
        .unwrap_or(&track.artists)
}

fn session_seed() -> u64 {
    fastrand::u64(..)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::time::Duration;

    use spotify::{ArtistRef, Track};

    use super::{GROUP_SIZE, LIMIT, mixed_tracks};

    fn track(index: usize, artist: usize, playable: bool) -> Track {
        Track {
            id: Some(format!("track-{index}")),
            name: format!("Track {index}"),
            playable,
            artists: format!("Artist {artist}"),
            artist_refs: vec![ArtistRef {
                name: format!("Artist {artist}"),
                id: Some(format!("artist-{artist}")),
            }],
            album: String::new(),
            album_id: None,
            cover: None,
            duration: Duration::from_secs(180),
            popularity: 0,
            explicit: false,
        }
    }

    #[test]
    fn mixed_selection_is_stable_and_has_no_duplicates() {
        let tracks = (0..60)
            .map(|index| track(index, index, true))
            .collect::<Vec<_>>();

        let first = mixed_tracks(&tracks, 42);
        let second = mixed_tracks(&tracks, 42);
        let ids = first
            .iter()
            .filter_map(|track| track.id.as_ref())
            .collect::<HashSet<_>>();

        assert_eq!(first, second);
        assert_eq!(first.len(), LIMIT);
        assert_eq!(ids.len(), LIMIT);
        for index in 0..GROUP_SIZE {
            let expected = format!("track-{index}");
            assert!(
                first
                    .iter()
                    .any(|track| track.id.as_deref() == Some(expected.as_str()))
            );
        }
    }

    #[test]
    fn mixed_selection_excludes_unavailable_tracks() {
        let tracks = (0..50)
            .map(|index| track(index, index, index % 2 == 0))
            .collect::<Vec<_>>();

        let selected = mixed_tracks(&tracks, 7);

        assert_eq!(selected.len(), 25);
        assert!(selected.iter().all(|track| track.playable));
    }

    #[test]
    fn mixed_selection_adds_artist_variety() {
        let tracks = (0..64)
            .map(|index| track(index, index.saturating_sub(23), true))
            .collect::<Vec<_>>();

        let selected = mixed_tracks(&tracks, 99);
        let artists = selected
            .iter()
            .map(|track| &track.artists)
            .collect::<HashSet<_>>();

        assert!(artists.len() > GROUP_SIZE);
    }
}
