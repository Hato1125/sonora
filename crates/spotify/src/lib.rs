// SPDX-License-Identifier: GPL-3.0-or-later

pub mod auth;

mod albums;
mod artists;
mod client;
mod collection;
mod collection2;
mod pathfinder;
mod pb;
mod playlists;
mod profiles;
mod provider;
mod radio;
mod search;
mod wire;

pub use auth::AuthConfig;
pub use client::LibrespotClient;
pub use provider::SpotifyProvider;
