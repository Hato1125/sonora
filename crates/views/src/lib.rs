// SPDX-License-Identifier: GPL-3.0-or-later

mod root;
mod screens;
mod shared;

pub use root::Root;
use screens::artist::ArtistView;
use screens::detail::DetailView;
use screens::home::HomeView;
pub use screens::library::LibraryView;
pub use screens::login::LoginView;
pub use screens::settings::SettingsView;
use screens::song::SongView;
use shared::adaptive::Adaptive;
