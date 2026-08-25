//! Thin client for the `megatokyo-daemon` HTTP API, mirroring the
//! `x-megatokyo-daemon-token` header convention the desktop GUI already uses
//! (see `gui/src/background.rs`'s `fetch_status`).

use megatokyo_core::domain::Chapter;
use serde::Deserialize;

const TOKEN_HEADER: &str = "x-megatokyo-daemon-token";

pub async fn fetch_chapters(base_url: &str, token: &str) -> Result<Vec<Chapter>, String> {
    let url = format!("{}/chapters", base_url.trim_end_matches('/'));
    let response = gloo_net::http::Request::get(&url)
        .header(TOKEN_HEADER, token)
        .send()
        .await
        .map_err(|err| err.to_string())?;

    if !response.ok() {
        return Err(format!("daemon returned {}", response.status()));
    }

    response
        .json::<Vec<Chapter>>()
        .await
        .map_err(|err| err.to_string())
}

#[derive(Deserialize)]
struct VapidPublicKeyResponse {
    vapid_public_key: String,
}

pub async fn fetch_vapid_public_key(base_url: &str, token: &str) -> Result<String, String> {
    let url = format!("{}/push/vapid-public-key", base_url.trim_end_matches('/'));
    let response = gloo_net::http::Request::get(&url)
        .header(TOKEN_HEADER, token)
        .send()
        .await
        .map_err(|err| err.to_string())?;

    if !response.ok() {
        return Err(format!("daemon returned {}", response.status()));
    }

    response
        .json::<VapidPublicKeyResponse>()
        .await
        .map(|body| body.vapid_public_key)
        .map_err(|err| err.to_string())
}

/// `endpoint`/`p256dh`/`auth` are exactly what the browser's
/// `PushSubscription.toJSON()` gives — see `push::subscribe`. Only
/// `endpoint` needs percent-encoding: `p256dh`/`auth` are already
/// URL-safe base64.
pub async fn push_subscribe(
    base_url: &str,
    token: &str,
    endpoint: &str,
    p256dh: &str,
    auth: &str,
) -> Result<(), String> {
    let encoded_endpoint = js_sys::encode_uri_component(endpoint);
    let url = format!(
        "{}/push/subscribe?endpoint={encoded_endpoint}&p256dh={p256dh}&auth={auth}",
        base_url.trim_end_matches('/'),
    );
    let response = gloo_net::http::Request::post(&url)
        .header(TOKEN_HEADER, token)
        .send()
        .await
        .map_err(|err| err.to_string())?;

    if !response.ok() {
        return Err(format!("daemon returned {}", response.status()));
    }
    Ok(())
}
