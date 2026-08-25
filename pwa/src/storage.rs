//! Persists the remote daemon's base URL and auth token in `localStorage`.
//! The PWA has no way to spawn a local daemon process the way the desktop
//! GUI can, so it always operates in "remote daemon" mode — these are the
//! only two settings it needs, entered once by the user.

use gloo_storage::{LocalStorage, Storage};

const BASE_URL_KEY: &str = "megatokyo_daemon_base_url";
const TOKEN_KEY: &str = "megatokyo_daemon_token";

#[derive(Debug, Clone, Default)]
pub struct DaemonLink {
    pub base_url: String,
    pub token: String,
}

pub fn load() -> DaemonLink {
    DaemonLink {
        base_url: LocalStorage::get(BASE_URL_KEY).unwrap_or_default(),
        token: LocalStorage::get(TOKEN_KEY).unwrap_or_default(),
    }
}

pub fn save(link: &DaemonLink) {
    let _ = LocalStorage::set(BASE_URL_KEY, &link.base_url);
    let _ = LocalStorage::set(TOKEN_KEY, &link.token);
}
