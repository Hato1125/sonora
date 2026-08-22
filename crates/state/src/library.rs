use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use gpui::{Context, Entity, Task};
use music::{Album, MusicApi, Playlist, SavedArtist, Track};

use crate::{Io, Note, Session, SessionEvent, Toasts, join, mosaic};

const PAGE_LIMIT: u32 = 10000;

type Loaded = (
    anyhow::Result<Vec<Track>>,
    anyhow::Result<Vec<Playlist>>,
    anyhow::Result<Vec<Album>>,
    anyhow::Result<Vec<SavedArtist>>,
);

type LoadedLocal = (anyhow::Result<Vec<Track>>, anyhow::Result<Vec<Album>>);

type PlaylistMutation = (&'static str, &'static str, Option<String>, Option<String>);

fn partial(loaded: Loaded) -> LibraryState {
    let (tracks, playlists, albums, artists) = loaded;
    if let (Err(tracks), Err(playlists), Err(albums)) = (&tracks, &playlists, &albums) {
        return LibraryState::Failed(format!("{tracks:#}\n{playlists:#}\n{albums:#}"));
    }

    let mut problems = Vec::new();
    LibraryState::Ready {
        tracks: take(LibraryPart::Tracks, tracks, &mut problems),
        playlists: take(LibraryPart::Playlists, playlists, &mut problems),
        albums: take(LibraryPart::Albums, albums, &mut problems),
        artists: take(LibraryPart::Artists, artists, &mut problems),
        problems,
    }
}

fn partial_local(loaded: LoadedLocal) -> LibraryState {
    let (tracks, albums) = loaded;
    if let (Err(tracks), Err(albums)) = (&tracks, &albums) {
        return LibraryState::Failed(format!("{tracks:#}\n{albums:#}"));
    }

    let mut problems = Vec::new();
    LibraryState::Ready {
        tracks: take(LibraryPart::Tracks, tracks, &mut problems),
        playlists: Vec::new(),
        albums: take(LibraryPart::Albums, albums, &mut problems),
        artists: Vec::new(),
        problems,
    }
}

fn stamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn take<T>(
    part: LibraryPart,
    result: anyhow::Result<Vec<T>>,
    problems: &mut Vec<Problem>,
) -> Vec<T> {
    result.unwrap_or_else(|error| {
        log::warn!("library: cannot load {}: {error:#}", part.label());
        problems.push(Problem {
            part,
            reason: format!("{error:#}"),
        });
        Vec::new()
    })
}

pub enum LibraryEvent {
    PlaylistGone(String),
    TrackDropped { playlist: String, track: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LibraryPart {
    Tracks,
    Playlists,
    Albums,
    Artists,
}

impl LibraryPart {
    fn label(self) -> &'static str {
        match self {
            Self::Tracks => "songs",
            Self::Playlists => "playlists",
            Self::Albums => "albums",
            Self::Artists => "artists",
        }
    }
}

pub struct Problem {
    pub part: LibraryPart,
    pub reason: String,
}

pub enum LibraryState {
    Empty,
    Loading,
    Ready {
        tracks: Vec<Track>,
        playlists: Vec<Playlist>,
        albums: Vec<Album>,
        artists: Vec<SavedArtist>,
        problems: Vec<Problem>,
    },
    Failed(String),
}

impl gpui::EventEmitter<LibraryEvent> for Library {}

pub struct Library {
    state: LibraryState,
    local: LibraryState,
    session: Entity<Session>,
    io: Io,
    task: Option<Task<()>>,
    local_task: Option<Task<()>>,
    playlist_task: Option<Task<()>>,
    pending: HashMap<String, Task<()>>,
    pending_albums: HashMap<String, Task<()>>,
    pending_artists: HashMap<String, Task<()>>,
    contents: HashMap<String, HashSet<String>>,
    reading: HashMap<String, Task<()>>,
    mosaics: HashMap<String, Task<()>>,
}

impl Library {
    pub fn new(session: Entity<Session>, io: Io, cx: &mut Context<Self>) -> Self {
        cx.subscribe(&session, |this, session, event, cx| match event {
            SessionEvent::SignedIn => {
                if !session.read(cx).authenticated() {
                    this.state = LibraryState::Empty;
                    cx.notify();
                    return;
                }
                let client = session.read(cx).client();
                if let Some(client) = client {
                    this.load(client, cx);
                }
            }
            SessionEvent::SignedOut => {
                this.contents.clear();
                this.reading.clear();
                this.mosaics.clear();
                this.task = None;
                this.playlist_task = None;
                this.pending.clear();
                this.pending_albums.clear();
                this.pending_artists.clear();
                this.state = LibraryState::Empty;
                cx.notify();
            }
            SessionEvent::Reconnected => {
                if matches!(this.state, LibraryState::Failed(_))
                    && let Some(client) = session.read(cx).client()
                {
                    this.load(client, cx);
                }
            }
            SessionEvent::LocalChanged => {
                let client = session.read(cx).local_client();
                match client {
                    Some(client) => this.load_local(client, cx),
                    None => {
                        this.local_task = None;
                        this.local = LibraryState::Empty;
                        cx.notify();
                    }
                }
            }
        })
        .detach();

        let local_client = session.read(cx).local_client();

        let mut library = Self {
            state: LibraryState::Loading,
            local: LibraryState::Empty,
            session,
            io,
            task: None,
            local_task: None,
            playlist_task: None,
            pending: HashMap::new(),
            pending_albums: HashMap::new(),
            pending_artists: HashMap::new(),
            contents: HashMap::new(),
            reading: HashMap::new(),
            mosaics: HashMap::new(),
        };
        if let Some(client) = local_client {
            library.load_local(client, cx);
        }
        library
    }

    pub fn state(&self) -> &LibraryState {
        &self.state
    }

    pub fn local_state(&self) -> &LibraryState {
        &self.local
    }

    pub fn part_failed(&self, part: LibraryPart) -> bool {
        Self::failed_parts(&self.state).any(|failed| failed == part)
    }

    pub fn local_part_failed(&self, part: LibraryPart) -> bool {
        Self::failed_parts(&self.local).any(|failed| failed == part)
    }

    fn failed_parts(state: &LibraryState) -> impl Iterator<Item = LibraryPart> + '_ {
        let problems = match state {
            LibraryState::Ready { problems, .. } => problems.as_slice(),
            _ => &[],
        };
        problems.iter().map(|problem| problem.part)
    }

    pub fn rescan_local(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.session
            .update(cx, |session, cx| session.choose_local_folder(path, cx));
    }

    pub fn forget_local(&mut self, cx: &mut Context<Self>) {
        self.session
            .update(cx, |session, cx| session.clear_local_folder(cx));
    }

    pub fn is_loading(&self) -> bool {
        matches!(self.state, LibraryState::Loading)
    }

    pub fn local_is_loading(&self) -> bool {
        matches!(self.local, LibraryState::Loading)
    }

    pub fn saved(&self, track_id: &str) -> bool {
        let LibraryState::Ready { tracks, .. } = &self.state else {
            return false;
        };
        tracks
            .iter()
            .any(|track| track.id.as_deref() == Some(track_id))
    }

    pub fn pending(&self, track_id: &str) -> bool {
        self.pending.contains_key(track_id)
    }

    pub fn toggle(&mut self, mut track: Track, cx: &mut Context<Self>) {
        let Some(track_id) = track.id.clone() else {
            return;
        };
        if self.pending(&track_id) {
            return;
        }
        let Some(client) = self.session.read(cx).client() else {
            return;
        };
        let saved = !self.saved(&track_id);
        let previous = match &self.state {
            LibraryState::Ready { tracks, .. } => tracks
                .iter()
                .find(|track| track.id.as_deref() == Some(track_id.as_str()))
                .cloned(),
            _ => None,
        };
        if saved {
            track.added_at = Some(stamp());
        }
        self.set_saved(track.clone(), saved);

        let io = self.io.clone();
        let request_id = track_id.clone();
        let pending_id = track_id.clone();
        let task = cx.spawn(async move |this, cx| {
            let result =
                join(io.spawn(async move { client.set_track_saved(&request_id, saved).await }))
                    .await;

            this.update(cx, |this, cx| {
                this.pending.remove(&pending_id);
                if let Err(error) = result {
                    match previous {
                        Some(previous) => this.set_saved(previous, true),
                        None => this.set_saved(track, false),
                    }
                    log::warn!("library: cannot update saved track: {error:#}");
                }
                cx.notify();
            })
            .ok();
        });
        self.pending.insert(track_id, task);
        cx.notify();
    }

    pub fn saved_album(&self, album_id: &str) -> bool {
        self.album(album_id).is_some()
    }

    pub fn pending_album(&self, album_id: &str) -> bool {
        self.pending_albums.contains_key(album_id)
    }

    pub fn toggle_album(&mut self, album: Album, cx: &mut Context<Self>) {
        let album_id = album.id.clone();
        if self.pending_album(&album_id) {
            return;
        }
        let Some(client) = self.session.read(cx).client() else {
            return;
        };
        let saved = !self.saved_album(&album_id);
        let previous = self.album(&album_id).cloned();
        self.set_album_saved(album.clone(), saved);

        let io = self.io.clone();
        let request_id = album_id.clone();
        let pending_id = album_id.clone();
        let task = cx.spawn(async move |this, cx| {
            let result =
                join(io.spawn(async move { client.set_album_saved(&request_id, saved).await }))
                    .await;

            this.update(cx, |this, cx| {
                this.pending_albums.remove(&pending_id);
                if let Err(error) = result {
                    match previous {
                        Some(previous) => this.set_album_saved(previous, true),
                        None => this.set_album_saved(album, false),
                    }
                    log::warn!("library: cannot update saved album: {error:#}");
                }
                cx.notify();
            })
            .ok();
        });
        self.pending_albums.insert(album_id, task);
        cx.notify();
    }

    fn set_album_saved(&mut self, album: Album, saved: bool) {
        let LibraryState::Ready { albums, .. } = &mut self.state else {
            return;
        };
        match saved {
            true if !albums.iter().any(|known| known.id == album.id) => albums.push(album),
            false => albums.retain(|known| known.id != album.id),
            _ => {}
        }
    }

    pub fn saved_artist(&self, artist_id: &str) -> bool {
        self.artist(artist_id).is_some()
    }

    pub fn pending_artist(&self, artist_id: &str) -> bool {
        self.pending_artists.contains_key(artist_id)
    }

    pub fn artist(&self, id: &str) -> Option<&SavedArtist> {
        let LibraryState::Ready { artists, .. } = &self.state else {
            return None;
        };
        artists.iter().find(|artist| artist.id == id)
    }

    pub fn toggle_artist(&mut self, mut artist: SavedArtist, cx: &mut Context<Self>) {
        let artist_id = artist.id.clone();
        if self.pending_artist(&artist_id) {
            return;
        }
        let Some(client) = self.session.read(cx).client() else {
            return;
        };
        let saved = !self.saved_artist(&artist_id);
        let previous = self.artist(&artist_id).cloned();
        if saved {
            artist.added_at = Some(stamp());
        }
        self.set_artist_saved(artist.clone(), saved);

        let io = self.io.clone();
        let request_id = artist_id.clone();
        let pending_id = artist_id.clone();
        let task = cx.spawn(async move |this, cx| {
            let result =
                join(io.spawn(async move { client.set_artist_saved(&request_id, saved).await }))
                    .await;

            this.update(cx, |this, cx| {
                this.pending_artists.remove(&pending_id);
                if let Err(error) = result {
                    match previous {
                        Some(previous) => this.set_artist_saved(previous, true),
                        None => this.set_artist_saved(artist, false),
                    }
                    log::warn!("library: cannot update the followed artist: {error:#}");
                }
                cx.notify();
            })
            .ok();
        });
        self.pending_artists.insert(artist_id, task);
        cx.notify();
    }

    fn set_artist_saved(&mut self, artist: SavedArtist, saved: bool) {
        let LibraryState::Ready { artists, .. } = &mut self.state else {
            return;
        };
        match saved {
            true if !artists.iter().any(|known| known.id == artist.id) => artists.push(artist),
            false => artists.retain(|known| known.id != artist.id),
            _ => {}
        }
    }

    pub fn create_playlist(&mut self, name: String, track: Option<String>, cx: &mut Context<Self>) {
        self.mutate_playlist(
            ("create playlist", "toast-playlist-created", None, None),
            move |client| async move {
                let id = client.create_playlist(&name).await?;
                if let Some(track) = track {
                    client.add_track_to_playlist(&id, &track).await?;
                }
                let fetched = client.playlist(&id).await.map(|detail| detail.playlist);
                Ok(fetched.unwrap_or_else(|error| {
                    log::warn!("library: a new playlist is not readable yet: {error:#}");
                    Playlist {
                        id,
                        name,
                        owner: String::new(),
                        owned: true,
                        collaborative: false,
                        public: false,
                        cover: None,
                        track_count: 0,
                        modified_at: None,
                    }
                }))
            },
            Self::insert_playlist,
            cx,
        );
    }

    pub fn rename_playlist(&mut self, id: String, name: String, cx: &mut Context<Self>) {
        let renamed = (id.clone(), name.clone());
        self.mutate_playlist(
            (
                "rename playlist",
                "toast-playlist-renamed",
                None,
                Some(id.clone()),
            ),
            move |client| async move { client.rename_playlist(&id, &name).await },
            move |this, _, cx| {
                let (id, name) = renamed;
                this.amend_playlist(&id, |playlist| playlist.name = name, cx);
            },
            cx,
        );
    }

    pub fn set_playlist_public(&mut self, id: String, public: bool, cx: &mut Context<Self>) {
        let changed = id.clone();
        self.mutate_playlist(
            (
                "change playlist visibility",
                "toast-playlist-visibility",
                None,
                Some(id.clone()),
            ),
            move |client| async move { client.set_playlist_public(&id, public).await },
            move |this, _, cx| {
                this.amend_playlist(&changed, |playlist| playlist.public = public, cx);
            },
            cx,
        );
    }

    pub fn add_to_playlist(
        &mut self,
        playlist_id: String,
        track_id: String,
        cx: &mut Context<Self>,
    ) {
        let added = playlist_id.clone();
        let held = track_id.clone();
        let name = self
            .playlist(&playlist_id)
            .map(|playlist| playlist.name.clone());
        self.mutate_playlist(
            (
                "add track to playlist",
                "toast-track-added",
                name,
                Some(playlist_id.clone()),
            ),
            move |client| async move { client.add_track_to_playlist(&playlist_id, &track_id).await },
            move |this, _, cx| {
                this.amend_playlist(&added, |playlist| playlist.track_count += 1, cx);
                if let Some(ids) = this.contents.get_mut(&added) {
                    ids.insert(held);
                }
            },
            cx,
        );
    }

    pub fn remove_from_playlist(
        &mut self,
        playlist_id: String,
        track_id: String,
        cx: &mut Context<Self>,
    ) {
        let emptied = playlist_id.clone();
        let dropped = track_id.clone();
        let name = self
            .playlist(&playlist_id)
            .map(|playlist| playlist.name.clone());
        self.mutate_playlist(
            (
                "remove track from playlist",
                "toast-track-removed",
                name,
                Some(playlist_id.clone()),
            ),
            move |client| async move {
                client
                    .remove_track_from_playlist(&playlist_id, &track_id)
                    .await
            },
            move |this, _, cx| {
                this.amend_playlist(
                    &emptied,
                    |playlist| playlist.track_count = playlist.track_count.saturating_sub(1),
                    cx,
                );
                if let Some(ids) = this.contents.get_mut(&emptied) {
                    ids.remove(&dropped);
                }
                cx.emit(LibraryEvent::TrackDropped {
                    playlist: emptied,
                    track: dropped,
                });
            },
            cx,
        );
    }

    pub fn delete_playlist(&mut self, id: String, cx: &mut Context<Self>) {
        let deleted = id.clone();
        self.mutate_playlist(
            (
                "delete playlist",
                "toast-playlist-deleted",
                None,
                Some(id.clone()),
            ),
            move |client| async move { client.delete_playlist(&id).await },
            move |this, _, cx| this.forget_playlist(&deleted, cx),
            cx,
        );
    }

    pub fn add_playlist_to_library(&mut self, playlist: Playlist, cx: &mut Context<Self>) {
        let id = playlist.id.clone();
        self.mutate_playlist(
            (
                "add playlist to library",
                "toast-playlist-added",
                None,
                Some(id.clone()),
            ),
            move |client| async move { client.add_playlist_to_library(&id).await },
            move |this, _, cx| this.insert_playlist(playlist, cx),
            cx,
        );
    }

    pub fn remove_playlist_from_library(&mut self, id: String, cx: &mut Context<Self>) {
        let removed = id.clone();
        self.mutate_playlist(
            (
                "remove playlist from library",
                "toast-playlist-removed",
                None,
                Some(id.clone()),
            ),
            move |client| async move { client.remove_playlist_from_library(&id).await },
            move |this, _, cx| this.forget_playlist(&removed, cx),
            cx,
        );
    }

    pub fn album(&self, id: &str) -> Option<&Album> {
        let LibraryState::Ready { albums, .. } = &self.state else {
            return None;
        };
        albums.iter().find(|album| album.id == id)
    }

    pub fn local_album(&self, id: &str) -> Option<&Album> {
        let LibraryState::Ready { albums, .. } = &self.local else {
            return None;
        };
        albums.iter().find(|album| album.id == id)
    }

    pub fn holds(&self, playlist_id: &str, track_id: &str) -> Option<bool> {
        Some(self.contents.get(playlist_id)?.contains(track_id))
    }

    fn adopt_mosaics(&mut self) -> Vec<(String, u32)> {
        let LibraryState::Ready { playlists, .. } = &mut self.state else {
            return Vec::new();
        };

        let mut wanted = Vec::new();
        for playlist in playlists.iter_mut() {
            if playlist.cover.is_some() || (playlist.track_count as usize) < mosaic::TILES {
                continue;
            }
            match mosaic::cached(&playlist.id, playlist.track_count) {
                Some(cover) => playlist.cover = Some(cover),
                None => wanted.push((playlist.id.clone(), playlist.track_count)),
            }
        }

        wanted
    }

    fn build_mosaics(&mut self, cx: &mut Context<Self>) {
        let wanted = self.adopt_mosaics();
        let Some(client) = self.session.read(cx).client() else {
            return;
        };

        for (id, stamp) in wanted {
            if self.mosaics.contains_key(&id) {
                continue;
            }
            if self.holds_tracks(&id) {
                continue;
            }

            let io = self.io.clone();
            let client = client.clone();
            let asked = id.clone();
            let key = id.clone();
            let task = cx.spawn(async move |this, cx| {
                let covers = join(
                    io.spawn(async move { client.playlist_covers(&asked, mosaic::TILES).await }),
                )
                .await;
                match covers {
                    Ok(covers) => {
                        this.update(cx, |this, cx| this.paint_mosaic(id, stamp, covers, cx))
                            .ok();
                    }
                    Err(error) => log::warn!("library: cannot read playlist covers: {error:#}"),
                }
            });
            self.mosaics.insert(key, task);
        }
    }

    fn uncovered(&self, id: &str) -> Option<u32> {
        let LibraryState::Ready { playlists, .. } = &self.state else {
            return None;
        };
        let playlist = playlists.iter().find(|playlist| playlist.id == id)?;

        (playlist.cover.is_none() && playlist.track_count as usize >= mosaic::TILES)
            .then_some(playlist.track_count)
    }

    fn holds_tracks(&self, id: &str) -> bool {
        let LibraryState::Ready { playlists, .. } = &self.state else {
            return false;
        };

        playlists
            .iter()
            .find(|playlist| playlist.id == id)
            .is_some_and(|playlist| playlist.owned || playlist.collaborative)
    }

    fn paint_mosaic(
        &mut self,
        id: String,
        stamp: u32,
        covers: Vec<String>,
        cx: &mut Context<Self>,
    ) {
        if covers.len() < mosaic::TILES {
            self.mosaics.remove(&id);
            return;
        }

        let io = self.io.clone();
        let http = cx.http_client();
        let built_for = id.clone();
        let key = id.clone();
        let task = cx.spawn(async move |this, cx| {
            let built =
                join(io.spawn(async move { mosaic::build(http, &built_for, stamp, covers).await }))
                    .await;

            this.update(cx, |this, cx| {
                this.mosaics.remove(&id);
                match built {
                    Ok(cover) => {
                        this.set_playlist_cover(&id, cover);
                        cx.notify();
                    }
                    Err(error) => log::warn!("library: cannot build a mosaic: {error:#}"),
                }
            })
            .ok();
        });
        self.mosaics.insert(key, task);
    }

    fn set_playlist_cover(&mut self, id: &str, cover: String) {
        let LibraryState::Ready { playlists, .. } = &mut self.state else {
            return;
        };
        if let Some(playlist) = playlists.iter_mut().find(|playlist| playlist.id == id) {
            playlist.cover = Some(cover);
        }
    }

    pub fn read_playlists(&mut self, cx: &mut Context<Self>) {
        let LibraryState::Ready { playlists, .. } = &self.state else {
            return;
        };
        let wanted: Vec<String> = playlists
            .iter()
            .filter(|playlist| playlist.owned || playlist.collaborative)
            .map(|playlist| playlist.id.clone())
            .filter(|id| !self.contents.contains_key(id) && !self.reading.contains_key(id))
            .collect();
        let Some(client) = self.session.read(cx).client() else {
            return;
        };

        for id in wanted {
            let io = self.io.clone();
            let client = client.clone();
            let key = id.clone();
            let asked = id.clone();
            let task = cx.spawn(async move |this, cx| {
                let listed =
                    join(io.spawn(async move { client.playlist_tracks(&asked).await })).await;

                this.update(cx, |this, cx| {
                    this.reading.remove(&key);
                    match listed {
                        Ok(tracks) => {
                            if let Some(stamp) = this.uncovered(&key) {
                                let covers = music::distinct_covers(&tracks, mosaic::TILES);
                                this.paint_mosaic(key.clone(), stamp, covers, cx);
                            }
                            let ids = tracks.into_iter().filter_map(|track| track.id).collect();
                            this.contents.insert(key, ids);
                            cx.notify();
                        }
                        Err(error) => {
                            log::warn!("library: cannot read a playlist: {error:#}")
                        }
                    }
                })
                .ok();
            });
            self.reading.insert(id.clone(), task);
        }
    }

    pub fn playlist(&self, id: &str) -> Option<&Playlist> {
        let LibraryState::Ready { playlists, .. } = &self.state else {
            return None;
        };
        playlists.iter().find(|playlist| playlist.id == id)
    }

    fn mutate_playlist<F, R, T, A>(
        &mut self,
        mutation_info: PlaylistMutation,
        mutation: F,
        apply: A,
        cx: &mut Context<Self>,
    ) where
        F: FnOnce(Arc<dyn MusicApi>) -> R + Send + 'static,
        R: Future<Output = anyhow::Result<T>> + Send + 'static,
        T: Send + 'static,
        A: FnOnce(&mut Self, T, &mut Context<Self>) + 'static,
    {
        let (action, done, name, invalidated) = mutation_info;
        if self.playlist_task.is_some() {
            log::warn!("library: cannot {action} while another change is running");
            Toasts::show(Note::Failed, "toast-playlist-busy", cx);
            return;
        }
        let Some(client) = self.session.read(cx).client() else {
            log::warn!("library: cannot {action} while signed out");
            Toasts::show(Note::Failed, "toast-playlist-signed-out", cx);
            return;
        };
        let catalog = invalidated
            .as_deref()
            .and_then(|id| self.session.read(cx).catalog(id));
        let io = self.io.clone();
        self.playlist_task = Some(cx.spawn(async move |this, cx| {
            let result = join(io.spawn(async move { mutation(client).await })).await;
            if result.is_ok()
                && let (Some(catalog), Some(id)) = (catalog, invalidated)
            {
                catalog.invalidate_playlist(&id).await;
            }
            this.update(cx, |this, cx| {
                this.playlist_task = None;
                match result {
                    Ok(outcome) => {
                        apply(this, outcome, cx);
                        match name {
                            Some(name) => Toasts::about(Note::Done, done, name, cx),
                            None => Toasts::show(Note::Done, done, cx),
                        }
                    }
                    Err(error) => {
                        log::warn!("library: cannot {action}: {error:#}");
                        Toasts::show(Note::Failed, "toast-playlist-failed", cx);
                    }
                }
                cx.notify();
            })
            .ok();
        }));
    }

    fn insert_playlist(&mut self, playlist: Playlist, cx: &mut Context<Self>) {
        let LibraryState::Ready { playlists, .. } = &mut self.state else {
            return;
        };
        playlists.retain(|known| known.id != playlist.id);
        playlists.push(playlist);
        cx.notify();
    }

    fn forget_playlist(&mut self, id: &str, cx: &mut Context<Self>) {
        cx.emit(LibraryEvent::PlaylistGone(id.to_owned()));
        self.drop_playlist(id, cx);
    }

    fn drop_playlist(&mut self, id: &str, cx: &mut Context<Self>) {
        let LibraryState::Ready { playlists, .. } = &mut self.state else {
            return;
        };
        playlists.retain(|playlist| playlist.id != id);
        cx.notify();
    }

    fn amend_playlist(
        &mut self,
        id: &str,
        amend: impl FnOnce(&mut Playlist),
        cx: &mut Context<Self>,
    ) {
        let LibraryState::Ready { playlists, .. } = &mut self.state else {
            return;
        };
        let Some(playlist) = playlists.iter_mut().find(|playlist| playlist.id == id) else {
            return;
        };
        amend(playlist);
        cx.notify();
    }

    fn set_saved(&mut self, track: Track, saved: bool) {
        let LibraryState::Ready { tracks, .. } = &mut self.state else {
            return;
        };
        let id = track.id.as_deref();
        match saved {
            true if !tracks.iter().any(|saved| saved.id.as_deref() == id) => tracks.push(track),
            false => tracks.retain(|saved| saved.id.as_deref() != id),
            _ => {}
        }
    }

    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        let client = self.session.read(cx).client();
        if let Some(client) = client {
            self.load(client, cx);
        }
    }

    fn load(&mut self, client: Arc<dyn MusicApi>, cx: &mut Context<Self>) {
        self.playlist_task = None;
        self.pending.clear();
        self.pending_albums.clear();
        self.pending_artists.clear();
        self.state = LibraryState::Loading;
        cx.notify();

        let io = self.io.clone();
        self.task = Some(cx.spawn(async move |this, cx| {
            let loaded = join(io.spawn(async move {
                anyhow::Ok(tokio::join!(
                    client.saved_tracks(PAGE_LIMIT),
                    client.playlists(PAGE_LIMIT),
                    client.saved_albums(PAGE_LIMIT),
                    client.saved_artists(PAGE_LIMIT)
                ))
            }))
            .await;

            this.update(cx, |this, cx| {
                this.state = match loaded {
                    Ok(loaded) => partial(loaded),
                    Err(error) => LibraryState::Failed(format!("{error:#}")),
                };
                this.read_playlists(cx);
                this.build_mosaics(cx);
                cx.notify();
            })
            .ok();
        }));
    }

    fn load_local(&mut self, client: Arc<dyn MusicApi>, cx: &mut Context<Self>) {
        self.local = LibraryState::Loading;
        cx.notify();

        let io = self.io.clone();
        self.local_task = Some(cx.spawn(async move |this, cx| {
            let loaded = join(io.spawn(async move {
                anyhow::Ok(tokio::join!(
                    client.saved_tracks(PAGE_LIMIT),
                    client.saved_albums(PAGE_LIMIT)
                ))
            }))
            .await;

            this.update(cx, |this, cx| {
                this.local = match loaded {
                    Ok(loaded) => partial_local(loaded),
                    Err(error) => LibraryState::Failed(format!("{error:#}")),
                };
                cx.notify();
            })
            .ok();
        }));
    }
}
