pub mod domain;

// `local_ctrl` uses `std::os::unix` for owner-only config-file permissions,
// which doesn't exist on wasm32 — and it's server-side (daemon) / local-ctrl
// (gui) plumbing that the wasm-targeted `pwa` crate has no use for anyway.
#[cfg(feature = "native")]
pub mod feed;
#[cfg(feature = "native")]
pub mod image_cache;
#[cfg(feature = "native")]
pub mod local_ctrl;
#[cfg(feature = "native")]
pub mod scraper;
#[cfg(feature = "native")]
pub mod store;
#[cfg(feature = "native")]
pub mod translate;
