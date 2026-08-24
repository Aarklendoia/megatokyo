//! Parses `https://megatokyo.com/rant-archive.php`, the full historical
//! index of every rant ever published (grouped by author, not
//! chronologically). Unlike the RSS feed — which only ever carries the
//! last handful of items (verified live: 5 rants, out of 1000+ total) —
//! this page is the only place that lists every rant number that has ever
//! existed, which is what a full backfill needs to discover them all.
//!
//! Each entry looks like
//! `<a title="September 18th, 2014" name="1081" href="./rant/1081">The
//! Tower of Kartage (It's Here!)</a>` (verified live, 2026-08) — the `name`
//! attribute is the rant number. This page only lists numbers, not
//! content: resolving one still means following `/rant/<n>`'s redirect to
//! whichever strip page hosts it and parsing that page with
//! `scraper::rants`, same as the feed-driven path already does.

use scraper::{Html, Selector};

pub const RANT_ARCHIVE_URL: &str = "https://megatokyo.com/rant-archive.php";

/// Every rant number referenced on the archive page, in whatever order
/// they appear on it (grouped by author). May contain duplicates if a
/// rant is ever cross-listed; callers already skip numbers they've stored
/// (see `daemon::poll`), so this doesn't bother deduplicating itself.
pub fn parse_numbers(html: &str) -> Vec<i32> {
    let document = Html::parse_document(html);
    let sel = Selector::parse(r#"a[name][href^="./rant/"]"#).unwrap();

    document
        .select(&sel)
        .filter_map(|a| a.value().attr("name")?.parse().ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> String {
        std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/rant_archive.html"
        ))
        .unwrap()
    }

    #[test]
    fn parses_every_rant_number_on_the_fixture_page() {
        let numbers = parse_numbers(&fixture());
        assert_eq!(numbers.len(), 1078);
        assert!(numbers.contains(&996));
        assert!(numbers.contains(&1107));
        // The oldest rant on record (see the fixture's "Dom" section).
        assert!(numbers.contains(&136));
    }
}
