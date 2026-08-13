use anyhow::{Context as _, Result};
use librespot_core::Session;
use serde::Deserialize;

use super::query;
use crate::{Album, ArtistRef, Genre, GenreDetail, GenreItem, GenreSection, Playlist, ReleaseType};

const PAGE_PREFIX: &str = "spotify:page:";
const PLAYLIST_PREFIX: &str = "spotify:playlist:";
const ALBUM_PREFIX: &str = "spotify:album:";
const ARTIST_PREFIX: &str = "spotify:artist:";
const INTEGRATION: &str = "INTEGRATION_WEB_PLAYER";
const SECTIONS: u32 = 20;
const ITEMS: u32 = 10;
const CARDS: u32 = 99;

#[derive(Deserialize)]
struct Start {
    #[serde(rename = "browseStart")]
    start: Option<Container>,
}

#[derive(Deserialize)]
struct Page {
    browse: Option<Container>,
}

#[derive(Deserialize)]
struct Container {
    header: Option<Header>,
    #[serde(default)]
    sections: Sections,
}

#[derive(Deserialize)]
struct Header {
    title: Option<Label>,
    color: Option<Hex>,
    #[serde(rename = "backgroundImage")]
    background: Option<Artwork>,
}

#[derive(Default, Deserialize)]
struct Sections {
    #[serde(default)]
    items: Vec<Section>,
}

#[derive(Deserialize)]
struct Section {
    #[serde(default)]
    data: SectionData,
    #[serde(rename = "sectionItems", default)]
    items: Items,
}

#[derive(Default, Deserialize)]
struct SectionData {
    title: Option<Label>,
}

#[derive(Default, Deserialize)]
struct Items {
    #[serde(default)]
    items: Vec<Item>,
}

#[derive(Deserialize)]
struct Item {
    #[serde(default)]
    uri: String,
    content: Option<Content>,
}

#[derive(Deserialize)]
struct Content {
    data: Option<Entity>,
}

#[derive(Deserialize)]
#[serde(tag = "__typename")]
enum Entity {
    Playlist(WirePlaylist),
    Album(WireAlbum),
    BrowseSectionContainer(WireCard),
    #[serde(other)]
    Unknown,
}

#[derive(Deserialize)]
struct WirePlaylist {
    #[serde(default)]
    name: String,
    #[serde(default)]
    images: Images,
    #[serde(rename = "ownerV2")]
    owner: Option<Owner>,
}

#[derive(Deserialize)]
struct WireAlbum {
    #[serde(default)]
    name: String,
    #[serde(rename = "coverArt", default)]
    cover: Artwork,
    #[serde(default)]
    artists: Artists,
}

#[derive(Deserialize)]
struct WireCard {
    data: Option<CardData>,
}

#[derive(Deserialize)]
struct CardData {
    #[serde(rename = "cardRepresentation")]
    card: Option<Representation>,
}

#[derive(Deserialize)]
struct Representation {
    title: Option<Label>,
    #[serde(default)]
    artwork: Artwork,
    #[serde(rename = "backgroundColor")]
    background: Option<Hex>,
}

#[derive(Deserialize)]
struct Owner {
    data: Option<OwnerData>,
}

#[derive(Deserialize)]
struct OwnerData {
    #[serde(default)]
    name: String,
}

#[derive(Default, Deserialize)]
struct Artists {
    #[serde(default)]
    items: Vec<Artist>,
}

#[derive(Deserialize)]
struct Artist {
    #[serde(default)]
    uri: String,
    profile: Option<Profile>,
}

#[derive(Deserialize)]
struct Profile {
    #[serde(default)]
    name: String,
}

#[derive(Default, Deserialize)]
struct Images {
    #[serde(default)]
    items: Vec<Artwork>,
}

#[derive(Default, Deserialize)]
struct Artwork {
    #[serde(default)]
    sources: Vec<Source>,
}

#[derive(Deserialize)]
struct Source {
    url: String,
}

#[derive(Deserialize)]
struct Label {
    #[serde(rename = "transformedLabel", default)]
    label: String,
}

#[derive(Deserialize)]
struct Hex {
    #[serde(default)]
    hex: String,
}

pub(crate) async fn all(session: &Session) -> Result<Vec<Genre>> {
    let variables = serde_json::json!({
        "pagePagination": { "offset": 0, "limit": ITEMS },
        "sectionPagination": { "offset": 0, "limit": CARDS },
        "browseEndUserIntegration": INTEGRATION,
    });
    cards(query::<Start>(session, "browseAll", variables).await?)
}

pub(crate) async fn page(session: &Session, genre_id: &str) -> Result<GenreDetail> {
    let variables = serde_json::json!({
        "uri": format!("{PAGE_PREFIX}{genre_id}"),
        "pagePagination": { "offset": 0, "limit": SECTIONS },
        "sectionPagination": { "offset": 0, "limit": ITEMS },
        "browseEndUserIntegration": INTEGRATION,
        "includeEpisodeContentRatingsV2": false,
    });
    detail(query::<Page>(session, "browsePage", variables).await?)
}

fn cards(data: Start) -> Result<Vec<Genre>> {
    let start = data
        .start
        .context("browseAll Pathfinder response has no browse")?;

    Ok(start
        .sections
        .items
        .into_iter()
        .flat_map(|section| section.items.items)
        .filter_map(|item| match item.content?.data? {
            Entity::BrowseSectionContainer(card) => genre(&item.uri, card),
            _ => None,
        })
        .collect())
}

fn detail(data: Page) -> Result<GenreDetail> {
    let browse = data
        .browse
        .context("browsePage Pathfinder response has no browse")?;
    let header = browse.header;

    Ok(GenreDetail {
        name: header
            .as_ref()
            .and_then(|header| header.title.as_ref())
            .map(|title| title.label.clone())
            .unwrap_or_default(),
        cover: header
            .as_ref()
            .and_then(|header| header.background.as_ref())
            .and_then(image),
        color: header
            .as_ref()
            .and_then(|header| header.color.as_ref())
            .and_then(|color| tint(&color.hex)),
        sections: browse
            .sections
            .items
            .into_iter()
            .filter_map(section)
            .collect(),
    })
}

fn section(section: Section) -> Option<GenreSection> {
    let title = section
        .data
        .title
        .map(|title| title.label)
        .unwrap_or_default();
    let items: Vec<GenreItem> = section.items.items.into_iter().filter_map(item).collect();

    (!items.is_empty()).then_some(GenreSection { title, items })
}

fn item(item: Item) -> Option<GenreItem> {
    let Item { uri, content } = item;
    match content?.data? {
        Entity::Playlist(playlist) => Some(GenreItem::Playlist(Playlist {
            id: trimmed(&uri, PLAYLIST_PREFIX)?,
            name: playlist.name,
            owner: playlist
                .owner
                .and_then(|owner| owner.data)
                .map(|data| data.name)
                .unwrap_or_default(),
            owned: false,
            collaborative: false,
            public: true,
            cover: playlist.images.items.first().and_then(image),
            track_count: 0,
            modified_at: None,
        })),
        Entity::Album(album) => {
            let artists: Vec<ArtistRef> = album
                .artists
                .items
                .into_iter()
                .filter_map(|artist| {
                    Some(ArtistRef {
                        name: artist.profile?.name,
                        id: trimmed(&artist.uri, ARTIST_PREFIX),
                    })
                })
                .collect();
            Some(GenreItem::Album(Album {
                id: trimmed(&uri, ALBUM_PREFIX)?,
                name: album.name,
                artists: joined(&artists),
                artist_refs: artists,
                cover: image(&album.cover),
                cover_large: None,
                release_type: ReleaseType::Album,
                year: 0,
                track_count: 0,
                release_date: String::new(),
                label: String::new(),
                copyrights: Vec::new(),
                added_at: None,
            }))
        }
        Entity::BrowseSectionContainer(card) => genre(&uri, card).map(GenreItem::Genre),
        Entity::Unknown => None,
    }
}

fn genre(uri: &str, container: WireCard) -> Option<Genre> {
    let id = trimmed(uri, PAGE_PREFIX)?;
    let card = container.data?.card?;

    Some(Genre {
        id,
        name: card.title.map(|title| title.label).unwrap_or_default(),
        cover: image(&card.artwork),
        color: card.background.and_then(|color| tint(&color.hex)),
    })
}

fn image(artwork: &Artwork) -> Option<String> {
    artwork
        .sources
        .first()
        .map(|source| source.url.clone())
        .filter(|url| !url.is_empty())
}

fn joined(artists: &[ArtistRef]) -> String {
    artists
        .iter()
        .map(|artist| artist.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

fn tint(hex: &str) -> Option<u32> {
    let digits = hex.strip_prefix('#').unwrap_or(hex);
    (digits.len() == 6)
        .then(|| u32::from_str_radix(digits, 16).ok())
        .flatten()
}

fn trimmed(uri: &str, prefix: &str) -> Option<String> {
    uri.strip_prefix(prefix)
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
}
