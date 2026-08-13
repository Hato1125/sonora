use anyhow::Result;
use serde_json::{Value, json};
use ytmusic::nav::Nav as _;
use ytmusic::{Client, YtMusic, parse};

use crate::youtube::wire;
use crate::{Genre, GenreDetail, GenreItem, GenreSection};

const HOME: &str = "FEmusic_home";
const CATEGORIES: &str = "FEmusic_moods_and_genres";
const CATEGORY: &str = "FEmusic_moods_and_genres_category";
const THUMB: u32 = 120;

pub(crate) async fn home(api: &YtMusic) -> Result<Vec<GenreSection>> {
    let answer = api
        .execute("browse", Client::Music, json!({ "browseId": HOME }))
        .await?;

    Ok(parse::find_renderers(&answer, "musicCarouselShelfRenderer")
        .into_iter()
        .filter_map(section)
        .collect())
}

pub(crate) async fn genres(api: &YtMusic) -> Result<Vec<Genre>> {
    let answer = api
        .execute("browse", Client::Music, json!({ "browseId": CATEGORIES }))
        .await?;

    Ok(parse::find_renderers(&answer, "gridRenderer")
        .into_iter()
        .flat_map(|grid| grid.items(&["items"]))
        .filter_map(card)
        .collect())
}

pub(crate) async fn genre(api: &YtMusic, params: &str) -> Result<GenreDetail> {
    let answer = api
        .execute(
            "browse",
            Client::Music,
            json!({ "browseId": CATEGORY, "params": params }),
        )
        .await?;

    Ok(GenreDetail {
        name: answer
            .run_text(&["header", "musicHeaderRenderer", "title"])
            .unwrap_or_default(),
        sections: parse::find_renderers(&answer, "musicCarouselShelfRenderer")
            .into_iter()
            .filter_map(section)
            .collect(),
    })
}

fn section(shelf: &Value) -> Option<GenreSection> {
    let title = shelf
        .run_text(&["header", "musicCarouselShelfBasicHeaderRenderer", "title"])
        .unwrap_or_default();
    let items: Vec<GenreItem> = shelf.items(&["contents"]).iter().filter_map(item).collect();

    (!items.is_empty()).then_some(GenreSection { title, items })
}

fn item(node: &Value) -> Option<GenreItem> {
    if let Some(source) = parse::two_row_playlist(node) {
        let thumb = thumb(&source.thumbnails);
        let mut playlist = wire::playlist(source, false, true);
        playlist.cover = thumb;
        return Some(GenreItem::Playlist(playlist));
    }

    let source = parse::two_row_album(node)?;
    let thumb = thumb(&source.thumbnails);
    let mut album = wire::album(source);
    album.cover = thumb;

    Some(GenreItem::Album(album))
}

fn thumb(thumbnails: &[ytmusic::Thumbnail]) -> Option<String> {
    thumbnails
        .iter()
        .find(|thumb| thumb.width >= THUMB)
        .or_else(|| thumbnails.last())
        .map(|thumb| thumb.url.clone())
}

fn card(item: &Value) -> Option<Genre> {
    let button = item.get("musicNavigationButtonRenderer")?;

    Some(Genre {
        id: button
            .str_at(&["clickCommand", "browseEndpoint", "params"])?
            .to_owned(),
        name: button.run_text(&["buttonText"])?,
        cover: None,
    })
}
