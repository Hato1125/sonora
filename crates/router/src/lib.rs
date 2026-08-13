mod link;
mod navigation;
mod uri;

pub use link::Link;
pub use navigation::{Navigation, NavigationEvent};
pub use uri::destination;

use gpui::{App, AppContext as _, Entity, Global, SharedString};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LibraryTab {
    Songs,
    Albums,
    Playlists,
    Artists,
    Local,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Home,
    Search,
    Songs,
    Albums,
    Playlists,
    Artists,
    Imported,
}

impl Screen {
    pub const ALL: [Self; 7] = [
        Self::Home,
        Self::Search,
        Self::Songs,
        Self::Albums,
        Self::Artists,
        Self::Playlists,
        Self::Imported,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Self::Home => "home",
            Self::Search => "search",
            Self::Songs => "songs",
            Self::Albums => "albums",
            Self::Playlists => "playlists",
            Self::Artists => "artists",
            Self::Imported => "imported",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Home => "nav-home",
            Self::Search => "nav-search",
            Self::Songs => "nav-songs",
            Self::Albums => "nav-albums",
            Self::Playlists => "nav-playlists",
            Self::Artists => "nav-artists",
            Self::Imported => "nav-local",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|screen| screen.id() == id)
    }

    pub fn destination(self) -> Destination {
        match self {
            Self::Home => Destination::Home,
            Self::Search => Destination::Search,
            Self::Songs => Destination::Library(LibraryTab::Songs),
            Self::Albums => Destination::Library(LibraryTab::Albums),
            Self::Playlists => Destination::Library(LibraryTab::Playlists),
            Self::Artists => Destination::Library(LibraryTab::Artists),
            Self::Imported => Destination::Library(LibraryTab::Local),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsTab {
    General,
    Appearance,
    Playback,
    About,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Destination {
    Home,
    Library(LibraryTab),
    Album(SharedString),
    Song(SharedString),
    Playlist(SharedString),
    Artist(SharedString),
    Genre(SharedString),
    Search,
    Settings(SettingsTab),
}

impl From<&ui::Pin> for Destination {
    fn from(pin: &ui::Pin) -> Self {
        let id = SharedString::from(pin.id.clone());
        match pin.kind {
            ui::PinKind::Album => Destination::Album(id),
            ui::PinKind::Artist => Destination::Artist(id),
            ui::PinKind::Playlist => Destination::Playlist(id),
            ui::PinKind::Song => Destination::Song(id),
        }
    }
}

impl Destination {
    pub fn same_section(&self, other: &Destination) -> bool {
        match (self, other) {
            (Destination::Library(_), Destination::Library(_))
            | (Destination::Settings(_), Destination::Settings(_)) => true,
            _ => self == other,
        }
    }
}

#[derive(Clone)]
struct Router(Entity<Navigation>);

impl Global for Router {}

pub fn init(start: Destination, cx: &mut App) {
    let navigation = cx.new(|_| Navigation::new(start));
    cx.set_global(Router(navigation));
}

pub fn trail(cx: &App) -> Entity<Navigation> {
    cx.global::<Router>().0.clone()
}

pub fn navigate(destination: Destination, cx: &mut App) {
    trail(cx).update(cx, |navigation, cx| navigation.go(destination, cx));
}

pub fn back(cx: &mut App) {
    trail(cx).update(cx, |navigation, cx| navigation.back(cx));
}

pub fn forward(cx: &mut App) {
    trail(cx).update(cx, |navigation, cx| navigation.forward(cx));
}
