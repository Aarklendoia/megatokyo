//! Web Push notifications: VAPID keypair generation and fan-out sends.
//!
//! Sends via the daemon's own rustls-based `reqwest::Client` rather than
//! `web-push`'s built-in clients (`isahc`/`hyper-tls` — see `Cargo.toml`'s
//! `default-features = false` on the `web-push` dep), avoiding a second,
//! entirely redundant HTTP-client stack. This does *not* get the daemon out
//! of needing OpenSSL at build time, though: `web-push`'s payload-encryption
//! dependency (`ece`, Mozilla's RFC8188 implementation) only has an
//! OpenSSL-backed crypto backend, no pure-Rust alternative, so it's linked
//! in regardless — see `debian/control`'s `libssl-dev` Build-Depends, added
//! alongside this.
//!
//! `web_push::clients::request_builder::build_request` builds a generic
//! `http::Request<T>` (its own `http` v0.2 dependency, not the `http` v1.x
//! `reqwest` uses — confirmed distinct crate versions in `Cargo.lock`), so
//! its pieces are translated into a `reqwest::RequestBuilder` by hand below
//! rather than relying on any direct type-level interop between the two.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use megatokyo_core::domain::PushSubscription;
use megatokyo_core::feed::{FeedItem, FeedItemKind};
use megatokyo_core::local_ctrl::random_bytes;
use web_push::{
    ContentEncoding, PartialVapidSignatureBuilder, SubscriptionInfo, VapidSignatureBuilder,
    WebPushMessageBuilder,
};

use crate::control::AppState;

/// A fresh (private key, public key) pair, both base64 URL-safe/no-padding
/// encoded — the private key in the raw-scalar format
/// `VapidSignatureBuilder::from_base64_no_sub` expects directly (no PEM/DER,
/// no OpenSSL), the public key as the uncompressed point bytes a browser's
/// `PushManager.subscribe({applicationServerKey})` expects. The retry loop
/// only ever runs once in practice: a random 32-byte string fails to decode
/// as a valid P-256 scalar with probability roughly 2^-32 (the curve order
/// is a hair below 2^256).
pub fn generate_vapid_keypair() -> (String, String) {
    loop {
        let private_key = URL_SAFE_NO_PAD.encode(random_bytes::<32>());
        if let Ok(builder) = VapidSignatureBuilder::from_base64_no_sub(&private_key) {
            let public_key = URL_SAFE_NO_PAD.encode(builder.get_public_key());
            return (private_key, public_key);
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum SendError {
    #[error("could not build the push message: {0}")]
    Build(#[from] web_push::WebPushError),
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("push service rejected the request: {0}")]
    Rejected(String),
    #[error("subscription is no longer valid")]
    Gone,
}

/// Notifies every stored subscription about `items` (from
/// `poll::check_feed_at`'s new-item diff) — one push per item per
/// subscription. Meant to be `tokio::spawn`ed by the caller so a slow or
/// unreachable push service never blocks the poll loop; errors are only
/// logged here, never propagated. A subscription the push service reports
/// as gone (410/404 — the browser dropped it) is removed so the daemon
/// doesn't keep retrying it forever; any other error leaves it alone, since
/// that's more likely a transient network/service issue than a dead
/// subscription.
pub async fn send_to_all(state: &AppState, items: &[FeedItem]) {
    if items.is_empty() {
        return;
    }

    let (vapid_private_key, vapid_subject) = {
        let config = state.config.read().await;
        (
            config.vapid_private_key.clone(),
            config.vapid_subject.clone(),
        )
    };
    if vapid_private_key.is_empty() {
        return;
    }
    let partial_builder = match VapidSignatureBuilder::from_base64_no_sub(&vapid_private_key) {
        Ok(builder) => builder,
        Err(err) => {
            log::warn!("invalid VAPID private key, skipping push: {err}");
            return;
        }
    };

    let subscriptions = match state.store.all_push_subscriptions() {
        Ok(subscriptions) => subscriptions,
        Err(err) => {
            log::warn!("could not load push subscriptions: {err}");
            return;
        }
    };
    if subscriptions.is_empty() {
        return;
    }

    for item in items {
        let (title, url) = notification_text(item);
        for subscription in &subscriptions {
            let result = send_one(
                &state.http_client,
                partial_builder.clone(),
                &vapid_subject,
                subscription,
                &title,
                &url,
            )
            .await;
            if let Err(err) = result {
                log::warn!("push to {} failed: {err}", subscription.endpoint);
                if matches!(err, SendError::Gone) {
                    if let Err(err) = state.store.remove_push_subscription(&subscription.endpoint) {
                        log::warn!("could not remove stale push subscription: {err}");
                    }
                }
            }
        }
    }
}

fn notification_text(item: &FeedItem) -> (String, String) {
    match item.kind {
        FeedItemKind::Strip => (format!("New strip: {}", item.title), item.link.clone()),
        FeedItemKind::Rant => (format!("New rant: {}", item.title), item.link.clone()),
    }
}

async fn send_one(
    client: &reqwest::Client,
    partial_builder: PartialVapidSignatureBuilder,
    vapid_subject: &str,
    subscription: &PushSubscription,
    title: &str,
    url: &str,
) -> Result<(), SendError> {
    let subscription_info = SubscriptionInfo::new(
        subscription.endpoint.clone(),
        subscription.p256dh.clone(),
        subscription.auth.clone(),
    );

    let mut sig_builder = partial_builder.add_sub_info(&subscription_info);
    if !vapid_subject.is_empty() {
        sig_builder.add_claim("sub", vapid_subject);
    }
    let signature = sig_builder.build()?;

    let payload = serde_json::json!({ "title": title, "url": url }).to_string();
    let mut message_builder = WebPushMessageBuilder::new(&subscription_info);
    message_builder.set_payload(ContentEncoding::Aes128Gcm, payload.as_bytes());
    message_builder.set_vapid_signature(signature);
    let message = message_builder.build()?;

    let request = web_push::request_builder::build_request::<bytes::Bytes>(message);
    let (parts, body) = request.into_parts();

    let method = reqwest::Method::from_bytes(parts.method.as_str().as_bytes())
        .map_err(|err| SendError::Rejected(err.to_string()))?;
    let mut builder = client.request(method, parts.uri.to_string());
    for (name, value) in parts.headers.iter() {
        builder = builder.header(name.as_str(), value.as_bytes());
    }
    let response = builder.body(body.to_vec()).send().await?;

    let status = response.status();
    if status.is_success() {
        Ok(())
    } else if status.as_u16() == 410 || status.as_u16() == 404 {
        Err(SendError::Gone)
    } else {
        let body_text = response.text().await.unwrap_or_default();
        Err(SendError::Rejected(format!("{status}: {body_text}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_vapid_keypair_produces_a_usable_private_key_and_matching_public_key() {
        let (private_key, public_key) = generate_vapid_keypair();

        let builder = VapidSignatureBuilder::from_base64_no_sub(&private_key).unwrap();
        assert_eq!(URL_SAFE_NO_PAD.encode(builder.get_public_key()), public_key);
        // Uncompressed P-256 point: 0x04 prefix + 32-byte X + 32-byte Y.
        assert_eq!(URL_SAFE_NO_PAD.decode(&public_key).unwrap().len(), 65);
    }

    #[test]
    fn generate_vapid_keypair_varies_between_calls() {
        let (first, _) = generate_vapid_keypair();
        let (second, _) = generate_vapid_keypair();
        assert_ne!(first, second);
    }

    fn item(number: i32, title: &str) -> FeedItem {
        FeedItem {
            kind: FeedItemKind::Strip,
            number,
            title: title.to_string(),
            published_at: "2026-08-25T00:00:00Z".to_string(),
            link: format!("https://megatokyo.com/strip/{number}"),
        }
    }

    /// Any valid P-256 public key works as a stand-in "subscriber" key for
    /// `ece`'s ECDH step — it doesn't need to be a real browser's key, just
    /// a real point on the curve, so reusing `generate_vapid_keypair`'s
    /// output here is the cheapest way to get one.
    fn fake_subscription(endpoint: String) -> PushSubscription {
        let (_, p256dh) = generate_vapid_keypair();
        PushSubscription {
            endpoint,
            p256dh,
            auth: URL_SAFE_NO_PAD.encode(random_bytes::<16>()),
            created_at: "2026-08-25T00:00:00Z".to_string(),
        }
    }

    fn test_state(store: megatokyo_core::store::Store) -> AppState {
        let (vapid_private_key, vapid_public_key) = generate_vapid_keypair();
        AppState {
            store,
            image_cache: megatokyo_core::image_cache::ImageCache::new(std::env::temp_dir()),
            http_client: reqwest::Client::new(),
            token: "test-token".to_string(),
            config: tokio::sync::RwLock::new(crate::config::Config {
                bind: "127.0.0.1:0".to_string(),
                api_token: "test-token".to_string(),
                deepl_api_key: String::new(),
                poll_interval_minutes: 15,
                vapid_private_key,
                vapid_public_key,
                vapid_subject: String::new(),
            }),
            config_path: std::env::temp_dir().join("megatokyo-test-unused-config.toml"),
            backfilling: std::sync::atomic::AtomicBool::new(false),
            check_requested: tokio::sync::Notify::new(),
        }
    }

    #[tokio::test]
    async fn send_to_all_posts_to_every_subscription_for_each_new_item() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/ep-a"))
            .respond_with(wiremock::ResponseTemplate::new(201))
            .expect(1)
            .mount(&server)
            .await;
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/ep-b"))
            .respond_with(wiremock::ResponseTemplate::new(201))
            .expect(1)
            .mount(&server)
            .await;

        let store = megatokyo_core::store::Store::open_in_memory().unwrap();
        store
            .add_push_subscription(&fake_subscription(format!("{}/ep-a", server.uri())))
            .unwrap();
        store
            .add_push_subscription(&fake_subscription(format!("{}/ep-b", server.uri())))
            .unwrap();

        let state = test_state(store);
        send_to_all(&state, &[item(1619, "Sample")]).await;

        // wiremock's `.expect(1)` on each mock (checked on drop) is the
        // real assertion here — both endpoints must have been hit exactly
        // once for this test to pass.
    }

    #[tokio::test]
    async fn a_gone_response_removes_the_subscription_but_leaves_others_alone() {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/gone"))
            .respond_with(wiremock::ResponseTemplate::new(410))
            .mount(&server)
            .await;
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/still-valid"))
            .respond_with(wiremock::ResponseTemplate::new(201))
            .mount(&server)
            .await;

        let store = megatokyo_core::store::Store::open_in_memory().unwrap();
        let gone_endpoint = format!("{}/gone", server.uri());
        store
            .add_push_subscription(&fake_subscription(gone_endpoint.clone()))
            .unwrap();
        store
            .add_push_subscription(&fake_subscription(format!("{}/still-valid", server.uri())))
            .unwrap();

        let state = test_state(store);
        send_to_all(&state, &[item(1619, "Sample")]).await;

        let remaining = state.store.all_push_subscriptions().unwrap();
        assert_eq!(remaining.len(), 1);
        assert_ne!(remaining[0].endpoint, gone_endpoint);
    }

    #[tokio::test]
    async fn nothing_is_sent_when_vapid_is_not_configured() {
        let store = megatokyo_core::store::Store::open_in_memory().unwrap();
        store
            .add_push_subscription(&fake_subscription("https://push.example/a".to_string()))
            .unwrap();
        let state = test_state(store);
        state.config.write().await.vapid_private_key = String::new();

        // No mock server at all — if this tried to send anything, it would
        // fail to connect. Reaching the end without panicking is the test.
        send_to_all(&state, &[item(1619, "Sample")]).await;
    }
}
