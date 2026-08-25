//! Shared building block for this crate's hand-rolled flat-TOML reads —
//! `config.rs`, `daemon_link.rs`, and `background.rs` each read a different
//! flat `key = value` file (this GUI's own config, the daemon's config, and
//! the background watcher's small state file) but all need the same "find
//! the line for this key, strip the `key =` prefix" step. Not a real TOML
//! parser (no nesting, no escaping beyond what each caller handles itself)
//! — these files are all flat scalar tables, so this is all any of them
//! ever needs. Pulling in the `toml` crate here would be the one external
//! dependency this otherwise dependency-light binary has none of (see
//! Cargo.toml's doc comment).

/// Returns the trimmed right-hand side text of `key`'s line, if `contents`
/// has one — quotes (if any) are left in place for the caller to strip,
/// since some fields are quoted strings and others (numbers, booleans)
/// aren't.
pub fn raw_value<'a>(contents: &'a str, key: &str) -> Option<&'a str> {
    contents.lines().find_map(|line| {
        let rest = line.trim().strip_prefix(key)?.trim_start();
        rest.strip_prefix('=').map(str::trim)
    })
}

/// [`raw_value`], with surrounding `"..."` stripped — for fields that are
/// always a quoted string with no escaping beyond a literal `"` (this
/// crate's own config's `escape`d fields aside, which unquote themselves).
pub fn quoted_string(contents: &str, key: &str) -> Option<String> {
    let rest = raw_value(contents, key)?;
    rest.strip_prefix('"')?
        .strip_suffix('"')
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_value_reads_the_trimmed_right_hand_side() {
        let toml = "bind = \"127.0.0.1:8420\"\npoll_interval_minutes = 15\n";
        assert_eq!(raw_value(toml, "bind"), Some("\"127.0.0.1:8420\""));
        assert_eq!(raw_value(toml, "poll_interval_minutes"), Some("15"));
        assert_eq!(raw_value(toml, "missing"), None);
    }

    #[test]
    fn raw_value_does_not_confuse_a_key_that_is_a_prefix_of_another() {
        // "bind" must not match a "bind_extra" line.
        let toml = "bind_extra = \"nope\"\n";
        assert_eq!(raw_value(toml, "bind"), None);
    }

    #[test]
    fn quoted_string_strips_the_surrounding_quotes() {
        let toml = "api_token = \"abc123\"\n";
        assert_eq!(quoted_string(toml, "api_token"), Some("abc123".to_string()));
    }
}
