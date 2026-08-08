// SPDX-License-Identifier: GPL-3.0-or-later

use gpui::{Hsla, SharedString};
use router::{Destination, navigate};
use spotify::ArtistRef;
use ui::{InlineLink, InlineLinks};

pub fn artist_links(
    id: impl Into<SharedString>,
    artists: Vec<ArtistRef>,
    fallback: impl Into<SharedString>,
    color: Hsla,
) -> InlineLinks {
    InlineLinks::new(
        id,
        artists
            .into_iter()
            .map(|artist| InlineLink::new(artist.name, artist.id.map(Into::into))),
        fallback,
        color,
    )
    .on_click(|id, cx| navigate(Destination::Artist(id), cx))
}
