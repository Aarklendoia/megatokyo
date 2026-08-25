//! Works out which daemon to talk to (local, auto-managed, or a remote one
//! configured in Settings) and, in the local case, gets it running.
//!
//! Deliberately no HTTP client dependency, matching the rest of this crate
//! (see Cargo.toml's doc comment): the daemon's config file already has
//! everything needed (bind address, token), and "is it up yet" only needs a
//! bare TCP connect, not a real request — QML's own XHR is what actually
//! talks to the daemon once it's running, remote HTTPS included.

use std::net::TcpStream;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use megatokyo_core::local_ctrl;

use crate::flat_toml;

pub struct DaemonLink {
    pub base_url: String,
    pub token: String,
}

/// Same path `megatokyo-daemon`'s own `config::Config::default_path()`
/// resolves to — duplicated rather than shared via a dependency, since
/// pulling in the daemon crate (or a config-parsing dependency) just for
/// this one path/field pair would defeat the point of keeping this
/// launcher dependency-free.
fn daemon_config_path() -> PathBuf {
    let config_home = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config")
        });
    config_home.join("megatokyo-daemon").join("config.toml")
}

/// `--background` mode's poll interval, read from this GUI's own config —
/// defaults to matching the daemon's own default (see
/// `megatokyo-daemon`'s `config::default_poll_interval_minutes`) since
/// there's rarely a reason for the two to disagree.
pub fn poll_interval_minutes() -> u64 {
    std::fs::read_to_string(crate::config::gui_config_path())
        .ok()
        .and_then(|contents| {
            flat_toml::raw_value(&contents, "poll_interval_minutes")?
                .parse()
                .ok()
        })
        .unwrap_or(15)
}

/// Reads this GUI's own config, if a remote daemon override is set there.
fn configured_remote() -> Option<DaemonLink> {
    let contents = std::fs::read_to_string(crate::config::gui_config_path()).ok()?;
    let base_url = flat_toml::quoted_string(&contents, "remote_base_url")?;
    let token = flat_toml::quoted_string(&contents, "remote_api_token")?;
    (!base_url.is_empty() && !token.is_empty()).then_some(DaemonLink { base_url, token })
}

fn local_daemon_from_config() -> Option<DaemonLink> {
    let contents = std::fs::read_to_string(daemon_config_path()).ok()?;
    let bind = flat_toml::quoted_string(&contents, "bind")?;
    let token = flat_toml::quoted_string(&contents, "api_token")?;
    Some(DaemonLink {
        base_url: format!("http://{bind}"),
        token,
    })
}

/// Bare TCP connect, no data exchanged — enough to know something is
/// listening on the daemon's configured local port.
fn is_listening(base_url: &str) -> bool {
    let Some(host_port) = base_url.strip_prefix("http://") else {
        return false;
    };
    TcpStream::connect_timeout(
        &match host_port.parse() {
            Ok(addr) => addr,
            Err(_) => return false,
        },
        Duration::from_millis(300),
    )
    .is_ok()
}

/// Resolves which daemon to use: a configured remote one first, else a
/// local one — spawning `megatokyo-daemon` if it isn't already listening,
/// then giving it a few seconds to come up (it creates its own config with
/// a fresh token on first run, which is what `local_daemon_from_config`
/// then reads back).
pub fn resolve() -> Option<DaemonLink> {
    if let Some(remote) = configured_remote() {
        return Some(remote);
    }

    if let Some(link) = local_daemon_from_config() {
        if is_listening(&link.base_url) {
            return Some(link);
        }
    }

    if let Err(err) = Command::new("megatokyo-daemon").spawn() {
        log::warn!("could not spawn megatokyo-daemon: {err}");
    }

    for _ in 0..50 {
        std::thread::sleep(Duration::from_millis(100));
        if let Some(link) = local_daemon_from_config() {
            if is_listening(&link.base_url) {
                return Some(link);
            }
        }
    }

    // Best-effort: hand back whatever config exists even if the liveness
    // probe never succeeded, so the QML UI can at least show a real
    // connection error instead of the launcher refusing to start at all.
    local_daemon_from_config()
}

/// Current user's UID, for [`local_ctrl::runtime_dir`] — re-exported here so
/// `main.rs` doesn't need its own `use megatokyo_core::local_ctrl` just for
/// this one call.
pub fn current_uid() -> u32 {
    local_ctrl::current_uid()
}
