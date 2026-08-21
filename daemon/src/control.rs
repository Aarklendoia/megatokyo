//! Hand-rolled HTTP API — same request-parsing style as
//! `kio-protondrive`'s local control server (see
//! `megatokyo_core::local_ctrl`), but async (tokio) and bindable to a real
//! network address, not just loopback: see the plan's "Déploiement
//! distant". `/health` is the only unauthenticated route (HAProxy health
//! checks); every other route requires the `x-megatokyo-daemon-token`
//! header to match [`AppState::token`].

use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use megatokyo_core::image_cache::{content_type_for, ImageCache};
use megatokyo_core::local_ctrl::{
    constant_time_eq, extract_header, extract_query_param, request_method, request_path,
};
use megatokyo_core::store::Store;
use megatokyo_core::translate::{get_translated_rant, Translator};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

const TOKEN_HEADER: &str = "x-megatokyo-daemon-token";

pub struct AppState {
    pub store: Store,
    pub image_cache: ImageCache,
    pub translator: Option<Translator>,
    pub token: String,
    /// Set while the initial archive backfill (or a `/check`-triggered
    /// cycle) is running — surfaced on `/status` so a client can tell "no
    /// content yet" apart from "still loading".
    pub backfilling: AtomicBool,
    /// Woken by `POST /check` — the poll loop (see the poll-loop issue)
    /// waits on this to run an out-of-schedule cycle, same
    /// `Arc<tokio::sync::Notify>` pattern linux-hello's `hello_daemon` uses
    /// for its own retry signal.
    pub check_requested: tokio::sync::Notify,
}

pub async fn serve(bind: SocketAddr, state: Arc<AppState>) -> std::io::Result<()> {
    let listener = TcpListener::bind(bind).await?;
    log::info!("listening on {bind}");
    loop {
        let (stream, peer) = listener.accept().await?;
        let state = state.clone();
        tokio::spawn(async move {
            if let Err(err) = handle_connection(stream, &state).await {
                log::debug!("connection from {peer} failed: {err}");
            }
        });
    }
}

async fn handle_connection(mut stream: TcpStream, state: &AppState) -> std::io::Result<()> {
    let mut buf = vec![0u8; 8192];
    let n = stream.read(&mut buf).await?;
    let req = String::from_utf8_lossy(&buf[..n]).into_owned();
    let path = request_path(&req).to_string();

    let response = if path == "/health" {
        Response::text("OK")
    } else {
        let token_ok = extract_header(&req, TOKEN_HEADER)
            .map(|t| constant_time_eq(t, &state.token))
            .unwrap_or(false);
        if !token_ok {
            Response {
                status: "403 Forbidden",
                content_type: "application/json",
                body: b"{\"ok\":false,\"error\":\"forbidden\"}".to_vec(),
            }
        } else {
            route(&req, &path, state).await
        }
    };

    stream.write_all(&response.into_bytes()).await?;
    Ok(())
}

struct Response {
    status: &'static str,
    content_type: &'static str,
    body: Vec<u8>,
}

impl Response {
    fn text(body: &str) -> Self {
        Self {
            status: "200 OK",
            content_type: "text/plain",
            body: body.as_bytes().to_vec(),
        }
    }

    fn json(status: &'static str, body: String) -> Self {
        Self {
            status,
            content_type: "application/json",
            body: body.into_bytes(),
        }
    }

    fn ok_json<T: serde::Serialize>(value: &T) -> Self {
        Self::json("200 OK", serde_json::to_string(value).unwrap())
    }

    fn not_found() -> Self {
        Self::json(
            "404 Not Found",
            "{\"ok\":false,\"error\":\"not found\"}".to_string(),
        )
    }

    fn bad_request(message: &str) -> Self {
        Self::json(
            "400 Bad Request",
            serde_json::json!({"ok": false, "error": message}).to_string(),
        )
    }

    fn server_error(message: &str) -> Self {
        Self::json(
            "500 Internal Server Error",
            serde_json::json!({"ok": false, "error": message}).to_string(),
        )
    }

    fn into_bytes(self) -> Vec<u8> {
        let mut out = format!(
            "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            self.status,
            self.content_type,
            self.body.len()
        )
        .into_bytes();
        out.extend(self.body);
        out
    }
}

async fn route(req: &str, path: &str, state: &AppState) -> Response {
    match path {
        "/chapters" => match state.store.all_chapters() {
            Ok(chapters) => Response::ok_json(&chapters),
            Err(err) => Response::server_error(&err.to_string()),
        },
        "/strips" => {
            let result = match extract_query_param(req, "category") {
                Some(category) => state.store.strips_by_category(&category),
                None => state.store.all_strips(),
            };
            match result {
                Ok(strips) => Response::ok_json(&strips),
                Err(err) => Response::server_error(&err.to_string()),
            }
        }
        "/strip" => route_strip(req, state),
        "/rants" => match state.store.all_rants() {
            Ok(rants) => Response::ok_json(&rants),
            Err(err) => Response::server_error(&err.to_string()),
        },
        "/rant" => route_rant(req, state).await,
        "/image" => route_image(req, state).await,
        "/status" => route_status(state),
        "/favorites" => route_favorites(req, state),
        "/progress" => route_progress(req, state),
        "/check" => {
            state.check_requested.notify_one();
            Response::ok_json(&serde_json::json!({"ok": true}))
        }
        _ => Response::not_found(),
    }
}

fn parse_number(req: &str) -> Result<i32, Response> {
    extract_query_param(req, "number")
        .ok_or_else(|| Response::bad_request("missing number"))?
        .parse()
        .map_err(|_| Response::bad_request("number must be an integer"))
}

fn route_strip(req: &str, state: &AppState) -> Response {
    let number = match parse_number(req) {
        Ok(n) => n,
        Err(response) => return response,
    };
    match state.store.strip_by_number(number) {
        Ok(Some(strip)) => Response::ok_json(&strip),
        Ok(None) => Response::not_found(),
        Err(err) => Response::server_error(&err.to_string()),
    }
}

async fn route_rant(req: &str, state: &AppState) -> Response {
    let number = match parse_number(req) {
        Ok(n) => n,
        Err(response) => return response,
    };
    let lang = extract_query_param(req, "lang").unwrap_or_default();

    let rant = match state.store.rant_by_number(number) {
        Ok(Some(rant)) => rant,
        Ok(None) => return Response::not_found(),
        Err(err) => return Response::server_error(&err.to_string()),
    };

    let content = if lang.is_empty() || lang.eq_ignore_ascii_case("en") {
        rant.content.clone()
    } else {
        let translated = match &state.translator {
            Some(translator) => get_translated_rant(&state.store, translator, number, &lang)
                .await
                .map_err(|e| e.to_string()),
            None => Err("no DeepL API key configured".to_string()),
        };
        match translated {
            // `rant` above already confirmed this number exists, so
            // get_translated_rant returning None here would mean it
            // vanished between the two lookups — fall back to the
            // original rather than 404 a request that clearly has one.
            Ok(Some(content)) => content,
            Ok(None) => rant.content.clone(),
            Err(err) => return Response::server_error(&err),
        }
    };

    Response::ok_json(&serde_json::json!({
        "number": rant.number,
        "author": rant.author,
        "title": rant.title,
        "url": rant.url,
        "publish_date": rant.publish_date,
        "lang": if lang.is_empty() { "en" } else { &lang },
        "content": content,
    }))
}

async fn route_image(req: &str, state: &AppState) -> Response {
    let number = match parse_number(req) {
        Ok(n) => n,
        Err(response) => return response,
    };
    let strip = match state.store.strip_by_number(number) {
        Ok(Some(strip)) => strip,
        Ok(None) => return Response::not_found(),
        Err(err) => return Response::server_error(&err.to_string()),
    };
    match state.image_cache.ensure_cached(number, &strip.url).await {
        Ok(path) => match std::fs::read(&path) {
            Ok(bytes) => Response {
                status: "200 OK",
                content_type: content_type_for(&path),
                body: bytes,
            },
            Err(err) => Response::server_error(&err.to_string()),
        },
        Err(err) => Response::server_error(&err.to_string()),
    }
}

fn route_status(state: &AppState) -> Response {
    match state.store.get_checking() {
        Ok(checking) => Response::ok_json(&serde_json::json!({
            "last_check": checking.last_check,
            "last_strip_number": checking.last_strip_number,
            "last_rant_number": checking.last_rant_number,
            "backfilling": state.backfilling.load(Ordering::Relaxed),
        })),
        Err(err) => Response::server_error(&err.to_string()),
    }
}

/// `GET` lists favorites (most recently starred first), `POST` stars a
/// strip, `DELETE` unstars one — no user concept, see
/// `megatokyo_core::domain::Favorite`'s doc comment: one shared list per
/// daemon instance, gated by the same token as everything else.
fn route_favorites(req: &str, state: &AppState) -> Response {
    match request_method(req) {
        "GET" => match state.store.all_favorites() {
            Ok(favorites) => Response::ok_json(&favorites),
            Err(err) => Response::server_error(&err.to_string()),
        },
        "POST" => {
            let number = match parse_number(req) {
                Ok(n) => n,
                Err(response) => return response,
            };
            let added_at = chrono::Utc::now().to_rfc3339();
            match state.store.add_favorite(number, &added_at) {
                Ok(()) => Response::ok_json(&serde_json::json!({"ok": true})),
                Err(err) => Response::server_error(&err.to_string()),
            }
        }
        "DELETE" => {
            let number = match parse_number(req) {
                Ok(n) => n,
                Err(response) => return response,
            };
            match state.store.remove_favorite(number) {
                Ok(()) => Response::ok_json(&serde_json::json!({"ok": true})),
                Err(err) => Response::server_error(&err.to_string()),
            }
        }
        _ => Response::not_found(),
    }
}

/// `GET` reports the last strip number the user was reading (`null` if
/// never set), `POST` updates it — same single-daemon-wide state as
/// favorites, meant to let a client "resume where they left off" on any
/// device pointed at the same daemon.
fn route_progress(req: &str, state: &AppState) -> Response {
    match request_method(req) {
        "GET" => match state.store.get_reading_progress() {
            Ok(strip_number) => Response::ok_json(&serde_json::json!({
                "strip_number": strip_number,
            })),
            Err(err) => Response::server_error(&err.to_string()),
        },
        "POST" => {
            let number = match parse_number(req) {
                Ok(n) => n,
                Err(response) => return response,
            };
            match state.store.save_reading_progress(number) {
                Ok(()) => Response::ok_json(&serde_json::json!({"ok": true})),
                Err(err) => Response::server_error(&err.to_string()),
            }
        }
        _ => Response::not_found(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use megatokyo_core::domain::{Chapter, Rant};

    fn state() -> AppState {
        AppState {
            store: Store::open_in_memory().unwrap(),
            image_cache: ImageCache::new(std::env::temp_dir()),
            translator: None,
            token: "test-token".to_string(),
            backfilling: AtomicBool::new(false),
            check_requested: tokio::sync::Notify::new(),
        }
    }

    #[tokio::test]
    async fn health_requires_no_token() {
        let state = state();
        let response = route_or_health("GET /health HTTP/1.1\r\n", &state).await;
        assert_eq!(response.status, "200 OK");
    }

    async fn route_or_health(req: &str, state: &AppState) -> Response {
        let path = request_path(req).to_string();
        if path == "/health" {
            Response::text("OK")
        } else {
            route(req, &path, state).await
        }
    }

    #[tokio::test]
    async fn chapters_route_returns_stored_chapters_as_json() {
        let state = state();
        state
            .store
            .upsert_chapter(&Chapter {
                number: 13,
                category: "C-13".to_string(),
                title: "Redemption".to_string(),
            })
            .unwrap();

        let response = route("GET /chapters HTTP/1.1\r\n", "/chapters", &state).await;
        assert_eq!(response.status, "200 OK");
        let body: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
        assert_eq!(body[0]["category"], "C-13");
    }

    #[tokio::test]
    async fn strip_route_404s_for_an_unknown_number() {
        let state = state();
        let response = route("GET /strip?number=9999 HTTP/1.1\r\n", "/strip", &state).await;
        assert_eq!(response.status, "404 Not Found");
    }

    #[tokio::test]
    async fn strip_route_requires_a_number_param() {
        let state = state();
        let response = route("GET /strip HTTP/1.1\r\n", "/strip", &state).await;
        assert_eq!(response.status, "400 Bad Request");
    }

    #[tokio::test]
    async fn rant_route_returns_original_content_without_a_lang_param() {
        let state = state();
        state
            .store
            .upsert_rant(&Rant {
                number: 1106,
                author: "Piro".to_string(),
                title: "Clearing of the Air".to_string(),
                url: "https://megatokyo.com/rantimgs/1106.png".to_string(),
                publish_date: "2023-09-27T00:00:00Z".to_string(),
                content: "<p>hello</p>".to_string(),
            })
            .unwrap();

        let response = route("GET /rant?number=1106 HTTP/1.1\r\n", "/rant", &state).await;
        assert_eq!(response.status, "200 OK");
        let body: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
        assert_eq!(body["content"], "<p>hello</p>");
        assert_eq!(body["lang"], "en");
    }

    #[tokio::test]
    async fn check_route_wakes_up_a_waiting_poll_loop() {
        let state = state();
        let response = route("POST /check HTTP/1.1\r\n", "/check", &state).await;
        assert_eq!(response.status, "200 OK");
        // Should not hang: notify_one() already queued a permit before
        // notified() is even called, since Notify remembers one
        // "surplus" wake-up.
        state.check_requested.notified().await;
    }

    #[tokio::test]
    async fn status_route_reports_the_checking_checkpoint() {
        let state = state();
        let response = route("GET /status HTTP/1.1\r\n", "/status", &state).await;
        assert_eq!(response.status, "200 OK");
        let body: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
        assert_eq!(body["last_strip_number"], 0);
        assert_eq!(body["backfilling"], false);
    }

    #[tokio::test]
    async fn favorites_can_be_added_listed_and_removed() {
        let state = state();
        let response = route(
            "POST /favorites?number=1619 HTTP/1.1\r\n",
            "/favorites",
            &state,
        )
        .await;
        assert_eq!(response.status, "200 OK");

        let response = route("GET /favorites HTTP/1.1\r\n", "/favorites", &state).await;
        let body: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
        assert_eq!(body[0]["strip_number"], 1619);

        let response = route(
            "DELETE /favorites?number=1619 HTTP/1.1\r\n",
            "/favorites",
            &state,
        )
        .await;
        assert_eq!(response.status, "200 OK");
        let response = route("GET /favorites HTTP/1.1\r\n", "/favorites", &state).await;
        let body: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
        assert_eq!(body.as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn reading_progress_is_null_until_set() {
        let state = state();
        let response = route("GET /progress HTTP/1.1\r\n", "/progress", &state).await;
        let body: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
        assert_eq!(body["strip_number"], serde_json::Value::Null);

        let response = route(
            "POST /progress?number=1619 HTTP/1.1\r\n",
            "/progress",
            &state,
        )
        .await;
        assert_eq!(response.status, "200 OK");

        let response = route("GET /progress HTTP/1.1\r\n", "/progress", &state).await;
        let body: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
        assert_eq!(body["strip_number"], 1619);
    }

    #[tokio::test]
    async fn unknown_route_404s() {
        let state = state();
        let response = route("GET /nope HTTP/1.1\r\n", "/nope", &state).await;
        assert_eq!(response.status, "404 Not Found");
    }
}
