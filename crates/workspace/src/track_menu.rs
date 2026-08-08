// SPDX-License-Identifier: GPL-3.0-or-later

use gpui::{App, ClipboardItem, Entity, Styled as _};
use i18n::t;
use router::{Destination, navigate};
use spotify::Track;
use state::{LibraryState, Sonora};
use ui::{Menu, MenuItem, Scrollbar, SubmenuState};

#[derive(Clone)]
pub struct TrackMenu {
    playlist_submenu: SubmenuState,
    artist_submenu: SubmenuState,
    playlist_scrollbar: Entity<Scrollbar>,
}

impl TrackMenu {
    pub fn new(playlist_scrollbar: Entity<Scrollbar>) -> Self {
        Self {
            playlist_submenu: SubmenuState::default(),
            artist_submenu: SubmenuState::default(),
            playlist_scrollbar,
        }
    }

    pub fn reset(&self) {
        self.playlist_submenu.reset();
        self.artist_submenu.reset();
    }

    pub fn for_track(&self, track: &Track, cx: &App) -> Menu {
        let playlists = match Sonora::global(cx).library.read(cx).state() {
            LibraryState::Ready { playlists, .. } => playlists.clone(),
            _ => Vec::new(),
        };
        let playlist_menu = if playlists.is_empty() {
            Menu::new("playlist-submenu")
                .w(gpui::px(220.))
                .item(MenuItem::new("no-playlists", t!("menu-no-playlists")).disabled())
        } else {
            Menu::new("playlist-submenu")
                .w(gpui::px(220.))
                .max_h(gpui::px(360.))
                .scrollbar(self.playlist_scrollbar.clone())
                .item(
                    MenuItem::new("new-playlist", t!("menu-new-playlist"))
                        .icon("icons/plus.svg")
                        .disabled(),
                )
                .item(MenuItem::separator("playlist-separator"))
                .items(playlists.into_iter().map(|playlist| {
                    MenuItem::new(format!("playlist-{}", playlist.id), playlist.name)
                        .artwork(playlist.cover)
                        .disabled()
                }))
        };
        let copy = match track.id.clone() {
            Some(id) => MenuItem::new("copy-track-link", t!("menu-copy-link"))
                .icon("icons/link.svg")
                .on_click(move |_, _, cx| {
                    cx.write_to_clipboard(ClipboardItem::new_string(format!(
                        "https://open.spotify.com/track/{id}"
                    )));
                }),
            None => MenuItem::new("copy-track-link", t!("menu-copy-link"))
                .icon("icons/link.svg")
                .disabled(),
        };
        let queue = match track.playable {
            true => {
                let track = track.clone();
                MenuItem::new("add-to-queue", t!("menu-add-to-queue"))
                    .icon("icons/list-end.svg")
                    .on_click(move |_, _, cx| {
                        let queue = Sonora::global(cx).queue.clone();
                        queue.update(cx, |queue, cx| queue.append(track.clone(), cx));
                    })
            }
            false => MenuItem::new("add-to-queue", t!("menu-add-to-queue"))
                .icon("icons/list-end.svg")
                .disabled(),
        };
        let library = Sonora::global(cx).library.clone();
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
                .icon("icons/heart.svg")
                .on_click(move |_, _, cx| {
                    library.update(cx, |library, cx| library.toggle(track.clone(), cx));
                })
            }
            _ => MenuItem::new("toggle-library", t!("menu-add-to-library"))
                .icon("icons/heart.svg")
                .disabled(),
        };

        let album = match track.album_id.clone() {
            Some(id) => MenuItem::new("go-to-album", t!("menu-go-to-album"))
                .icon("icons/disc-3.svg")
                .on_click(move |_, _, cx| navigate(Destination::Album(id.clone().into()), cx)),
            None => MenuItem::new("go-to-album", t!("menu-go-to-album"))
                .icon("icons/disc-3.svg")
                .disabled(),
        };

        let artists = track
            .artist_refs
            .iter()
            .filter_map(|artist| {
                let id = artist.id.clone()?;
                Some((artist.name.clone(), id))
            })
            .collect::<Vec<_>>();
        let artist = match artists.len() {
            0 => MenuItem::new("go-to-artist", t!("menu-go-to-artist"))
                .icon("icons/user.svg")
                .disabled(),
            1 => {
                let id = artists[0].1.clone();
                MenuItem::new("go-to-artist", t!("menu-go-to-artist"))
                    .icon("icons/user.svg")
                    .on_click(move |_, _, cx| navigate(Destination::Artist(id.clone().into()), cx))
            }
            _ => {
                let artist_menu = Menu::new("artist-submenu")
                    .w(gpui::px(220.))
                    .max_h(gpui::px(360.))
                    .items(artists.into_iter().map(|(name, id)| {
                        MenuItem::new(format!("artist-{id}"), name).on_click(move |_, _, cx| {
                            navigate(Destination::Artist(id.clone().into()), cx)
                        })
                    }));
                MenuItem::new("go-to-artist", t!("menu-go-to-artist"))
                    .icon("icons/user.svg")
                    .submenu(artist_menu, self.artist_submenu.clone())
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

        Menu::new("track-context-menu")
            .relative()
            .w(gpui::px(210.))
            .item(
                MenuItem::new("add-to-playlist", t!("menu-add-to-playlist"))
                    .icon("icons/list-plus.svg")
                    .submenu(playlist_menu, self.playlist_submenu.clone()),
            )
            .item(toggle_library)
            .item(queue)
            .item(
                MenuItem::new("song-radio", t!("menu-song-radio"))
                    .icon("icons/radio.svg")
                    .disabled(),
            )
            .item(album)
            .item(artist)
            .item(details)
            .item(copy)
    }
}
