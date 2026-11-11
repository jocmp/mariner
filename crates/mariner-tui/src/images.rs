//! Article images, fetched and decoded in the background and kept for
//! previews.

use std::collections::HashMap;
use std::io::Cursor;
use std::sync::{Arc, Mutex};

use image::{ImageReader, Limits, RgbaImage};
use mariner_feeds::fetch::{Fetch, HttpFetcher, Request};
use tokio::runtime::Runtime;

/// Images are shrunk to this width once decoded: previews never need more.
const KEEP_WIDTH: u32 = 320;
const MAX_DIMENSION: u32 = 10_000;
const MAX_ALLOCATION: u64 = 256 * 1024 * 1024;

#[derive(Clone)]
pub enum Image {
    Loading,
    Ready(Arc<RgbaImage>),
    Failed,
}

/// Fetched images by URL, shared with the fetching tasks.
#[derive(Clone)]
pub struct Images {
    fetcher: HttpFetcher,
    images: Arc<Mutex<HashMap<String, Image>>>,
}

impl Images {
    pub fn new(fetcher: HttpFetcher) -> Self {
        Images {
            fetcher,
            images: Arc::default(),
        }
    }

    /// The image at `url`, if one was requested.
    pub fn get(&self, url: &str) -> Option<Image> {
        self.images.lock().unwrap().get(url).cloned()
    }

    /// Starts fetching each URL not requested before.
    pub fn request(&self, runtime: &Runtime, urls: impl IntoIterator<Item = String>) {
        for url in urls {
            if url.starts_with("data:") {
                continue;
            }
            {
                let mut images = self.images.lock().unwrap();
                if images.contains_key(&url) {
                    continue;
                }
                images.insert(url.clone(), Image::Loading);
            }
            let this = self.clone();
            runtime.spawn(async move {
                let image = match this.fetcher.fetch(&Request::get(&url)).await {
                    Ok(response) if response.is_success() => decode(&response.body),
                    _ => None,
                };
                let state = image.map_or(Image::Failed, |image| Image::Ready(Arc::new(image)));
                this.images.lock().unwrap().insert(url, state);
            });
        }
    }
}

#[cfg(test)]
impl Images {
    pub fn insert(&self, url: &str, image: Image) {
        self.images.lock().unwrap().insert(url.to_string(), image);
    }
}

/// Decodes within limits and shrinks the result to [`KEEP_WIDTH`].
fn decode(bytes: &[u8]) -> Option<RgbaImage> {
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_DIMENSION);
    limits.max_image_height = Some(MAX_DIMENSION);
    limits.max_alloc = Some(MAX_ALLOCATION);
    let mut reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;
    reader.limits(limits);
    let image = reader.decode().ok()?;
    let image = if image.width() > KEEP_WIDTH {
        image.resize(KEEP_WIDTH, u32::MAX, image::imageops::FilterType::Triangle)
    } else {
        image
    };
    Some(image.to_rgba8())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_and_shrinks_wide_images() {
        let mut png = Vec::new();
        image::RgbaImage::new(1_000, 500)
            .write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        let image = decode(&png).unwrap();
        assert_eq!(image.dimensions(), (KEEP_WIDTH, KEEP_WIDTH / 2));
        assert!(decode(b"not an image").is_none());
    }
}
