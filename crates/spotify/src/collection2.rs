use anyhow::{Context as _, Result};
use http::header::{ACCEPT, CONTENT_TYPE};
use http::{HeaderMap, HeaderValue, Method};
use librespot_core::Session;

use crate::pb::{Reader, Value, Writer, text};

const PAGING: &str = "/collection/v2/paging";
const CONTENT: &str = "application/vnd.collection-v2.spotify.proto";
const SET: &str = "collection";
const PAGE: i32 = 300;

const REQUEST_USERNAME: u32 = 1;
const REQUEST_SET: u32 = 2;
const REQUEST_TOKEN: u32 = 3;
const REQUEST_LIMIT: u32 = 4;

const RESPONSE_ITEMS: u32 = 1;
const RESPONSE_TOKEN: u32 = 2;

const ITEM_URI: u32 = 1;
const ITEM_REMOVED: u32 = 3;

struct Page {
    uris: Vec<String>,
    next: String,
}

pub async fn saved_uris(session: &Session, prefix: &str, limit: usize) -> Result<Vec<String>> {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static(CONTENT));
    headers.insert(ACCEPT, HeaderValue::from_static(CONTENT));

    let username = session.username();
    let mut found = Vec::new();
    let mut token = String::new();

    loop {
        let body = request(&username, &token);
        let raw = session
            .spclient()
            .request(&Method::POST, PAGING, Some(headers.clone()), Some(&body))
            .await
            .context("cannot read the saved collection")?;

        let page = response(&raw).context("cannot decode the collection page")?;

        found.extend(
            page.uris
                .into_iter()
                .filter(|uri| uri.starts_with(prefix)),
        );

        token = page.next;
        if found.len() >= limit || token.is_empty() {
            break;
        }
    }

    found.truncate(limit);
    Ok(found)
}

fn request(username: &str, token: &str) -> Vec<u8> {
    let mut writer = Writer::default();
    writer.string(REQUEST_USERNAME, username);
    writer.string(REQUEST_SET, SET);
    writer.string(REQUEST_TOKEN, token);
    writer.int32(REQUEST_LIMIT, PAGE);
    writer.finish()
}

fn response(bytes: &[u8]) -> Result<Page> {
    let mut page = Page {
        uris: Vec::new(),
        next: String::new(),
    };
    let mut reader = Reader::new(bytes);

    while let Some((field, value)) = reader.field()? {
        match (field, value) {
            (RESPONSE_ITEMS, Value::Bytes(item)) => page.uris.extend(kept(item)?),
            (RESPONSE_TOKEN, Value::Bytes(token)) => page.next = text(token)?,
            _ => (),
        }
    }

    Ok(page)
}

fn kept(bytes: &[u8]) -> Result<Option<String>> {
    let mut uri = None;
    let mut removed = false;
    let mut reader = Reader::new(bytes);

    while let Some((field, value)) = reader.field()? {
        match (field, value) {
            (ITEM_URI, Value::Bytes(raw)) => uri = Some(text(raw)?),
            (ITEM_REMOVED, Value::Varint(flag)) => removed = flag != 0,
            _ => (),
        }
    }

    Ok(uri.filter(|_| !removed))
}
