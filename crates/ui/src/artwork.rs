use crate::metrics::snapped;
use crate::skeleton::Skeleton;
use crate::theme::ActiveTheme as _;
use gpui::prelude::*;
use gpui::{
    App, Context, Div, Entity, Global, Hsla, ImageCache, ImageCacheError, ImageSource,
    ImgResourceLoader, Interactivity, ObjectFit, Pixels, RenderImage, Resource, SharedString,
    SharedUri, StyleRefinement, Styled, Task, Window, div, img, px, svg,
};
use image::{Frame, RgbaImage, imageops};
use std::path::Path;
use std::time::{Duration, Instant};
use std::{collections::HashMap, sync::Arc};

const FILE_PREFIX: &str = "file://";

const FALLBACK_ICON: &str = "icons/music.svg";
pub(crate) const ROUNDED: Pixels = px(4.);
const CACHE_BYTES: usize = 32 * 1024 * 1024;
const CACHE_ITEMS: usize = 256;
const HARD_BYTES: usize = 192 * 1024 * 1024;
const SAMPLED_BYTES: usize = 64 * 1024 * 1024;
const SAMPLED_ITEMS: usize = 256;
const MAX_SAMPLE_EDGE: u32 = 1024;
const GRACE: Duration = Duration::from_secs(5);
const KEEP_ITEMS: usize = 96;
const IDLE: Duration = Duration::from_secs(120);
const ORPHAN: Duration = Duration::from_secs(20);
const SWEEP: Duration = Duration::from_secs(30);
const SOFT_ITEMS: usize = 8;
const SOFT_SIGMA: f32 = 1.6;
const SMALL_BYTES: usize = 64 * 1024;
const BIG_BYTES: usize = 256 * 1024;

struct Cached {
    value: Result<Arc<RenderImage>, ImageCacheError>,
    bytes: usize,
    used: Instant,
}

struct Sampled {
    image: Arc<RenderImage>,
    bytes: usize,
    used: Instant,
}

struct ArtworkCache {
    items: HashMap<Resource, Cached>,
    pending: HashMap<Resource, Instant>,
    sampled: HashMap<(Resource, u32), Sampled>,
    soft: HashMap<(Resource, u32), Arc<RenderImage>>,
    bytes: usize,
    sampled_bytes: usize,
    _sweep: Task<()>,
}

struct Installed(Entity<ArtworkCache>);

impl Global for Installed {}

impl ArtworkCache {
    fn entity(cx: &mut App) -> Entity<Self> {
        if cx.try_global::<Installed>().is_none() {
            let cache = cx.new(|cx| Self {
                items: HashMap::new(),
                pending: HashMap::new(),
                sampled: HashMap::new(),
                soft: HashMap::new(),
                bytes: 0,
                sampled_bytes: 0,
                _sweep: sweeper(cx),
            });
            cx.set_global(Installed(cache));
        }
        cx.global::<Installed>().0.clone()
    }

    fn insert(
        &mut self,
        resource: Resource,
        value: Result<Arc<RenderImage>, ImageCacheError>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let bytes = value.as_ref().map_or(0, |image| image_bytes(image));
        self.bytes = self.bytes.saturating_add(bytes);
        self.items.insert(
            resource,
            Cached {
                value,
                bytes,
                used: Instant::now(),
            },
        );

        while self.items.len() > 1 {
            let forced = self.bytes > HARD_BYTES;
            if !forced && self.bytes <= CACHE_BYTES && self.items.len() <= CACHE_ITEMS {
                break;
            }
            let Some((resource, used)) = self.oldest() else {
                break;
            };
            if !forced && used.elapsed() < GRACE {
                break;
            }
            self.evict(&resource, Some(&mut *window), cx);
        }
    }

    fn oldest(&self) -> Option<(Resource, Instant)> {
        self.items
            .iter()
            .min_by_key(|(_, cached)| cached.used)
            .map(|(resource, cached)| (resource.clone(), cached.used))
    }

    fn evict(&mut self, resource: &Resource, window: Option<&mut Window>, cx: &mut App) {
        let Some(cached) = self.items.remove(resource) else {
            return;
        };
        self.bytes = self.bytes.saturating_sub(cached.bytes);
        cx.remove_asset::<ImgResourceLoader>(resource);
        if let Ok(image) = cached.value {
            cx.drop_image(image, window);
        }
    }

    fn prepared(&mut self, resource: &Resource, edge: u32, soft: bool) -> Option<Arc<RenderImage>> {
        let key = (resource.clone(), edge);
        if soft {
            return self.soft.get(&key).cloned();
        }
        let sampled = self.sampled.get_mut(&key)?;
        sampled.used = Instant::now();
        Some(sampled.image.clone())
    }

    fn prepare(
        &mut self,
        resource: &Resource,
        edge: u32,
        soft: bool,
        image: Arc<RenderImage>,
        window: &mut Window,
        cx: &mut App,
    ) -> Arc<RenderImage> {
        let image = match edge {
            0 => image,
            edge => self.sample(resource, edge, image, window, cx),
        };
        if !soft {
            return image;
        }

        let key = (resource.clone(), edge);
        if let Some(found) = self.soft.get(&key) {
            return found.clone();
        }
        if self.soft.len() >= SOFT_ITEMS {
            for image in self.soft.drain().map(|(_, image)| image) {
                cx.drop_image(image, Some(&mut *window));
            }
        }
        let Some(softened) = blurred(&image) else {
            return image;
        };
        self.soft.insert(key, softened.clone());
        softened
    }

    fn sample(
        &mut self,
        resource: &Resource,
        edge: u32,
        image: Arc<RenderImage>,
        window: &mut Window,
        cx: &mut App,
    ) -> Arc<RenderImage> {
        let key = (resource.clone(), edge);
        if let Some(found) = self.sampled.get_mut(&key) {
            found.used = Instant::now();
            return found.image.clone();
        }
        let Some(image) = downsampled(&image, edge) else {
            return image;
        };
        let bytes = image_bytes(&image);
        self.sampled_bytes = self.sampled_bytes.saturating_add(bytes);
        self.sampled.insert(
            key.clone(),
            Sampled {
                image: image.clone(),
                bytes,
                used: Instant::now(),
            },
        );

        while self.sampled.len() > 1
            && (self.sampled.len() > SAMPLED_ITEMS || self.sampled_bytes > SAMPLED_BYTES)
        {
            let Some(oldest) = self
                .sampled
                .iter()
                .min_by_key(|(_, sampled)| sampled.used)
                .map(|(key, _)| key.clone())
            else {
                break;
            };
            let Some(sampled) = self.sampled.remove(&oldest) else {
                break;
            };
            self.sampled_bytes = self.sampled_bytes.saturating_sub(sampled.bytes);
            cx.drop_image(sampled.image, Some(&mut *window));
        }

        image
    }

    fn sweep(&mut self, cx: &mut App) {
        let held = self.items.len();
        let abandoned: Vec<Resource> = self
            .pending
            .iter()
            .filter(|(_, started)| started.elapsed() > ORPHAN)
            .map(|(resource, _)| resource.clone())
            .collect();

        for resource in &abandoned {
            self.pending.remove(resource);
            cx.remove_asset::<ImgResourceLoader>(resource);
        }

        let mut ages: Vec<(Resource, Instant, usize)> = self
            .items
            .iter()
            .map(|(resource, cached)| (resource.clone(), cached.used, cached.bytes))
            .collect();
        ages.sort_unstable_by_key(|(_, used, _)| *used);

        let idle = ages
            .iter()
            .filter(|(_, used, _)| used.elapsed() > IDLE)
            .count();
        let protected = ages.len().saturating_sub(KEEP_ITEMS);
        let mut bytes = self.bytes;
        let mut stale = Vec::new();

        for (index, (resource, used, size)) in ages.iter().enumerate() {
            if index >= protected || used.elapsed() <= GRACE {
                break;
            }
            if bytes <= CACHE_BYTES && used.elapsed() <= IDLE {
                break;
            }
            stale.push(resource.clone());
            bytes = bytes.saturating_sub(*size);
        }

        for resource in &stale {
            self.evict(resource, None, cx);
        }

        let tiny = self.count(..SMALL_BYTES);
        let small = self.count(SMALL_BYTES..BIG_BYTES);
        let big = self.count(BIG_BYTES..);

        log::debug!(
            "artwork: {} originals / {} KiB, {} sampled / {} KiB, dropped {}, idle {idle}, abandoned {}, waiting {}, sizes {tiny}/{small}/{big}",
            self.items.len(),
            self.bytes / 1024,
            self.sampled.len(),
            self.sampled_bytes / 1024,
            held - self.items.len(),
            abandoned.len(),
            self.pending.len()
        );
    }

    fn count(&self, range: impl std::ops::RangeBounds<usize>) -> usize {
        self.items
            .values()
            .filter(|cached| range.contains(&cached.bytes))
            .count()
    }
}

fn sweeper(cx: &mut Context<ArtworkCache>) -> Task<()> {
    cx.spawn(async move |this, cx| {
        loop {
            cx.background_executor().timer(SWEEP).await;
            if this.update(cx, |this, cx| this.sweep(cx)).is_err() {
                return;
            }
        }
    })
}

impl ImageCache for ArtworkCache {
    fn load(
        &mut self,
        resource: &Resource,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<Result<Arc<RenderImage>, ImageCacheError>> {
        if let Some(cached) = self.items.get_mut(resource) {
            cached.used = Instant::now();
            return Some(cached.value.clone());
        }

        let Some(value) = window.use_asset::<ImgResourceLoader>(resource, cx) else {
            self.pending.insert(resource.clone(), Instant::now());
            return None;
        };

        self.pending.remove(resource);
        self.insert(resource.clone(), value.clone(), window, cx);
        Some(value)
    }
}

fn blurred(image: &RenderImage) -> Option<Arc<RenderImage>> {
    let frames: Vec<Frame> = (0..image.frame_count())
        .filter_map(|index| {
            let size = image.size(index);
            let width = size.width.0.max(0) as u32;
            let height = size.height.0.max(0) as u32;
            let bytes = image.as_bytes(index)?.to_vec();
            let whole = RgbaImage::from_raw(width, height, bytes)?;

            Some(Frame::from_parts(
                imageops::fast_blur(&whole, SOFT_SIGMA),
                0,
                0,
                image.delay(index),
            ))
        })
        .collect();
    if frames.len() != image.frame_count() {
        log::warn!("artwork: cannot soften an image");
        return None;
    }

    Some(Arc::new(RenderImage::new(frames)))
}

fn sample_edge(size: Pixels, window: &Window) -> u32 {
    let physical = ((size / px(1.)) * window.scale_factor()).ceil().max(1.) as u32;
    physical
        .checked_next_power_of_two()
        .filter(|edge| *edge <= MAX_SAMPLE_EDGE)
        .unwrap_or(0)
}

fn downsampled(image: &RenderImage, edge: u32) -> Option<Arc<RenderImage>> {
    if edge == 0 || image.frame_count() == 0 {
        return None;
    }
    let frames: Vec<Frame> = (0..image.frame_count())
        .filter_map(|index| {
            let size = image.size(index);
            let width = size.width.0.max(0) as u32;
            let height = size.height.0.max(0) as u32;
            let side = width.min(height);
            if side <= edge {
                return None;
            }
            let bytes = image.as_bytes(index)?.to_vec();
            let whole = RgbaImage::from_raw(width, height, bytes)?;
            let square =
                imageops::crop_imm(&whole, (width - side) / 2, (height - side) / 2, side, side);
            let sampled = match side > edge.saturating_mul(2) {
                true => imageops::thumbnail(&*square, edge, edge),
                false => imageops::resize(&*square, edge, edge, imageops::FilterType::Triangle),
            };

            Some(Frame::from_parts(sampled, 0, 0, image.delay(index)))
        })
        .collect();
    if frames.len() != image.frame_count() {
        return None;
    }
    Some(Arc::new(RenderImage::new(frames)))
}

pub(crate) fn resource(url: impl Into<SharedString>) -> Resource {
    let url = url.into();
    match url.strip_prefix(FILE_PREFIX) {
        Some(path) => Resource::Path(Arc::from(Path::new(path))),
        None => Resource::Uri(SharedUri::from(url)),
    }
}

pub fn artwork_usage(cx: &App) -> Option<(usize, usize)> {
    let installed = cx.try_global::<Installed>()?;
    let cache = installed.0.read(cx);
    let soft_bytes: usize = cache.soft.values().map(|image| image_bytes(image)).sum();
    Some((
        cache.items.len() + cache.sampled.len() + cache.soft.len(),
        cache.bytes + cache.sampled_bytes + soft_bytes,
    ))
}

fn image_bytes(image: &RenderImage) -> usize {
    (0..image.frame_count())
        .filter_map(|frame| image.as_bytes(frame))
        .fold(0, |bytes, frame| bytes.saturating_add(frame.len()))
}

#[derive(IntoElement)]
pub struct Avatar {
    art: Artwork,
}

impl Avatar {
    #[track_caller]
    pub fn new(url: Option<impl Into<SharedString>>) -> Self {
        Self {
            art: Artwork::new(url).circle().flex_none(),
        }
    }

    pub fn size(mut self, size: Pixels) -> Self {
        self.art = self.art.size(size);
        self
    }
}

impl Styled for Avatar {
    fn style(&mut self) -> &mut StyleRefinement {
        self.art.style()
    }
}

impl RenderOnce for Avatar {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        self.art
    }
}

#[derive(IntoElement)]
pub struct Artwork {
    url: Option<SharedString>,
    size: Pixels,
    circle: bool,
    radius: Option<Pixels>,
    fallback: SharedString,
    accent: bool,
    soft: bool,
    interactivity: Interactivity,
}

impl Artwork {
    #[track_caller]
    pub fn new(url: Option<impl Into<SharedString>>) -> Self {
        Self {
            url: url.map(Into::into),
            size: px(28.),
            circle: false,
            radius: None,
            soft: false,
            fallback: FALLBACK_ICON.into(),
            accent: false,
            interactivity: Interactivity::new(),
        }
    }

    pub fn size(mut self, size: Pixels) -> Self {
        self.size = size;
        self
    }

    pub fn circle(mut self) -> Self {
        self.circle = true;
        self
    }

    pub fn corner_radius(mut self, radius: Pixels) -> Self {
        self.radius = Some(radius);
        self
    }

    pub fn fallback(mut self, icon: impl Into<SharedString>) -> Self {
        self.fallback = icon.into();
        self
    }

    pub fn soft(mut self, soft: bool) -> Self {
        self.soft = soft;
        self
    }

    pub fn accent(mut self) -> Self {
        self.accent = true;
        self
    }
}

impl Styled for Artwork {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.interactivity.base_style
    }
}

impl InteractiveElement for Artwork {
    fn interactivity(&mut self) -> &mut Interactivity {
        &mut self.interactivity
    }
}

impl RenderOnce for Artwork {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Self {
            url,
            size,
            circle,
            radius,
            fallback,
            accent,
            soft,
            interactivity,
        } = self;
        let theme = *cx.theme();
        let muted = theme.muted_foreground;
        let glyph = match accent {
            true => theme.tint.unwrap_or(theme.primary),
            false => muted.opacity(0.5),
        };
        let size = snapped(size, window);
        let rounded = match (circle, radius) {
            (true, _) => size / 2.,
            (false, Some(radius)) => radius,
            (false, None) => cx.theme().radius.min(ROUNDED),
        };
        let placeholder = {
            let fallback = fallback.clone();
            move || blank(size, rounded, muted, glyph, fallback.clone()).into_any_element()
        };

        match url {
            Some(url) => {
                let cache = ArtworkCache::entity(cx);
                let resource = resource(url);
                let edge = sample_edge(size, window);
                let source = ImageSource::Custom(Arc::new({
                    let cache = cache.clone();
                    move |window, cx| {
                        if let Some(prepared) =
                            cache.update(cx, |cache, _| cache.prepared(&resource, edge, soft))
                        {
                            return Some(Ok(prepared));
                        }
                        let loaded = cache
                            .update(cx, |cache, cx| cache.load(&resource, window, cx))?
                            .map(|image| {
                                cache.update(cx, |cache, cx| {
                                    cache.prepare(&resource, edge, soft, image, window, cx)
                                })
                            });
                        Some(loaded)
                    }
                }));
                refined(
                    img(source)
                        .image_cache(&cache)
                        .size(size)
                        .object_fit(ObjectFit::Cover)
                        .rounded(rounded)
                        .with_loading(move || {
                            Skeleton::new()
                                .size(size)
                                .rounded(rounded)
                                .into_any_element()
                        })
                        .with_fallback(placeholder),
                    interactivity,
                )
                .into_any_element()
            }
            None => refined(blank(size, rounded, muted, glyph, fallback), interactivity)
                .into_any_element(),
        }
    }
}

fn refined<T: Styled + InteractiveElement>(mut element: T, mut caller: Interactivity) -> T {
    let mut style = std::mem::take(element.style());
    style.refine(&caller.base_style);
    *caller.base_style = style;
    *element.interactivity() = caller;
    element
}

fn blank(size: Pixels, rounded: Pixels, muted: Hsla, glyph: Hsla, fallback: SharedString) -> Div {
    div()
        .size(size)
        .rounded(rounded)
        .bg(muted.opacity(0.12))
        .flex()
        .items_center()
        .justify_center()
        .child(svg().path(fallback).size(size * 0.46).text_color(glyph))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Delay, Rgba};

    fn rendered(width: u32, height: u32) -> RenderImage {
        RenderImage::new(vec![Frame::from_parts(
            RgbaImage::from_pixel(width, height, Rgba([20, 40, 60, 255])),
            0,
            0,
            Delay::from_numer_denom_ms(80, 1),
        )])
    }

    #[test]
    fn downsampling_crops_to_the_square_artwork_surface() {
        let sampled = downsampled(&rendered(240, 120), 64).unwrap();

        assert_eq!(sampled.size(0).width.0, 64);
        assert_eq!(sampled.size(0).height.0, 64);
        assert_eq!(sampled.delay(0), Delay::from_numer_denom_ms(80, 1));
    }

    #[test]
    fn downsampling_never_upscales_a_source() {
        assert!(downsampled(&rendered(60, 100), 64).is_none());
    }
}
