use gpui::prelude::FluentBuilder as _;
use gpui::{App, ElementId, Entity, FontWeight, SharedString};
use music::{Album, Playlist, SavedArtist};
use router::{Destination, navigate};
use state::{Origin, Playback, PlaybackState};
use ui::{ActiveTheme as _, Card, Pinnable, Text};

use crate::shared::cells;
use crate::shared::pins::Pinned as _;

pub(crate) fn album_card(
    id: impl Into<ElementId>,
    album: &Album,
    playback: &Entity<Playback>,
    cx: &App,
) -> Card {
    let theme = *cx.theme();
    let cover = album.cover_large.clone().or_else(|| album.cover.clone());
    let origin = Origin::album(album.id.clone()).named(album.name.clone());
    let playing = matches!(
        playback.read(cx).playing_from(&origin),
        Some(PlaybackState::Playing)
    );
    let pin = album.pin();
    let opened = SharedString::from(album.id.clone());
    let toggled = playback.clone();
    let artists = cells::artist_links(
        SharedString::new_static("album-card-artist"),
        album.artist_refs.clone(),
        album.artists.clone(),
        theme.muted_foreground,
    )
    .text_size(theme.text(Text::Small))
    .truncate();

    Card::new(id, SharedString::from(album.name.clone()))
        .cover(cover)
        .weight(FontWeight::SEMIBOLD)
        .underline()
        .hint()
        .bare_meta(artists)
        .play(playing, move |_, _, cx| {
            toggled.update(cx, |playback, cx| playback.toggle_origin(&origin, cx));
        })
        .press(move |_, _, cx| navigate(Destination::Album(opened.clone()), cx))
        .when_some(pin, Pinnable::pin)
}

pub(crate) fn playlist_card(
    id: impl Into<ElementId>,
    playlist: &Playlist,
    playback: &Entity<Playback>,
    cx: &App,
) -> Card {
    let origin = Origin::playlist(playlist.id.clone()).named(playlist.name.clone());
    let playing = matches!(
        playback.read(cx).playing_from(&origin),
        Some(PlaybackState::Playing)
    );
    let pin = playlist.pin();
    let opened = SharedString::from(playlist.id.clone());
    let toggled = playback.clone();

    Card::new(id, SharedString::from(playlist.name.clone()))
        .cover(playlist.cover.clone())
        .weight(FontWeight::SEMIBOLD)
        .underline()
        .meta(SharedString::from(playlist.owner.clone()))
        .play(playing, move |_, _, cx| {
            toggled.update(cx, |playback, cx| playback.toggle_origin(&origin, cx));
        })
        .press(move |_, _, cx| navigate(Destination::Playlist(opened.clone()), cx))
        .when_some(pin, Pinnable::pin)
}

pub(crate) fn artist_card(
    id: impl Into<ElementId>,
    artist: &SavedArtist,
    playback: &Entity<Playback>,
    cx: &App,
) -> Card {
    let origin = Origin::artist(artist.id.clone()).named(artist.name.clone());
    let playing = matches!(
        playback.read(cx).playing_from(&origin),
        Some(PlaybackState::Playing)
    );
    let pin = artist.pin();
    let opened = SharedString::from(artist.id.clone());
    let toggled = playback.clone();

    Card::new(id, SharedString::from(artist.name.clone()))
        .cover(artist.cover.clone())
        .circle()
        .weight(FontWeight::SEMIBOLD)
        .underline()
        .meta(i18n::lookup("artist-eyebrow", None))
        .play(playing, move |_, _, cx| {
            toggled.update(cx, |playback, cx| playback.toggle_origin(&origin, cx));
        })
        .press(move |_, _, cx| navigate(Destination::Artist(opened.clone()), cx))
        .when_some(pin, Pinnable::pin)
}
