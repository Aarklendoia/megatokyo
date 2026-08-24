//! This GUI's own config: `$XDG_CONFIG_HOME/megatokyo-gui/config.toml`.
//!
//! Hand-rolled flat-TOML read/write, not the `toml` crate — see
//! `daemon_link`'s own doc comment: four scalar fields don't justify the one
//! external dependency this otherwise dependency-light binary would gain.
//! `daemon_link::configured_remote`/`poll_interval_minutes` keep their own
//! read-only parsing (unchanged, already tested); this module is the
//! write-back half the Settings screen needs — see the control server in
//! `control.rs` that calls [`GuiConfig::save`].

use std::path::PathBuf;

pub fn gui_config_path() -> PathBuf {
    let config_home = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config")
        });
    config_home.join("megatokyo-gui").join("config.toml")
}

#[derive(Debug, Clone, PartialEq)]
pub struct GuiConfig {
    pub remote_base_url: String,
    pub remote_api_token: String,
    pub poll_interval_minutes: u64,
    pub notifications_enabled: bool,
    /// The Reader's "All strips" / "Main story only" toggle — a per-install
    /// UI preference, not daemon state: unlike favorites/reading progress,
    /// which are meant to follow the user across every client of a shared
    /// remote daemon, this stays local to whichever GUI installation set
    /// it.
    pub main_story_only: bool,
}

impl Default for GuiConfig {
    fn default() -> Self {
        Self {
            remote_base_url: String::new(),
            remote_api_token: String::new(),
            // Matches megatokyo-daemon's own default poll interval — see
            // daemon_link::poll_interval_minutes's doc comment.
            poll_interval_minutes: 15,
            notifications_enabled: true,
            main_story_only: false,
        }
    }
}

impl GuiConfig {
    pub fn load(path: &std::path::Path) -> Self {
        let Ok(contents) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        let defaults = Self::default();
        Self {
            remote_base_url: read_string(&contents, "remote_base_url")
                .unwrap_or(defaults.remote_base_url),
            remote_api_token: read_string(&contents, "remote_api_token")
                .unwrap_or(defaults.remote_api_token),
            poll_interval_minutes: read_string(&contents, "poll_interval_minutes")
                .and_then(|v| v.parse().ok())
                .unwrap_or(defaults.poll_interval_minutes),
            notifications_enabled: read_string(&contents, "notifications_enabled")
                .map(|v| v == "true")
                .unwrap_or(defaults.notifications_enabled),
            main_story_only: read_string(&contents, "main_story_only")
                .map(|v| v == "true")
                .unwrap_or(defaults.main_story_only),
        }
    }

    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let contents = format!(
            "remote_base_url = \"{}\"\nremote_api_token = \"{}\"\npoll_interval_minutes = {}\nnotifications_enabled = {}\nmain_story_only = {}\n",
            escape(&self.remote_base_url),
            escape(&self.remote_api_token),
            self.poll_interval_minutes,
            self.notifications_enabled,
            self.main_story_only,
        );
        std::fs::write(path, contents)
    }
}

/// Same restricted, hand-rolled scalar parsing as `daemon_link`'s
/// `read_toml_string_field` (no nesting, no multi-line values) — reads
/// either a quoted string or a bare token (covers this module's own
/// non-string fields too: `15`, `true`), whichever the line actually has.
fn read_string(contents: &str, key: &str) -> Option<String> {
    contents.lines().find_map(|line| {
        let rest = line.trim().strip_prefix(key)?.trim_start();
        let rest = rest.strip_prefix('=')?.trim();
        Some(match rest.strip_prefix('"') {
            Some(quoted) => unescape(quoted.strip_suffix('"')?),
            None => rest.to_string(),
        })
    })
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Inverse of [`escape`] — a char-by-char scan (not sequential `.replace()`
/// calls, which would misinterpret a `\\` that escape produced as a literal
/// backslash into a second escape pass over its own output).
fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some(next) => out.push(next),
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_returns_defaults_when_the_file_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        let config = GuiConfig::load(&dir.path().join("does-not-exist.toml"));
        assert_eq!(config, GuiConfig::default());
    }

    #[test]
    fn a_saved_config_round_trips_through_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let config = GuiConfig {
            remote_base_url: "https://megatokyo.example.com".to_string(),
            remote_api_token: "abc123".to_string(),
            poll_interval_minutes: 30,
            notifications_enabled: false,
            main_story_only: true,
        };
        config.save(&path).unwrap();
        assert_eq!(GuiConfig::load(&path), config);
    }

    #[test]
    fn save_escapes_quotes_and_backslashes_in_string_fields() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let config = GuiConfig {
            remote_api_token: "has \"quotes\" and \\backslash".to_string(),
            ..GuiConfig::default()
        };
        config.save(&path).unwrap();
        assert_eq!(
            GuiConfig::load(&path).remote_api_token,
            config.remote_api_token
        );
    }

    #[test]
    fn load_fills_in_defaults_for_fields_missing_from_an_older_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "remote_base_url = \"https://x\"\n").unwrap();
        let config = GuiConfig::load(&path);
        assert_eq!(config.remote_base_url, "https://x");
        assert_eq!(config.poll_interval_minutes, 15);
        assert!(config.notifications_enabled);
        assert!(!config.main_story_only);
    }
}
