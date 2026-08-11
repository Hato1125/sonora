// SPDX-License-Identifier: GPL-3.0-or-later

use gpui::{App, ClipboardItem, Entity, Styled as _};
use i18n::t;
use music::{Album, MediaKind, Playlist, Track};
use router::{Destination, navigate};
use state::{Detail, LibraryState, Playback, Sonora};
use ui::{Menu, MenuItem, Pin, PinKind, Scrollbar, SubmenuState};

use crate::shared::playlist_editor::{Edit, PlaylistEditor};

#[derive(Clone, Copy, Default)]
pub(crate) struct TrackColumns {
    pub album: bool,
    pub artists: bool,
}

#[derive(Clone)]
pub(crate) struct ItemMenu {
    playlist_submenu: SubmenuState,
    artist_submenu: SubmenuState,
    playlist_scrollbar: Entity<Scrollbar>,
}

impl ItemMenu {
    pub fn new(playlist_scrollbar: Entity<Scrollbar>) -> Self {
        Self {
            playlist_submenu: SubmenuState::default(),
            artist_submenu: SubmenuState::default(),
            playlist_scrollbar,
        }
    }

    pub fn reset(&self, cx: &App) {
        self.playlist_submenu.reset();
        self.artist_submenu.reset();
        self.playlist_scrollbar
            .read(cx)
            .scroll()
            .set_offset(gpui::Point::default());
    }

    pub fn for_track(&self, track: &Track, cx: &App) -> Menu {
        self.build(track, None, None, TrackColumns::default(), cx)
    }

    pub fn for_table_track(&self, track: &Track, columns: TrackColumns, cx: &App) -> Menu {
        self.build(track, None, None, columns, cx)
    }

    pub fn for_album_track(
        &self,
        track: &Track,
        album_id: &str,
        columns: TrackColumns,
        cx: &App,
    ) -> Menu {
        self.build(track, None, Some(album_id), columns, cx)
    }

    pub fn for_playlist_track(
        &self,
        track: &Track,
        detail: Entity<Detail>,
        columns: TrackColumns,
        cx: &App,
    ) -> Menu {
        let remove = match track.id.clone() {
            Some(id) => MenuItem::new("remove-from-playlist", t!("menu-remove-from-playlist"))
                .icon("icons/x.svg")
                .on_click(move |_, _, cx| {
                    detail.update(cx, |detail, cx| detail.remove_from_playlist(id.clone(), cx));
                }),
            None => MenuItem::new("remove-from-playlist", t!("menu-remove-from-playlist"))
                .icon("icons/x.svg")
                .disabled(),
        };
        self.build(track, Some(remove), None, columns, cx)
    }

    fn build(
        &self,
        track: &Track,
        library_action: Option<MenuItem>,
        current_album: Option<&str>,
        columns: TrackColumns,
        cx: &App,
    ) -> Menu {
        let library = Sonora::global(cx).library.clone();
        let playlists = match library.read(cx).state() {
            LibraryState::Ready { playlists, .. } => playlists
                .iter()
                .filter(|playlist| playlist.owned || playlist.collaborative)
                .cloned()
                .collect(),
            _ => Vec::new(),
        };
        let created = track.id.clone();
        let new_playlist = MenuItem::new("new-playlist", t!("menu-new-playlist"))
            .icon("icons/plus.svg")
            .on_click(move |_, window, cx| {
                PlaylistEditor::open(Edit::Create(created.clone()), window, cx);
            });
        let playlist_menu = if playlists.is_empty() {
            Menu::new("playlist-submenu")
                .w(gpui::px(220.))
                .item(new_playlist)
                .item(MenuItem::separator("playlist-separator"))
                .item(MenuItem::new("no-playlists", t!("menu-no-playlists")).disabled())
        } else {
            Menu::new("playlist-submenu")
                .w(gpui::px(220.))
                .max_h(gpui::px(360.))
                .scrollbar(self.playlist_scrollbar.clone())
                .item(new_playlist)
                .item(MenuItem::separator("playlist-separator"))
                .items(playlists.into_iter().map(|playlist| {
                    let held = track
                        .id
                        .as_deref()
                        .and_then(|id| library.read(cx).holds(&playlist.id, id))
                        .unwrap_or(false);
                    let item =
                        MenuItem::new(format!("playlist-{}", playlist.id), playlist.name.clone())
                            .artwork(playlist.cover.clone())
                            .checked(held);
                    match track.id.clone() {
                        Some(track_id) => {
                            let library = library.clone();
                            let playlist_id = playlist.id.clone();
                            item.on_click(move |_, window, cx| match held {
                                true => PlaylistEditor::open(
                                    Edit::Again {
                                        playlist: playlist.clone(),
                                        track: track_id.clone(),
                                    },
                                    window,
                                    cx,
                                ),
                                false => {
                                    library.update(cx, |library, cx| {
                                        library.add_to_playlist(
                                            playlist_id.clone(),
                                            track_id.clone(),
                                            cx,
                                        )
                                    });
                                }
                            })
                        }
                        None => item.disabled(),
                    }
                }))
        };
        let copy = match track.id.clone() {
            Some(id) => MenuItem::new("copy-track-link", t!("menu-copy-link"))
                .icon("icons/link.svg")
                .on_click(move |_, _, cx| copy_link(MediaKind::Track, &id, cx)),
            None => MenuItem::new("copy-track-link", t!("menu-copy-link"))
                .icon("icons/link.svg")
                .disabled(),
        };
        let next = match track.playable {
            true => {
                let track = track.clone();
                MenuItem::new("play-next", t!("menu-play-next"))
                    .icon("icons/list-plus.svg")
                    .on_click(move |_, _, cx| {
                        let playback = Sonora::global(cx).playback.clone();
                        playback.update(cx, |playback, cx| playback.play_next(track.clone(), cx));
                    })
            }
            false => MenuItem::new("play-next", t!("menu-play-next"))
                .icon("icons/list-plus.svg")
                .disabled(),
        };
        let queue = match track.playable {
            true => {
                let track = track.clone();
                MenuItem::new("add-to-queue", t!("menu-add-to-queue"))
                    .icon("icons/list-end.svg")
                    .on_click(move |_, _, cx| {
                        let playback = Sonora::global(cx).playback.clone();
                        playback.update(cx, |playback, cx| playback.enqueue(track.clone(), cx));
                    })
            }
            false => MenuItem::new("add-to-queue", t!("menu-add-to-queue"))
                .icon("icons/list-end.svg")
                .disabled(),
        };
        let radio = match track.id.is_some() && track.playable {
            true => {
                let track = track.clone();
                MenuItem::new("song-radio", t!("menu-song-radio"))
                    .icon("icons/radio.svg")
                    .on_click(move |_, _, cx| {
                        let playback = Sonora::global(cx).playback.clone();
                        playback.update(cx, |playback, cx| playback.play_radio(&track, cx));
                    })
            }
            false => MenuItem::new("song-radio", t!("menu-song-radio"))
                .icon("icons/radio.svg")
                .disabled(),
        };
        let toggle_library = match track.id.as_deref() {
            Some(id) if !library.read(cx).pending(id) => {
                let saved = library.read(cx).saved(id);
                let track = track.clone();
                MenuItem::new(
                    "toggle-library",
                    match saved {
                        true => t!("menu-remove-from-library"),
                        false => t!("menu-add-to-library"),
                    },
                )
                .icon(match saved {
                    true => "icons/heart-off.svg",
                    false => "icons/heart.svg",
                })
                .on_click(move |_, _, cx| {
                    library.update(cx, |library, cx| library.toggle(track.clone(), cx));
                })
            }
            _ => MenuItem::new("toggle-library", t!("menu-add-to-library"))
                .icon("icons/heart.svg")
                .disabled(),
        };

        let album = match (columns.album, track.album_id.clone()) {
            (true, _) => None,
            (false, Some(id)) if Some(id.as_str()) == current_album => None,
            (false, Some(id)) => Some(
                MenuItem::new("go-to-album", t!("menu-go-to-album"))
                    .icon("icons/disc-3.svg")
                    .on_click(move |_, _, cx| navigate(Destination::Album(id.clone().into()), cx)),
            ),
            (false, None) => Some(
                MenuItem::new("go-to-album", t!("menu-go-to-album"))
                    .icon("icons/disc-3.svg")
                    .disabled(),
            ),
        };

        let artists = track
            .artist_refs
            .iter()
            .filter_map(|artist| {
                let id = artist.id.clone()?;
                Some((artist.name.clone(), id))
            })
            .collect::<Vec<_>>();
        let artist = match (columns.artists, artists.len()) {
            (true, _) => None,
            (false, 0) => Some(
                MenuItem::new("go-to-artist", t!("menu-go-to-artist"))
                    .icon("icons/user.svg")
                    .disabled(),
            ),
            (false, 1) => {
                let id = artists[0].1.clone();
                Some(
                    MenuItem::new("go-to-artist", t!("menu-go-to-artist"))
                        .icon("icons/user.svg")
                        .on_click(move |_, _, cx| {
                            navigate(Destination::Artist(id.clone().into()), cx)
                        }),
                )
            }
            (false, _) => {
                let artist_menu = Menu::new("artist-submenu")
                    .w(gpui::px(220.))
                    .max_h(gpui::px(360.))
                    .items(artists.into_iter().map(|(name, id)| {
                        MenuItem::new(format!("artist-{id}"), name).on_click(move |_, _, cx| {
                            navigate(Destination::Artist(id.clone().into()), cx)
                        })
                    }));
                Some(
                    MenuItem::new("go-to-artist", t!("menu-go-to-artist"))
                        .icon("icons/user.svg")
                        .submenu(artist_menu, self.artist_submenu.clone()),
                )
            }
        };

        let details = match track.id.clone() {
            Some(id) => MenuItem::new("view-details", t!("menu-view-details"))
                .icon("icons/info.svg")
                .on_click(move |_, _, cx| navigate(Destination::Song(id.clone().into()), cx)),
            None => MenuItem::new("view-details", t!("menu-view-details"))
                .icon("icons/info.svg")
                .disabled(),
        };

        sections(
            Menu::new("track-context-menu").relative().w(gpui::px(210.)),
            vec![
                vec![
                    MenuItem::new("add-to-playlist", t!("menu-add-to-playlist"))
                        .icon("icons/list-plus.svg")
                        .submenu(playlist_menu, self.playlist_submenu.clone()),
                    library_action.unwrap_or(toggle_library),
                ],
                vec![next, queue, radio],
                album.into_iter().chain(artist).collect(),
                vec![details, copy],
            ],
        )
    }
}

fn sections(menu: Menu, groups: Vec<Vec<MenuItem>>) -> Menu {
    groups
        .into_iter()
        .filter(|group| !group.is_empty())
        .enumerate()
        .fold(menu, |menu, (index, group)| {
            match index {
                0 => menu,
                _ => menu.item(MenuItem::separator(format!("section-{index}"))),
            }
            .items(group)
        })
}

pub(crate) fn album_menu(
    album: Album,
    playback: Entity<Playback>,
    opened_here: bool,
    cx: &App,
) -> Menu {
    let album_id = album.id.clone();
    let opened = album_id.clone();
    let played = album_id.clone();
    let next = album_id.clone();
    let queued = album_id.clone();
    let copied = album_id.clone();
    let playing = playback.clone();
    let nexting = playback.clone();
    let queueing = playback;

    let open = match opened_here {
        true => Vec::new(),
        false => vec![
            MenuItem::new("open-album", t!("menu-open-album"))
                .icon("icons/info.svg")
                .on_click(move |_, _, cx| navigate(Destination::Album(opened.clone().into()), cx)),
        ],
    };

    sections(
        Menu::new("album-context-menu"),
        vec![
            open,
            vec![
                MenuItem::new("play-album", t!("menu-play-album"))
                    .icon("icons/play.svg")
                    .on_click(move |_, _, cx| {
                        playing.update(cx, |playback, cx| playback.play_album(&played, cx));
                    }),
                MenuItem::new("play-album-next", t!("menu-play-next"))
                    .icon("icons/list-plus.svg")
                    .on_click(move |_, _, cx| {
                        nexting.update(cx, |playback, cx| playback.play_album_next(&next, cx));
                    }),
                MenuItem::new("enqueue-album", t!("menu-add-album-to-queue"))
                    .icon("icons/list-end.svg")
                    .on_click(move |_, _, cx| {
                        queueing.update(cx, |playback, cx| playback.enqueue_album(&queued, cx));
                    }),
            ],
            vec![album_library_item(album, cx)],
            vec![
                MenuItem::new("copy-album-link", t!("menu-copy-link"))
                    .icon("icons/link.svg")
                    .on_click(move |_, _, cx| copy_link(MediaKind::Album, &copied, cx)),
            ],
        ],
    )
}

fn album_library_item(album: Album, cx: &App) -> MenuItem {
    let library = Sonora::global(cx).library.clone();
    let saved = library.read(cx).saved_album(&album.id);
    let item = MenuItem::new(
        "toggle-album-library",
        match saved {
            true => t!("menu-remove-from-library"),
            false => t!("menu-add-to-library"),
        },
    )
    .icon(match saved {
        true => "icons/heart-off.svg",
        false => "icons/heart.svg",
    });

    match library.read(cx).pending_album(&album.id) {
        true => item.disabled(),
        false => item.on_click(move |_, _, cx| {
            library.update(cx, |library, cx| library.toggle_album(album.clone(), cx));
        }),
    }
}

pub(crate) fn artist_menu(artist_id: String) -> Menu {
    Menu::new("artist-context-menu").item(
        MenuItem::new("copy-artist-link", t!("menu-copy-link"))
            .icon("icons/link.svg")
            .on_click(move |_, _, cx| copy_link(MediaKind::Artist, &artist_id, cx)),
    )
}

pub(crate) fn playlist_menu(
    playlist: Playlist,
    playback: Entity<Playback>,
    opened_here: bool,
    cx: &App,
) -> Menu {
    let opened = playlist.id.clone();
    let played = playlist.id.clone();
    let next = playlist.id.clone();
    let queued = playlist.id.clone();
    let copied = playlist.id.clone();
    let playing = playback.clone();
    let nexting = playback.clone();
    let queueing = playback;
    let id = playlist.id.clone();
    let public = playlist.public;
    let actions = match playlist.owned {
        true => vec![
            MenuItem::new(
                "playlist-visibility",
                match public {
                    true => t!("menu-make-playlist-private"),
                    false => t!("menu-make-playlist-public"),
                },
            )
            .icon("icons/user.svg")
            .on_click({
                let id = id.clone();
                move |_, _, cx| {
                    let library = Sonora::global(cx).library.clone();
                    library.update(cx, |library, cx| {
                        library.set_playlist_public(id.clone(), !public, cx)
                    });
                }
            }),
            MenuItem::new("rename-playlist", t!("menu-rename-playlist"))
                .icon("icons/pencil.svg")
                .on_click({
                    let playlist = playlist.clone();
                    move |_, window, cx| {
                        PlaylistEditor::open(Edit::Rename(playlist.clone()), window, cx);
                    }
                }),
            MenuItem::new("delete-playlist", t!("menu-delete-playlist"))
                .icon("icons/trash-2.svg")
                .on_click(move |_, window, cx| {
                    PlaylistEditor::open(Edit::Delete(playlist.clone()), window, cx);
                }),
        ],
        false => vec![playlist_library_item(playlist.clone(), cx)],
    };

    let open = match opened_here {
        true => Vec::new(),
        false => vec![
            MenuItem::new("open-playlist", t!("menu-open-playlist"))
                .icon("icons/info.svg")
                .on_click(move |_, _, cx| {
                    navigate(Destination::Playlist(opened.clone().into()), cx)
                }),
        ],
    };

    sections(
        Menu::new("playlist-context-menu"),
        vec![
            open,
            vec![
                MenuItem::new("play-playlist", t!("menu-play-playlist"))
                    .icon("icons/play.svg")
                    .on_click(move |_, _, cx| {
                        playing.update(cx, |playback, cx| playback.play_playlist(&played, cx));
                    }),
                MenuItem::new("play-playlist-next", t!("menu-play-next"))
                    .icon("icons/list-plus.svg")
                    .on_click(move |_, _, cx| {
                        nexting.update(cx, |playback, cx| playback.play_playlist_next(&next, cx));
                    }),
                MenuItem::new("enqueue-playlist", t!("menu-add-to-queue"))
                    .icon("icons/list-end.svg")
                    .on_click(move |_, _, cx| {
                        queueing.update(cx, |playback, cx| playback.enqueue_playlist(&queued, cx));
                    }),
            ],
            actions,
            vec![
                MenuItem::new("copy-playlist-link", t!("menu-copy-link"))
                    .icon("icons/link.svg")
                    .on_click(move |_, _, cx| copy_link(MediaKind::Playlist, &copied, cx)),
            ],
        ],
    )
}

pub(crate) fn pin_menu(pin: &Pin, tracks: &ItemMenu, playback: Entity<Playback>, cx: &App) -> Menu {
    let library = Sonora::global(cx).library.clone();
    let built = match pin.kind {
        PinKind::Album => library
            .read(cx)
            .album(&pin.id)
            .cloned()
            .map(|album| album_menu(album, playback.clone(), false, cx)),
        PinKind::Playlist => library
            .read(cx)
            .playlist(&pin.id)
            .cloned()
            .map(|playlist| playlist_menu(playlist, playback.clone(), false, cx)),
        PinKind::Artist => Some(artist_menu(pin.id.clone())),
        PinKind::Song => saved_track(&pin.id, cx).map(|track| tracks.for_track(&track, cx)),
    };

    let menu = built.unwrap_or_else(|| sparse_pin_menu(pin, playback));

    menu.item(MenuItem::separator("pin-separator"))
        .item(unpin_item(pin))
}

fn sparse_pin_menu(pin: &Pin, playback: Entity<Playback>) -> Menu {
    let destination = Destination::from(pin);
    let copied = pin.id.clone();
    let kind = media_kind(pin.kind);
    let open = MenuItem::new("open-pin", i18n::lookup(open_key(pin.kind), None))
        .icon("icons/info.svg")
        .on_click(move |_, _, cx| navigate(destination.clone(), cx));

    sections(
        Menu::new("pin-context-menu"),
        vec![
            vec![open],
            transport_items(pin, playback),
            vec![
                MenuItem::new("copy-pin-link", t!("menu-copy-link"))
                    .icon("icons/link.svg")
                    .on_click(move |_, _, cx| copy_link(kind, &copied, cx)),
            ],
        ],
    )
}

fn open_key(kind: PinKind) -> &'static str {
    match kind {
        PinKind::Album => "menu-open-album",
        PinKind::Artist => "menu-go-to-artist",
        PinKind::Playlist => "menu-open-playlist",
        PinKind::Song => "menu-view-details",
    }
}

fn transport_items(pin: &Pin, playback: Entity<Playback>) -> Vec<MenuItem> {
    let played = pin.id.clone();
    let next = pin.id.clone();
    let queued = pin.id.clone();
    let nexting = playback.clone();
    let queueing = playback.clone();

    match pin.kind {
        PinKind::Album => vec![
            MenuItem::new("play-pin", t!("menu-play-album"))
                .icon("icons/play.svg")
                .on_click(move |_, _, cx| {
                    playback.update(cx, |playback, cx| playback.play_album(&played, cx));
                }),
            MenuItem::new("play-pin-next", t!("menu-play-next"))
                .icon("icons/list-plus.svg")
                .on_click(move |_, _, cx| {
                    nexting.update(cx, |playback, cx| playback.play_album_next(&next, cx));
                }),
            MenuItem::new("enqueue-pin", t!("menu-add-album-to-queue"))
                .icon("icons/list-end.svg")
                .on_click(move |_, _, cx| {
                    queueing.update(cx, |playback, cx| playback.enqueue_album(&queued, cx));
                }),
        ],
        PinKind::Playlist => vec![
            MenuItem::new("play-pin", t!("menu-play-playlist"))
                .icon("icons/play.svg")
                .on_click(move |_, _, cx| {
                    playback.update(cx, |playback, cx| playback.play_playlist(&played, cx));
                }),
            MenuItem::new("play-pin-next", t!("menu-play-next"))
                .icon("icons/list-plus.svg")
                .on_click(move |_, _, cx| {
                    nexting.update(cx, |playback, cx| playback.play_playlist_next(&next, cx));
                }),
            MenuItem::new("enqueue-pin", t!("menu-add-to-queue"))
                .icon("icons/list-end.svg")
                .on_click(move |_, _, cx| {
                    queueing.update(cx, |playback, cx| playback.enqueue_playlist(&queued, cx));
                }),
        ],
        PinKind::Artist => vec![
            MenuItem::new("play-pin", t!("common-play"))
                .icon("icons/play.svg")
                .on_click(move |_, _, cx| {
                    playback.update(cx, |playback, cx| playback.play_artist(&played, cx));
                }),
        ],
        PinKind::Song => vec![
            MenuItem::new("play-pin", t!("menu-song-radio"))
                .icon("icons/radio.svg")
                .on_click(move |_, _, cx| {
                    playback.update(cx, |playback, cx| playback.play_track(&played, cx));
                }),
        ],
    }
}

fn unpin_item(pin: &Pin) -> MenuItem {
    let unpinned = pin.clone();

    MenuItem::new("unpin", t!("nav-unpin"))
        .icon("icons/x.svg")
        .on_click(move |_, _, cx| {
            Sonora::global(cx)
                .settings
                .clone()
                .update(cx, |settings, cx| settings.unpin(&unpinned, cx));
        })
}

fn media_kind(kind: PinKind) -> MediaKind {
    match kind {
        PinKind::Album => MediaKind::Album,
        PinKind::Artist => MediaKind::Artist,
        PinKind::Playlist => MediaKind::Playlist,
        PinKind::Song => MediaKind::Track,
    }
}

fn saved_track(id: &str, cx: &App) -> Option<Track> {
    let library = Sonora::global(cx).library.read(cx);
    let LibraryState::Ready { tracks, .. } = library.state() else {
        return None;
    };

    tracks
        .iter()
        .find(|track| track.id.as_deref() == Some(id))
        .cloned()
}

fn copy_link(kind: MediaKind, id: &str, cx: &mut App) {
    let session = Sonora::global(cx).session.read(cx);
    let client = match music::is_local_id(id) {
        true => session.local_client(),
        false => session.client(),
    };
    let Some(client) = client else {
        return;
    };
    let Some(url) = client.share_url(kind, id) else {
        return;
    };
    cx.write_to_clipboard(ClipboardItem::new_string(url));
}

fn playlist_library_item(playlist: Playlist, cx: &App) -> MenuItem {
    let library = Sonora::global(cx).library.clone();
    let saved = library.read(cx).playlist(&playlist.id).is_some();

    match saved {
        true => {
            let id = playlist.id;
            MenuItem::new("leave-playlist", t!("menu-remove-playlist-from-library"))
                .icon("icons/heart-off.svg")
                .on_click(move |_, _, cx| {
                    let library = Sonora::global(cx).library.clone();
                    library.update(cx, |library, cx| {
                        library.remove_playlist_from_library(id.clone(), cx)
                    });
                })
        }
        false => MenuItem::new("join-playlist", t!("menu-add-playlist-to-library"))
            .icon("icons/heart.svg")
            .on_click(move |_, _, cx| {
                let library = Sonora::global(cx).library.clone();
                library.update(cx, |library, cx| {
                    library.add_playlist_to_library(playlist.clone(), cx)
                });
            }),
    }
}
