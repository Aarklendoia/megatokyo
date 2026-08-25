//! Daemon configuration: `$XDG_CONFIG_HOME/megatokyo-daemon/config.toml`.
//!
//! Unlike `core::local_ctrl`'s ephemeral per-run token (fine for a purely
//! local control server), this daemon can be reached by remote clients that
//! need to configure a token once and keep using it — so the API token is
//! generated on first boot and persisted here, not regenerated on restart.

use std::path::PathBuf;

use megatokyo_core::local_ctrl::{generate_ctrl_token, write_owner_only_file};
use serde::{Deserialize, Serialize};

use crate::push;

fn default_bind() -> String {
    "127.0.0.1:8420".to_string()
}

fn default_poll_interval_minutes() -> u64 {
    15
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_bind")]
    pub bind: String,
    pub api_token: String,
    #[serde(default)]
    pub deepl_api_key: String,
    #[serde(default = "default_poll_interval_minutes")]
    pub poll_interval_minutes: u64,
    /// Base64 URL-safe (no padding) raw private key bytes — the format
    /// `web_push::VapidSignatureBuilder::from_base64_no_sub` expects
    /// directly, no PEM/DER round-trip needed.
    #[serde(default)]
    pub vapid_private_key: String,
    /// Uncompressed public key bytes, same base64 encoding as
    /// `vapid_private_key` — derived from it once at generation time and
    /// persisted alongside it rather than re-derived on every read, and
    /// exactly what `GET /push/vapid-public-key` hands the PWA for
    /// `PushManager.subscribe({applicationServerKey})`.
    #[serde(default)]
    pub vapid_public_key: String,
    /// VAPID JWT `sub` claim (a `mailto:`/URL contact, RFC 8292) — unlike
    /// the keypair, this can't be self-generated, so it's just another
    /// operator-editable field like `deepl_api_key`: empty by default, set
    /// later via `POST /config`. No install-time wizard needed for either.
    #[serde(default)]
    pub vapid_subject: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("could not read {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("could not write {path}: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("could not parse {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
    #[error("could not serialize config: {0}")]
    Serialize(#[from] toml::ser::Error),
}

impl Config {
    pub fn default_path() -> PathBuf {
        let config_home = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config")
            });
        config_home.join("megatokyo-daemon").join("config.toml")
    }

    /// Loads the config at `path`, creating it with fresh defaults (and a
    /// freshly generated `api_token`) if it doesn't exist yet. A config
    /// written before VAPID support existed parses with empty
    /// `vapid_private_key`/`vapid_public_key` (`#[serde(default)]`) — those
    /// get backfilled and persisted here too, so an upgraded daemon gets
    /// working push on its very next start rather than needing a config
    /// wipe.
    pub fn load_or_init(path: &std::path::Path) -> Result<Self, ConfigError> {
        match std::fs::read_to_string(path) {
            Ok(contents) => {
                let mut config: Config =
                    toml::from_str(&contents).map_err(|source| ConfigError::Parse {
                        path: path.to_path_buf(),
                        source,
                    })?;
                if config.vapid_private_key.is_empty() {
                    let (private_key, public_key) = push::generate_vapid_keypair();
                    config.vapid_private_key = private_key;
                    config.vapid_public_key = public_key;
                    config.save(path)?;
                }
                Ok(config)
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                let (vapid_private_key, vapid_public_key) = push::generate_vapid_keypair();
                let config = Config {
                    bind: default_bind(),
                    api_token: generate_ctrl_token(),
                    deepl_api_key: String::new(),
                    poll_interval_minutes: default_poll_interval_minutes(),
                    vapid_private_key,
                    vapid_public_key,
                    vapid_subject: String::new(),
                };
                config.save(path)?;
                Ok(config)
            }
            Err(source) => Err(ConfigError::Read {
                path: path.to_path_buf(),
                source,
            }),
        }
    }

    pub fn save(&self, path: &std::path::Path) -> Result<(), ConfigError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| ConfigError::Write {
                path: path.to_path_buf(),
                source,
            })?;
        }
        let toml = toml::to_string_pretty(self)?;
        write_owner_only_file(path, &toml).map_err(|source| ConfigError::Write {
            path: path.to_path_buf(),
            source,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_or_init_creates_a_config_with_a_fresh_token_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        assert!(!path.exists());

        let config = Config::load_or_init(&path).unwrap();
        assert!(path.exists());
        assert_eq!(config.api_token.len(), 64);
        assert_eq!(config.bind, "127.0.0.1:8420");
        assert_eq!(config.poll_interval_minutes, 15);
        assert_eq!(config.deepl_api_key, "");
        assert!(!config.vapid_private_key.is_empty());
        assert!(!config.vapid_public_key.is_empty());
        assert_eq!(config.vapid_subject, "");
    }

    /// A config written before VAPID support existed (just `api_token`, same
    /// fixture `load_or_init_fills_in_defaults_for_fields_missing_from_an_older_file`
    /// uses) must come out with a working keypair, backfilled and persisted
    /// on this very load — not just filled with `#[serde(default)]`'s empty
    /// strings, which would silently leave push disabled forever.
    #[test]
    fn load_or_init_backfills_a_vapid_keypair_for_a_pre_vapid_config_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "api_token = \"abc123\"\n").unwrap();

        let config = Config::load_or_init(&path).unwrap();
        assert!(!config.vapid_private_key.is_empty());
        assert!(!config.vapid_public_key.is_empty());

        // Persisted, not just filled in-memory: a second load sees the same
        // keypair rather than generating yet another one.
        let reloaded = Config::load_or_init(&path).unwrap();
        assert_eq!(reloaded.vapid_private_key, config.vapid_private_key);
        assert_eq!(reloaded.vapid_public_key, config.vapid_public_key);
    }

    #[test]
    fn load_or_init_reuses_the_same_token_on_a_second_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");

        let first = Config::load_or_init(&path).unwrap();
        let second = Config::load_or_init(&path).unwrap();
        assert_eq!(first.api_token, second.api_token);
    }

    #[test]
    fn load_or_init_fills_in_defaults_for_fields_missing_from_an_older_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "api_token = \"abc123\"\n").unwrap();

        let config = Config::load_or_init(&path).unwrap();
        assert_eq!(config.api_token, "abc123");
        assert_eq!(config.bind, "127.0.0.1:8420");
        assert_eq!(config.poll_interval_minutes, 15);
    }
}
