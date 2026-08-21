//! Parses megatokyo's RSS feed to detect new strips/rants without re-scraping
//! `archive.php` on every poll cycle — mirrors the original .NET
//! `FeedManager`, but as pure parsing (no DB access, no notification side
//! effects: see the plan's `daemon::poll` for how the diff against the
//! `checking` table and the actual re-scrape are driven).
//!
//! Verified live (2026-08): still a standard RSS 2.0 feed, `<title>` values
//! shaped `Comic [1619] "Beautiful"` / `Rant [1107] "It Took Forever, But
//! It's Here!"` — unchanged from the original scraper's assumption.

use feed_rs::model::Entry;

pub const FEED_URL: &str = "https://megatokyo.com/rss/megatokyo.xml";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedItemKind {
    Strip,
    Rant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedItem {
    pub kind: FeedItemKind,
    pub number: i32,
    pub title: String,
    /// ISO 8601 (RFC 3339), taken from the entry's `pubDate`.
    pub published_at: String,
    /// The entry's `<link>` — for a strip, its page directly; for a rant,
    /// `https://megatokyo.com/rant/<n>`, which redirects to whichever strip
    /// page actually hosts it (rants aren't addressable by their own page,
    /// see `scraper::rants`). Callers that need the strip page should fetch
    /// this URL and let the HTTP client follow the redirect, rather than
    /// trying to derive a strip number from the rant number.
    pub link: String,
}

#[derive(Debug, thiserror::Error)]
pub enum FeedError {
    #[error("could not parse the feed: {0}")]
    Parse(#[from] feed_rs::parser::ParseFeedError),
    #[error("could not fetch the feed: {0}")]
    Fetch(#[from] reqwest::Error),
}

pub fn parse(xml: &[u8]) -> Result<Vec<FeedItem>, FeedError> {
    let feed = feed_rs::parser::parse(xml)?;
    Ok(feed.entries.iter().filter_map(entry_to_item).collect())
}

pub async fn fetch(client: &reqwest::Client) -> Result<Vec<FeedItem>, FeedError> {
    let bytes = client.get(FEED_URL).send().await?.bytes().await?;
    parse(&bytes)
}

/// `Comic [1619] "Beautiful"` -> `Some((Strip, 1619, "Beautiful"))`;
/// anything else (a non-comic/rant announcement, a malformed title) is
/// skipped rather than erroring — a poll cycle shouldn't fail outright just
/// because one feed item doesn't match the expected shape.
fn entry_to_item(entry: &Entry) -> Option<FeedItem> {
    let raw_title = entry.title.as_ref()?.content.as_str();
    let (kind, rest) = if let Some(rest) = raw_title.strip_prefix("Comic ") {
        (FeedItemKind::Strip, rest)
    } else {
        let rest = raw_title.strip_prefix("Rant ")?;
        (FeedItemKind::Rant, rest)
    };

    let rest = rest.strip_prefix('[')?;
    let (number_str, after_bracket) = rest.split_once(']')?;
    let number: i32 = number_str.trim().parse().ok()?;
    let title = after_bracket.trim().trim_matches('"').to_string();

    let published = entry.published.or(entry.updated)?;
    let link = entry.links.first()?.href.clone();
    Some(FeedItem {
        kind,
        number,
        title,
        published_at: published.to_rfc3339(),
        link,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<u8> {
        std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/feed.xml"
        ))
        .unwrap()
    }

    #[test]
    fn parses_strip_and_rant_entries_with_their_number_and_title() {
        let items = parse(&fixture()).unwrap();

        let strip = items
            .iter()
            .find(|i| i.kind == FeedItemKind::Strip && i.number == 1619)
            .unwrap();
        assert_eq!(strip.title, "Beautiful");

        let rant = items
            .iter()
            .find(|i| i.kind == FeedItemKind::Rant && i.number == 1107)
            .unwrap();
        assert_eq!(rant.title, "It Took Forever, But It's Here!");
    }

    #[test]
    fn a_rant_items_link_is_its_own_permalink_not_a_strip_page() {
        // https://megatokyo.com/rant/<n> 301-redirects to whichever strip
        // page actually hosts that rant (verified live) — callers fetch
        // this URL and let the HTTP client follow the redirect, rather than
        // trying to derive a strip number from the rant number themselves.
        let items = parse(&fixture()).unwrap();
        let rant = items
            .iter()
            .find(|i| i.kind == FeedItemKind::Rant && i.number == 1107)
            .unwrap();
        assert_eq!(rant.link, "https://megatokyo.com/rant/1107");
    }

    #[test]
    fn a_strip_items_link_is_its_strip_page() {
        let items = parse(&fixture()).unwrap();
        let strip = items
            .iter()
            .find(|i| i.kind == FeedItemKind::Strip && i.number == 1619)
            .unwrap();
        assert_eq!(strip.link, "https://megatokyo.com/strip/1619");
    }

    #[test]
    fn every_item_has_a_parsed_publish_date() {
        let items = parse(&fixture()).unwrap();
        assert!(!items.is_empty());
        for item in &items {
            assert!(!item.published_at.is_empty());
        }
    }

    #[test]
    fn items_stay_ordered_newest_first_as_the_feed_provides_them() {
        // megatokyo's feed is already newest-first; the poll loop's diff
        // logic (comparing published_at to the stored checkpoint) depends
        // on that ordering to stop early rather than scan the whole feed.
        let items = parse(&fixture()).unwrap();
        let dates: Vec<&str> = items.iter().map(|i| i.published_at.as_str()).collect();
        let mut sorted = dates.clone();
        sorted.sort_unstable_by(|a, b| b.cmp(a));
        assert_eq!(dates, sorted);
    }
}
