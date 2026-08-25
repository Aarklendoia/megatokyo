//! `megatokyo-gui --background`: no window, just a periodic
//! `/status` poll that fires a desktop notification when the daemon's last
//! known strip/rant number has moved forward — see `notification`'s doc
//! comment for why this lives here rather than in the daemon.
//!
//! Needs a real HTTP client (unlike the rest of this otherwise
//! dependency-free launcher, see `daemon_link`'s doc comment): the daemon
//! it polls may be a remote one behind HTTPS (the plan's "Déploiement
//! distant"), which a hand-rolled `TcpStream` can't speak — `reqwest`'s
//! blocking client keeps this module's own control flow synchronous rather
//! than pulling a tokio runtime into an otherwise sync launcher binary.

use std::path::PathBuf;
use std::time::Duration;

use crate::config::{gui_config_path, GuiConfig};
use crate::daemon_link::{self, DaemonLink};
use crate::flat_toml;
use crate::notification::Notifier;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct LastSeen {
    last_strip_number: Option<i32>,
    last_rant_number: Option<i32>,
}

fn state_path() -> PathBuf {
    let state_home = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/state")
        });
    state_home.join("megatokyo-gui").join("last_seen.toml")
}

impl LastSeen {
    fn load(path: &std::path::Path) -> Self {
        let Ok(contents) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        Self {
            last_strip_number: flat_toml::raw_value(&contents, "last_strip_number")
                .and_then(|v| v.parse().ok()),
            last_rant_number: flat_toml::raw_value(&contents, "last_rant_number")
                .and_then(|v| v.parse().ok()),
        }
    }

    fn save(&self, path: &std::path::Path) {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let contents = format!(
            "last_strip_number = {}\nlast_rant_number = {}\n",
            self.last_strip_number.unwrap_or(0),
            self.last_rant_number.unwrap_or(0),
        );
        if let Err(err) = std::fs::write(path, contents) {
            log::warn!("could not persist {}: {err}", path.display());
        }
    }
}

/// A number only counts as "new" once we have a real baseline to compare
/// against — on the very first run `previous` is `None` (no state file
/// yet), and the daemon's current last-known number is just history, not
/// something that just happened.
fn is_new(previous: Option<i32>, current: i32) -> bool {
    previous.is_some_and(|prev| current > prev)
}

struct StatusSnapshot {
    last_strip_number: i32,
    last_rant_number: i32,
}

fn fetch_status(
    client: &reqwest::blocking::Client,
    link: &DaemonLink,
) -> Result<StatusSnapshot, String> {
    let response = client
        .get(format!("{}/status", link.base_url))
        .header("x-megatokyo-daemon-token", &link.token)
        .send()
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    let body = response.text().map_err(|e| e.to_string())?;
    let value: serde_json::Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    Ok(StatusSnapshot {
        last_strip_number: value["last_strip_number"].as_i64().unwrap_or(0) as i32,
        last_rant_number: value["last_rant_number"].as_i64().unwrap_or(0) as i32,
    })
}

/// Takes the already-resolved [`DaemonLink`] and `notifications_enabled` as
/// parameters rather than resolving them itself, so tests can supply both
/// directly instead of going through `$XDG_CONFIG_HOME` (a process-global
/// that parallel `cargo test` runs can't safely mutate per test) — see
/// [`run`] for where each is actually read, fresh, every tick.
fn tick(
    client: &reqwest::blocking::Client,
    link: &DaemonLink,
    notifier: &dyn Notifier,
    state_path: &std::path::Path,
    notifications_enabled: bool,
) {
    let status = match fetch_status(client, link) {
        Ok(status) => status,
        Err(err) => {
            log::warn!("could not fetch /status: {err}");
            return;
        }
    };

    let previous = LastSeen::load(state_path);
    // State is still tracked either way, notifications_enabled or not —
    // flipping the toggle back on shouldn't cause a backlog of "new"
    // numbers that actually arrived while it was off.
    if notifications_enabled && is_new(previous.last_strip_number, status.last_strip_number) {
        notifier.new_strip(status.last_strip_number);
    }
    if notifications_enabled && is_new(previous.last_rant_number, status.last_rant_number) {
        notifier.new_rant(status.last_rant_number);
    }

    LastSeen {
        last_strip_number: Some(status.last_strip_number),
        last_rant_number: Some(status.last_rant_number),
    }
    .save(state_path);
}

pub fn run(notifier: &dyn Notifier, interval: Duration) {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("building the HTTP client should never fail with no custom TLS config");
    let path = state_path();
    loop {
        // Read fresh every tick (a plain file read, cheap compared to the
        // /status request in tick()) so toggling the Settings screen's
        // notifications switch takes effect on the very next tick, not
        // just after a restart.
        let notifications_enabled = GuiConfig::load(&gui_config_path()).notifications_enabled;
        match daemon_link::resolve() {
            Some(link) => tick(&client, &link, notifier, &path, notifications_enabled),
            None => log::warn!("no daemon available to poll"),
        }
        std::thread::sleep(interval);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notification::tests_support::RecordingNotifier;

    #[test]
    fn nothing_is_new_on_the_very_first_observation() {
        assert!(!is_new(None, 1619));
    }

    #[test]
    fn a_higher_number_than_the_baseline_is_new() {
        assert!(is_new(Some(1618), 1619));
    }

    #[test]
    fn an_unchanged_or_lower_number_is_not_new() {
        assert!(!is_new(Some(1619), 1619));
        assert!(!is_new(Some(1619), 1618));
    }

    #[test]
    fn last_seen_round_trips_through_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("last_seen.toml");
        let state = LastSeen {
            last_strip_number: Some(1619),
            last_rant_number: Some(1107),
        };
        state.save(&path);
        assert_eq!(LastSeen::load(&path), state);
    }

    #[test]
    fn last_seen_load_defaults_to_none_when_the_file_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("does-not-exist.toml");
        assert_eq!(LastSeen::load(&path), LastSeen::default());
    }

    #[test]
    fn tick_notifies_only_on_the_numbers_that_actually_advanced() {
        use wiremock::matchers::{header, method, path as path_matcher};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        // reqwest::blocking (what `tick` uses) spins up and tears down its
        // own tokio runtime internally, which panics if called from inside
        // an already-running one — so this test starts wiremock's async
        // server on a runtime of its own, kept alive (not `.await`ed on)
        // for the rest of the test, and calls `tick` from plain sync
        // context rather than `#[tokio::test]`.
        let rt = tokio::runtime::Runtime::new().unwrap();
        let server = rt.block_on(async {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .and(path_matcher("/status"))
                .and(header("x-megatokyo-daemon-token", "test-token"))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "last_check": null, "last_strip_number": 1619, "last_rant_number": 1107, "backfilling": false
                })))
                .mount(&server)
                .await;
            server
        });
        let link = DaemonLink {
            base_url: server.uri(),
            token: "test-token".to_string(),
        };

        let dir = tempfile::tempdir().unwrap();
        let state_path = dir.path().join("last_seen.toml");
        // Baseline: strip is one behind (should notify), rant is already
        // caught up (should not).
        LastSeen {
            last_strip_number: Some(1618),
            last_rant_number: Some(1107),
        }
        .save(&state_path);

        let notifier = RecordingNotifier::default();
        let client = reqwest::blocking::Client::new();
        tick(&client, &link, &notifier, &state_path, true);

        assert_eq!(*notifier.strips.borrow(), vec![1619]);
        assert_eq!(*notifier.rants.borrow(), Vec::<i32>::new());
        assert_eq!(
            LastSeen::load(&state_path),
            LastSeen {
                last_strip_number: Some(1619),
                last_rant_number: Some(1107),
            }
        );
    }

    #[test]
    fn tick_tracks_state_but_stays_silent_when_notifications_are_disabled() {
        use wiremock::matchers::{method, path as path_matcher};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let rt = tokio::runtime::Runtime::new().unwrap();
        let server = rt.block_on(async {
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .and(path_matcher("/status"))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "last_check": null, "last_strip_number": 1619, "last_rant_number": 1107, "backfilling": false
                })))
                .mount(&server)
                .await;
            server
        });
        let link = DaemonLink {
            base_url: server.uri(),
            token: "test-token".to_string(),
        };

        let dir = tempfile::tempdir().unwrap();
        let state_path = dir.path().join("last_seen.toml");
        LastSeen {
            last_strip_number: Some(1618),
            last_rant_number: Some(1106),
        }
        .save(&state_path);

        let notifier = RecordingNotifier::default();
        let client = reqwest::blocking::Client::new();
        tick(&client, &link, &notifier, &state_path, false);

        assert_eq!(*notifier.strips.borrow(), Vec::<i32>::new());
        assert_eq!(*notifier.rants.borrow(), Vec::<i32>::new());
        // State still advances even while silenced, so re-enabling later
        // doesn't dump a backlog of now-stale "new" numbers.
        assert_eq!(
            LastSeen::load(&state_path),
            LastSeen {
                last_strip_number: Some(1619),
                last_rant_number: Some(1107),
            }
        );
    }
}
