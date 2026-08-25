//! Thin client for the `megatokyo-daemon` HTTP API, mirroring the
//! `x-megatokyo-daemon-token` header convention the desktop GUI already uses
//! (see `gui/src/background.rs`'s `fetch_status`).

use megatokyo_core::domain::Chapter;

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
