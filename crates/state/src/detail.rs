use gpui::{Context, Entity, Task};
use i18n::t;
use music::{Album, AlbumDetail, ArtistRef, Playlist, PlaylistDetail, Track};

use crate::{Io, Library, Session, SessionEvent, join, mosaic};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Collection {
    Album,
    Playlist,
}

enum Loaded {
    Album(AlbumDetail),
    Playlist(PlaylistDetail),
}

pub struct Header {
    pub kind: Collection,
    pub title: String,
    pub artist: Option<String>,
    pub artist_refs: Vec<ArtistRef>,
    pub release_date: Option<String>,
    pub meta: Vec<String>,
    pub cover: Option<String>,
}

pub struct Detail {
    id: Option<String>,
    header: Option<Header>,
    kind: Option<Collection>,
    album: Option<Album>,
    playlist: Option<Playlist>,
    tracks: Vec<Track>,
    loading: bool,
    error: Option<String>,
    session: Entity<Session>,
    library: Entity<Library>,
    io: Io,
    task: Option<Task<()>>,
    mutation: Option<Task<()>>,
    mosaic: Option<Task<()>>,
}

impl Detail {
    pub fn new(
        session: Entity<Session>,
        library: Entity<Library>,
        io: Io,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.subscribe(&session, |this, _, event, cx| match event {
            SessionEvent::SignedOut => {
                if !this.id.as_deref().is_some_and(music::is_local_id) {
                    this.clear();
                    cx.notify();
                }
            }
            SessionEvent::SignedIn => this.resume(cx),
            SessionEvent::LocalChanged => {}
        })
        .detach();

        cx.observe(&library, |this, library, cx| {
            let Some(id) = this.id.clone() else {
                return;
            };
            let Some(mut playlist) = library.read(cx).playlist(&id).cloned() else {
                return;
            };
            if playlist.cover.is_none() {
                playlist.cover = this.playlist.as_ref().and_then(|shown| shown.cover.clone());
            }
            if this.playlist.as_ref() == Some(&playlist) {
                return;
            }
            this.header = Some(playlist_header(&playlist));
            this.playlist = Some(playlist);
            cx.notify();
        })
        .detach();

        Self {
            id: None,
            header: None,
            kind: None,
            album: None,
            playlist: None,
            tracks: Vec::new(),
            loading: false,
            error: None,
            session,
            library,
            io,
            task: None,
            mutation: None,
            mosaic: None,
        }
    }

    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    pub fn header(&self) -> Option<&Header> {
        self.header.as_ref()
    }

    pub fn album(&self) -> Option<&Album> {
        self.album.as_ref()
    }

    pub fn playlist(&self) -> Option<&Playlist> {
        self.playlist.as_ref()
    }

    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }

    pub fn is_loading(&self) -> bool {
        self.loading
    }

    pub fn remove_from_playlist(&mut self, track_id: String, cx: &mut Context<Self>) {
        if self.mutation.is_some() {
            log::warn!("detail: cannot remove a track while another change is running");
            return;
        }
        let Some(playlist_id) = self.id.clone() else {
            log::warn!("detail: cannot remove a track without a playlist");
            return;
        };
        let Some(client) = self.session.read(cx).client() else {
            log::warn!("detail: cannot remove a track while signed out");
            return;
        };
        let io = self.io.clone();
        self.mutation = Some(cx.spawn(async move |this, cx| {
            let removed_id = track_id.clone();
            let removed = join(io.spawn(async move {
                client
                    .remove_track_from_playlist(&playlist_id, &removed_id)
                    .await
            }))
            .await;
            this.update(cx, |this, cx| {
                this.mutation = None;
                match removed {
                    Ok(()) => {
                        this.tracks
                            .retain(|track| track.id.as_deref() != Some(&track_id));
                        cx.notify();
                    }
                    Err(error) => {
                        log::warn!("detail: cannot remove track from playlist: {error:#}")
                    }
                }
            })
            .ok();
        }));
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn open_album(&mut self, id: &str, cx: &mut Context<Self>) {
        let library = self.library.read(cx);
        let known = library
            .album(id)
            .or_else(|| library.local_album(id))
            .cloned();
        let header = known.as_ref().map(album_header);
        if self.open(Collection::Album, id, header, cx) {
            self.album = known;
        }
    }

    pub fn open_playlist(&mut self, id: &str, cx: &mut Context<Self>) {
        let known = self.library.read(cx).playlist(id).cloned();
        let header = known.as_ref().map(playlist_header);
        if self.open(Collection::Playlist, id, header, cx) {
            self.playlist = known;
        }
    }

    fn open(
        &mut self,
        kind: Collection,
        id: &str,
        known: Option<Header>,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.shows(id) {
            return false;
        }

        self.clear();
        self.id = Some(id.to_owned());
        self.kind = Some(kind);
        self.header = known;
        self.load(kind, id.to_owned(), cx);
        true
    }

    fn resume(&mut self, cx: &mut Context<Self>) {
        let (Some(kind), Some(id)) = (self.kind, self.id.clone()) else {
            return;
        };
        if self.loading || !self.tracks.is_empty() {
            return;
        }
        self.load(kind, id, cx);
    }

    fn load(&mut self, kind: Collection, id: String, cx: &mut Context<Self>) {
        let session = self.session.read(cx);
        let client = match music::is_local_id(&id) {
            true => session.local_client(),
            false => session.client(),
        };
        let Some(client) = client else {
            cx.notify();
            return;
        };

        self.loading = true;
        self.error = None;
        cx.notify();

        let io = self.io.clone();
        self.task = Some(cx.spawn(async move |this, cx| {
            let loaded = join(io.spawn(async move {
                match kind {
                    Collection::Album => client.album(&id).await.map(Loaded::Album),
                    Collection::Playlist => client.playlist(&id).await.map(Loaded::Playlist),
                }
            }))
            .await;

            this.update(cx, |this, cx| {
                this.loading = false;
                match loaded {
                    Ok(Loaded::Album(detail)) => {
                        this.header = Some(album_header(&detail.album));
                        this.album = Some(detail.album);
                        this.tracks = detail.tracks;
                    }
                    Ok(Loaded::Playlist(mut detail)) => {
                        if detail.playlist.cover.is_none() {
                            detail.playlist.cover = this.known_mosaic(&detail.playlist, cx);
                        }
                        if detail.playlist.cover.is_none() {
                            this.paint_mosaic(&detail.playlist, &detail.tracks, cx);
                        }
                        this.header = Some(playlist_header(&detail.playlist));
                        this.playlist = Some(detail.playlist);
                        this.tracks = detail.tracks;
                    }
                    Err(error) => this.error = Some(format!("{error:#}")),
                }
                cx.notify();
            })
            .ok();
        }));
    }

    fn known_mosaic(&self, playlist: &Playlist, cx: &Context<Self>) -> Option<String> {
        self.library
            .read(cx)
            .playlist(&playlist.id)
            .and_then(|known| known.cover.clone())
            .or_else(|| mosaic::cached(&playlist.id, playlist.track_count))
    }

    fn paint_mosaic(&mut self, playlist: &Playlist, tracks: &[Track], cx: &mut Context<Self>) {
        let covers = music::distinct_covers(tracks, mosaic::TILES);
        if covers.len() < mosaic::TILES {
            return;
        }

        let id = playlist.id.clone();
        let stamp = playlist.track_count;
        let io = self.io.clone();
        let http = cx.http_client();
        self.mosaic = Some(cx.spawn(async move |this, cx| {
            let built =
                join(io.spawn(async move { mosaic::build(http, &id, stamp, covers).await })).await;

            this.update(cx, |this, cx| match built {
                Ok(cover) => {
                    if let Some(header) = this.header.as_mut() {
                        header.cover = Some(cover.clone());
                    }
                    if let Some(playlist) = this.playlist.as_mut() {
                        playlist.cover = Some(cover);
                    }
                    cx.notify();
                }
                Err(error) => log::warn!("detail: cannot build a mosaic: {error:#}"),
            })
            .ok();
        }));
    }

    fn shows(&self, id: &str) -> bool {
        let same = self.id.as_deref() == Some(id);
        same && (self.loading || !self.tracks.is_empty())
    }

    fn clear(&mut self) {
        self.task = None;
        self.mutation = None;
        self.mosaic = None;
        self.id = None;
        self.header = None;
        self.kind = None;
        self.album = None;
        self.playlist = None;
        self.tracks.clear();
        self.loading = false;
        self.error = None;
    }
}

fn album_header(album: &Album) -> Header {
    let mut parts = Vec::new();
    if album.track_count > 0 {
        parts.push(t!("count-songs", count = album.track_count).to_string());
    }

    Header {
        kind: Collection::Album,
        title: album.name.clone(),
        artist: Some(album.artists.clone()),
        artist_refs: album.artist_refs.clone(),
        release_date: match album.release_date.is_empty() {
            true => (album.year > 0).then(|| album.year.to_string()),
            false => Some(album.release_date.clone()),
        },
        meta: parts,
        cover: album.cover_large.clone(),
    }
}

fn playlist_header(playlist: &Playlist) -> Header {
    let mut parts = vec![playlist.owner.clone()];
    if playlist.track_count > 0 {
        parts.push(t!("count-songs", count = playlist.track_count).to_string());
    }

    Header {
        kind: Collection::Playlist,
        title: playlist.name.clone(),
        artist: None,
        artist_refs: Vec::new(),
        release_date: None,
        meta: parts,
        cover: playlist.cover.clone(),
    }
}
