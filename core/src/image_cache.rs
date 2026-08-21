//! Downloads and caches strip images locally, so the daemon's `/image` route
//! (see the plan) can serve repeated requests — from any client, local or
//! remote — without re-fetching megatokyo.com every time.
//!
//! By the time a [`crate::domain::Strip`] exists, its `url` already points at
//! the correct file (the scraper resolved the gif/png/jpg extension probing
//! the original .NET version did — see `scraper::strips`), so this module's
//! only job is: fetch once, cache under the strip number, serve from disk
//! after that.

use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum ImageCacheError {
    #[error("could not download {url}: {source}")]
    Download {
        url: String,
        #[source]
        source: reqwest::Error,
    },
    #[error("{url} returned HTTP {status}")]
    Status { url: String, status: u16 },
    #[error("could not determine a file extension from {0}")]
    NoExtension(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub struct ImageCache {
    dir: PathBuf,
    client: reqwest::Client,
}

impl ImageCache {
    /// `$XDG_CACHE_HOME/megatokyo/strips`, falling back to
    /// `~/.cache/megatokyo/strips` per the XDG base dir spec.
    pub fn default_cache_dir() -> PathBuf {
        let cache_home = std::env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".cache")
            });
        cache_home.join("megatokyo").join("strips")
    }

    pub fn new(dir: PathBuf) -> Self {
        Self {
            dir,
            client: reqwest::Client::new(),
        }
    }

    /// Returns the already-cached path for `number`, if any file matching
    /// `<number>.<ext>` exists — checked before downloading anything.
    pub fn find_cached(&self, number: i32) -> Option<PathBuf> {
        for ext in ["png", "gif", "jpg", "jpeg"] {
            let path = self.dir.join(format!("{number}.{ext}"));
            if path.is_file() {
                return Some(path);
            }
        }
        None
    }

    /// Downloads `source_url` into the cache under `<number>.<ext>` (extension
    /// taken from `source_url` itself) unless already cached, and returns the
    /// local path either way.
    pub async fn ensure_cached(
        &self,
        number: i32,
        source_url: &str,
    ) -> Result<PathBuf, ImageCacheError> {
        if let Some(cached) = self.find_cached(number) {
            return Ok(cached);
        }
        let ext = extension_of(source_url)
            .ok_or_else(|| ImageCacheError::NoExtension(source_url.to_string()))?;

        let response = self.client.get(source_url).send().await.map_err(|source| {
            ImageCacheError::Download {
                url: source_url.to_string(),
                source,
            }
        })?;
        if !response.status().is_success() {
            return Err(ImageCacheError::Status {
                url: source_url.to_string(),
                status: response.status().as_u16(),
            });
        }
        let bytes = response
            .bytes()
            .await
            .map_err(|source| ImageCacheError::Download {
                url: source_url.to_string(),
                source,
            })?;

        std::fs::create_dir_all(&self.dir)?;
        let path = self.dir.join(format!("{number}.{ext}"));
        std::fs::write(&path, &bytes)?;
        Ok(path)
    }
}

/// The daemon's `/image` route uses this to set `Content-Type` when streaming
/// a cached file back to a client.
pub fn content_type_for(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("png") => "image/png",
        Some("gif") => "image/gif",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        _ => "application/octet-stream",
    }
}

fn extension_of(url: &str) -> Option<String> {
    let without_query = url.split(['?', '#']).next().unwrap_or(url);
    let ext = without_query.rsplit('.').next()?.to_ascii_lowercase();
    matches!(ext.as_str(), "png" | "gif" | "jpg" | "jpeg").then_some(ext)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn extension_of_reads_the_trailing_extension_and_ignores_query_strings() {
        assert_eq!(
            extension_of("https://megatokyo.com/strips/1619.png"),
            Some("png".to_string())
        );
        assert_eq!(
            extension_of("https://megatokyo.com/strips/1619.jpg?v=2"),
            Some("jpg".to_string())
        );
        assert_eq!(extension_of("https://megatokyo.com/strips/1619"), None);
        assert_eq!(
            extension_of("https://megatokyo.com/strips/1619.bogus"),
            None
        );
    }

    #[test]
    fn content_type_for_maps_known_extensions() {
        assert_eq!(content_type_for(Path::new("1619.png")), "image/png");
        assert_eq!(content_type_for(Path::new("1619.gif")), "image/gif");
        assert_eq!(content_type_for(Path::new("1619.jpg")), "image/jpeg");
        assert_eq!(
            content_type_for(Path::new("1619.weird")),
            "application/octet-stream"
        );
    }

    #[tokio::test]
    async fn ensure_cached_downloads_once_then_reuses_the_cached_file() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/1619.png"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"fake-png-bytes".to_vec()))
            .expect(1)
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let cache = ImageCache::new(dir.path().to_path_buf());
        let url = format!("{}/1619.png", server.uri());

        let first = cache.ensure_cached(1619, &url).await.unwrap();
        assert_eq!(std::fs::read(&first).unwrap(), b"fake-png-bytes");

        // Second call must not hit the server again (mock's `expect(1)`
        // fails the test on drop if it does) — it should find the file
        // ensure_cached already wrote.
        let second = cache.ensure_cached(1619, &url).await.unwrap();
        assert_eq!(first, second);
    }

    #[tokio::test]
    async fn ensure_cached_rejects_a_non_success_response() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/missing.png"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let cache = ImageCache::new(dir.path().to_path_buf());
        let url = format!("{}/missing.png", server.uri());

        let err = cache.ensure_cached(1, &url).await.unwrap_err();
        assert!(matches!(err, ImageCacheError::Status { status: 404, .. }));
    }
}
