pub mod auth;

mod albums;
mod client;
mod collection;
mod collection2;
mod models;
mod profiles;
mod wire;

mod protos {
    include!(concat!(env!("OUT_DIR"), "/protos/mod.rs"));
}

pub use auth::AuthConfig;
pub use client::{LibrespotClient, SpotifyApi};
pub use models::{Album, Playlist, Track, UserProfile};
