//! Parses the strip list out of `archive.php`, then (separately, since it
//! needs the network) resolves each strip's actual image file — extension
//! probing megatokyo.com never advertises up front, ported from the
//! original .NET `StripsParser.GetFileTypeAsync`.
//!
//! Verified live against the current site (2026-08): each strip entry is
//! `<li><a title="August 14th, 2000" name="1" href="./strip/1">0001 - E3
//! Nightmare Begins</a></li>`, inside the same `<div class="content">` that
//! [`crate::scraper::chapters`] reads the category from. This differs from
//! the original scraper, which expected the strip's title in the `title`
//! attribute alongside the date — here `title` is *only* the date, and the
//! strip's own title is the `"NNNN - Title"` link text instead.

use scraper::{Html, Selector};

use crate::domain::Strip;
use crate::scraper::date;

/// A strip as read off `archive.php`, before its image URL/extension has
/// been resolved (that step hits the network — see [`resolve`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedStrip {
    pub number: i32,
    pub category: String,
    pub title: String,
    pub publish_date: String,
}

pub fn parse(html: &str) -> Vec<UnresolvedStrip> {
    let document = Html::parse_document(html);
    let content_sel = Selector::parse("div.content").unwrap();
    let category_sel = Selector::parse("h2 > a[id]").unwrap();
    let strip_sel = Selector::parse("li > a[title]").unwrap();

    let mut strips = Vec::new();
    for content in document.select(&content_sel) {
        let Some(category) = content
            .select(&category_sel)
            .next()
            .and_then(|a| a.value().attr("id"))
        else {
            continue;
        };

        for anchor in content.select(&strip_sel) {
            let Some(number) = anchor
                .value()
                .attr("name")
                .and_then(|n| n.parse::<i32>().ok())
            else {
                continue;
            };
            let Some(date_attr) = anchor.value().attr("title") else {
                continue;
            };
            let Some(publish_date) = date::parse(date_attr) else {
                continue;
            };
            let text: String = anchor.text().collect();
            let title = text
                .split_once(" - ")
                .map(|(_, title)| title.trim().to_string())
                .unwrap_or_else(|| text.trim().to_string());

            strips.push(UnresolvedStrip {
                number,
                category: category.to_string(),
                title,
                publish_date,
            });
        }
    }
    strips
}

/// Extensions to probe, in order, for strips before/from #1081 — the
/// original scraper's cutoff for when megatokyo.com switched its default
/// strip format from GIF to PNG. Ported verbatim from
/// `StripsParser.GetFileTypeAsync`.
const FORMATS_BEFORE_1081: [&str; 3] = ["gif", "jpg", "png"];
const FORMATS_FROM_1081: [&str; 3] = ["png", "jpg", "gif"];

/// Probes `https://megatokyo.com/strips/<NNNN>.<ext>` with HTTP HEAD
/// requests in the order [`FORMATS_BEFORE_1081`]/[`FORMATS_FROM_1081`]
/// dictate, and returns a [`Strip`] with `url` set to the first one that
/// resolves — `None` if none do (mirrors the original's "skip this strip"
/// behavior on a `GetFileTypeAsync` failure).
pub async fn resolve(client: &reqwest::Client, strip: UnresolvedStrip) -> Option<Strip> {
    resolve_against(client, strip, "https://megatokyo.com/strips").await
}

/// [`resolve`]'s actual logic, parameterized on the strip-image base URL so
/// tests can point it at a local mock server instead of the real site.
async fn resolve_against(
    client: &reqwest::Client,
    strip: UnresolvedStrip,
    base_url: &str,
) -> Option<Strip> {
    let base = format!("{base_url}/{:04}", strip.number);
    let formats: &[&str] = if strip.number < 1081 {
        &FORMATS_BEFORE_1081
    } else {
        &FORMATS_FROM_1081
    };
    for ext in formats {
        let url = format!("{base}.{ext}");
        if head_ok(client, &url).await {
            return Some(Strip {
                number: strip.number,
                category: strip.category,
                title: strip.title,
                url,
                publish_date: strip.publish_date,
            });
        }
    }
    None
}

async fn head_ok(client: &reqwest::Client, url: &str) -> bool {
    client
        .head(url)
        .send()
        .await
        .map(|response| response.status().is_success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> String {
        std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/archive.html"
        ))
        .unwrap()
    }

    #[test]
    fn parses_the_first_strip_of_the_prologue() {
        let strips = parse(&fixture());
        let first = strips.iter().find(|s| s.number == 1).unwrap();
        assert_eq!(first.category, "C-0");
        assert_eq!(first.title, "E3 Nightmare Begins");
        assert_eq!(first.publish_date, "2000-08-14T00:00:00Z");
    }

    #[test]
    fn parses_every_strip_with_no_gaps_or_duplicates() {
        let strips = parse(&fixture());
        let mut numbers: Vec<i32> = strips.iter().map(|s| s.number).collect();
        numbers.sort_unstable();
        numbers.dedup();
        assert_eq!(numbers.len(), strips.len(), "duplicate strip numbers");
        assert_eq!(numbers, (1..=1619).collect::<Vec<_>>());
    }

    #[test]
    fn every_strip_belongs_to_the_category_it_was_listed_under() {
        let strips = parse(&fixture());
        assert!(strips
            .iter()
            .filter(|s| s.category == "C-13")
            .all(|s| s.number > 0));
    }

    fn sample(number: i32) -> UnresolvedStrip {
        UnresolvedStrip {
            number,
            category: "C-13".to_string(),
            title: "Sample".to_string(),
            publish_date: "2023-09-27T00:00:00Z".to_string(),
        }
    }

    #[tokio::test]
    async fn resolve_tries_gif_jpg_png_in_order_before_strip_1081() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("HEAD"))
            .and(path("/0050.gif"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        Mock::given(method("HEAD"))
            .and(path("/0050.jpg"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        let strip = resolve_against(&client, sample(50), &server.uri())
            .await
            .unwrap();
        assert_eq!(strip.url, format!("{}/0050.jpg", server.uri()));
    }

    #[tokio::test]
    async fn resolve_tries_png_jpg_gif_in_order_from_strip_1081() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("HEAD"))
            .and(path("/1200.png"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        let strip = resolve_against(&client, sample(1200), &server.uri())
            .await
            .unwrap();
        assert_eq!(strip.url, format!("{}/1200.png", server.uri()));
    }

    #[tokio::test]
    async fn resolve_returns_none_when_no_format_is_found() {
        let server = wiremock::MockServer::start().await;
        // No mocks mounted: every HEAD 404s by default.
        let client = reqwest::Client::new();
        assert!(resolve_against(&client, sample(50), &server.uri())
            .await
            .is_none());
    }
}
