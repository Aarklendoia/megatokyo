//! Web Push subscribe flow: request notification permission, subscribe via
//! the service worker's `PushManager` using the daemon's VAPID public key,
//! then register the resulting subscription with the daemon.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{PushManager, PushSubscriptionOptionsInit, ServiceWorkerRegistration};

use crate::daemon_client;

pub async fn subscribe(base_url: &str, token: &str) -> Result<(), String> {
    let vapid_public_key = daemon_client::fetch_vapid_public_key(base_url, token).await?;

    let permission = request_notification_permission().await?;
    if permission != "granted" {
        return Err("notification permission was not granted".to_string());
    }

    let registration = service_worker_registration().await?;
    let push_manager: PushManager = registration.push_manager().map_err(js_err)?;

    let key_bytes = URL_SAFE_NO_PAD
        .decode(&vapid_public_key)
        .map_err(|err| err.to_string())?;
    let key_array = js_sys::Uint8Array::from(key_bytes.as_slice());

    let options = PushSubscriptionOptionsInit::new();
    options.set_user_visible_only(true);
    options.set_application_server_key_opt_u8_array(Some(&key_array));

    let subscription = JsFuture::from(
        push_manager
            .subscribe_with_options(&options)
            .map_err(js_err)?,
    )
    .await
    .map_err(js_err)?;
    let subscription: web_sys::PushSubscription = subscription.unchecked_into();

    let json = subscription.to_json().map_err(js_err)?;
    let endpoint = json.get_endpoint().ok_or("subscription has no endpoint")?;
    let keys = json.get_keys().ok_or("subscription has no keys")?;
    let p256dh = keys.get_p256dh().ok_or("subscription has no p256dh key")?;
    let auth = keys.get_auth().ok_or("subscription has no auth key")?;

    daemon_client::push_subscribe(base_url, token, &endpoint, &p256dh, &auth).await
}

async fn request_notification_permission() -> Result<String, String> {
    let promise = web_sys::Notification::request_permission().map_err(js_err)?;
    let result = JsFuture::from(promise).await.map_err(js_err)?;
    Ok(result.as_string().unwrap_or_default())
}

/// `navigator.serviceWorker.ready` — waits for the service worker
/// registered in `index.html` to become active, rather than assuming it
/// already is (it may still be installing on a first visit).
async fn service_worker_registration() -> Result<ServiceWorkerRegistration, String> {
    let window = web_sys::window().ok_or("no window")?;
    let container = window.navigator().service_worker();
    let promise = container.ready().map_err(js_err)?;
    let registration = JsFuture::from(promise).await.map_err(js_err)?;
    Ok(registration.unchecked_into())
}

fn js_err(err: wasm_bindgen::JsValue) -> String {
    err.as_string().unwrap_or_else(|| format!("{err:?}"))
}
