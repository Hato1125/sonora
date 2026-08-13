use anyhow::Result;
use serde_json::{Value, json};
use ytmusic::nav::Nav as _;
use ytmusic::{Client, YtMusic, parse};

use crate::youtube::wire;
use crate::{Genre, GenreDetail, GenreItem, GenreSection};

const CATEGORIES: &str = "FEmusic_moods_and_genres";
const CATEGORY: &str = "FEmusic_moods_and_genres_category";
const RGB: u64 = 0x00ff_ffff;

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
    if let Some(playlist) = parse::two_row_playlist(node) {
        return Some(GenreItem::Playlist(wire::playlist(playlist, false, true)));
    }

    parse::two_row_album(node)
        .map(wire::album)
        .map(GenreItem::Album)
}

fn card(item: &Value) -> Option<Genre> {
    let button = item.get("musicNavigationButtonRenderer")?;

    Some(Genre {
        id: button
            .str_at(&["clickCommand", "browseEndpoint", "params"])?
            .to_owned(),
        name: button.run_text(&["buttonText"])?,
        cover: None,
        color: button
            .at(&["solid", "leftStripeColor"])
            .and_then(Value::as_u64)
            .map(|color| (color & RGB) as u32),
    })
}
