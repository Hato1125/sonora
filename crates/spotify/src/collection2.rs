use anyhow::{Context as _, Result};
use http::header::{ACCEPT, CONTENT_TYPE};
use http::{HeaderMap, HeaderValue, Method};
use librespot_core::Session;
use protobuf::Message as _;

use crate::protos::collection2v2::{PageRequest, PageResponse};

const PAGING: &str = "/collection/v2/paging";
const CONTENT: &str = "application/vnd.collection-v2.spotify.proto";
const SET: &str = "collection";
const PAGE: i32 = 300;

pub async fn saved_uris(session: &Session, prefix: &str, limit: usize) -> Result<Vec<String>> {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static(CONTENT));
    headers.insert(ACCEPT, HeaderValue::from_static(CONTENT));

    let mut found = Vec::new();
    let mut token = String::new();

    loop {
        let mut request = PageRequest::new();
        request.username = session.username();
        request.set = SET.to_owned();
        request.limit = PAGE;
        request.pagination_token = token.clone();

        let body = request.write_to_bytes()?;
        let raw = session
            .spclient()
            .request(&Method::POST, PAGING, Some(headers.clone()), Some(&body))
            .await
            .context("cannot read the saved collection")?;

        let page =
            PageResponse::parse_from_bytes(&raw).context("cannot decode the collection page")?;

        found.extend(
            page.items
                .iter()
                .filter(|item| !item.is_removed)
                .map(|item| item.uri.as_str())
                .filter(|uri| uri.starts_with(prefix))
                .map(str::to_owned),
        );

        token = page.next_page_token;
        if found.len() >= limit || token.is_empty() {
            break;
        }
    }

    found.truncate(limit);
    Ok(found)
}
