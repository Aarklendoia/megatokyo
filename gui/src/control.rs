//! Local control server for the windowed GUI's own Settings screen —
//! same pattern as `kio-protondrive`'s `wizard::main` and
//! `linux_hello_config`'s own control server, just hosted by this launcher
//! instead of a setup wizard: a plain `TcpListener` on `127.0.0.1:0`
//! (OS-assigned port), started unconditionally alongside the windowed QML
//! view, one thread per connection, manual HTTP request-line/header
//! parsing. Loopback-only, single-user, trusted-local-process threat model
//! throughout (see `megatokyo_core::local_ctrl`'s own doc comment) — the
//! token defends against another local user guessing the port, not a
//! network attacker.
//!
//! `--background` mode never starts this: nothing but the windowed QML view
//! ever needs to write this GUI's own config.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

use megatokyo_core::local_ctrl::{
    constant_time_eq, extract_header, extract_query_param, json_escape, request_method,
    request_path,
};

use crate::config::{gui_config_path, GuiConfig};

const TOKEN_HEADER: &str = "x-megatokyo-gui-ctrl-token";

/// Binds the control server and returns its assigned port. Panics if the
/// bind itself fails (loopback on port 0 essentially never does) — mirrors
/// `kio-protondrive-wizard`'s own `start_control_server`, which treats this
/// the same way: there is no reasonable fallback for "the Settings screen's
/// backend couldn't start at all".
pub fn start(token: String) -> u16 {
    let listener =
        TcpListener::bind("127.0.0.1:0").expect("unable to start the local control server");
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let token = token.clone();
            thread::spawn(move || handle_connection(stream, &token));
        }
    });
    port
}

fn handle_connection(mut stream: TcpStream, expected_token: &str) {
    let mut buf = [0u8; 4096];
    let Ok(n) = stream.read(&mut buf) else {
        return;
    };
    let req = String::from_utf8_lossy(&buf[..n]).into_owned();
    let path = request_path(&req).to_string();

    let authorized = extract_header(&req, TOKEN_HEADER)
        .map(|t| constant_time_eq(t, expected_token))
        .unwrap_or(false);

    let (status, body) = if !authorized {
        (
            "403 Forbidden",
            "{\"ok\":false,\"error\":\"forbidden\"}".to_string(),
        )
    } else {
        match path.as_str() {
            "/gui-config" => route_gui_config(&req, &gui_config_path()),
            _ => (
                "404 Not Found",
                "{\"ok\":false,\"error\":\"not found\"}".to_string(),
            ),
        }
    };

    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
}

fn config_json(config: &GuiConfig) -> String {
    format!(
        "{{\"remote_base_url\":\"{}\",\"remote_api_token\":\"{}\",\"poll_interval_minutes\":{},\"notifications_enabled\":{}}}",
        json_escape(&config.remote_base_url),
        json_escape(&config.remote_api_token),
        config.poll_interval_minutes,
        config.notifications_enabled,
    )
}

/// `GET` reports the current config, `POST` applies whichever query params
/// are present (each field independently updatable, like the daemon's own
/// `/config`) and persists the result. Takes `path` as a parameter (rather
/// than resolving `gui_config_path()` itself) so tests can point it at a
/// tempdir instead of the real user config.
fn route_gui_config(req: &str, path: &std::path::Path) -> (&'static str, String) {
    match request_method(req) {
        "GET" => ("200 OK", config_json(&GuiConfig::load(path))),
        "POST" => {
            let mut config = GuiConfig::load(path);
            if let Some(v) = extract_query_param(req, "remote_base_url") {
                config.remote_base_url = v;
            }
            if let Some(v) = extract_query_param(req, "remote_api_token") {
                config.remote_api_token = v;
            }
            if let Some(v) = extract_query_param(req, "poll_interval_minutes") {
                match v.parse::<u64>() {
                    Ok(m) if m > 0 => config.poll_interval_minutes = m,
                    _ => {
                        return (
                            "400 Bad Request",
                            "{\"ok\":false,\"error\":\"poll_interval_minutes must be a positive integer\"}"
                                .to_string(),
                        )
                    }
                }
            }
            if let Some(v) = extract_query_param(req, "notifications_enabled") {
                config.notifications_enabled = v == "true";
            }
            match config.save(path) {
                Ok(()) => ("200 OK", config_json(&config)),
                Err(err) => (
                    "500 Internal Server Error",
                    format!(
                        "{{\"ok\":false,\"error\":\"{}\"}}",
                        json_escape(&err.to_string())
                    ),
                ),
            }
        }
        _ => (
            "404 Not Found",
            "{\"ok\":false,\"error\":\"not found\"}".to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_gui_config_get_reports_defaults_when_unconfigured() {
        let dir = tempfile::tempdir().unwrap();
        let (status, body) = route_gui_config(
            "GET /gui-config HTTP/1.1\r\n",
            &dir.path().join("config.toml"),
        );
        assert_eq!(status, "200 OK");
        assert!(body.contains("\"poll_interval_minutes\":15"));
        assert!(body.contains("\"notifications_enabled\":true"));
    }

    #[test]
    fn route_gui_config_post_updates_only_the_given_fields_and_persists_them() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");

        let (status, body) = route_gui_config(
            "POST /gui-config?remote_base_url=https%3A%2F%2Fexample.com HTTP/1.1\r\n",
            &path,
        );
        assert_eq!(status, "200 OK");
        assert!(body.contains("\"remote_base_url\":\"https://example.com\""));

        let (_, body) = route_gui_config(
            "POST /gui-config?notifications_enabled=false HTTP/1.1\r\n",
            &path,
        );
        // The URL set by the previous call is still there — a partial
        // update never clobbers fields it wasn't given.
        assert!(body.contains("\"remote_base_url\":\"https://example.com\""));
        assert!(body.contains("\"notifications_enabled\":false"));

        assert_eq!(
            GuiConfig::load(&path).remote_base_url,
            "https://example.com"
        );
    }

    #[test]
    fn route_gui_config_post_rejects_a_non_positive_poll_interval() {
        let dir = tempfile::tempdir().unwrap();
        let (status, _) = route_gui_config(
            "POST /gui-config?poll_interval_minutes=0 HTTP/1.1\r\n",
            &dir.path().join("config.toml"),
        );
        assert_eq!(status, "400 Bad Request");
    }

    #[test]
    fn a_request_without_the_token_header_is_forbidden_end_to_end() {
        let port = start("secret".to_string());
        let mut stream =
            TcpStream::connect(("127.0.0.1", port)).expect("control server should be listening");
        stream
            .write_all(b"GET /gui-config HTTP/1.1\r\n\r\n")
            .unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        assert!(response.starts_with("HTTP/1.1 403 Forbidden"));
    }

    #[test]
    fn a_request_with_the_right_token_reaches_the_route_end_to_end() {
        let port = start("secret".to_string());
        let mut stream =
            TcpStream::connect(("127.0.0.1", port)).expect("control server should be listening");
        stream
            .write_all(
                format!("GET /gui-config HTTP/1.1\r\n{TOKEN_HEADER}: secret\r\n\r\n").as_bytes(),
            )
            .unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        assert!(response.starts_with("HTTP/1.1 200 OK"));
    }
}
