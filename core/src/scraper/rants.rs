//! Parses the rants (Fred Gallagher's blog posts, embedded alongside the
//! comic) off an individual `https://megatokyo.com/strip/<n>` page. A strip
//! page carries zero, one or two rants.
//!
//! Verified live (2026-08) against `strip/1619`, which carries two. The
//! container's `id` format changed since the original .NET scraper: it's
//! now `id="rant1106"` (a bare number), where the original expected
//! `r1106t` and stripped a leading `r`/trailing `t`. The rant's title also
//! moved from the link's `title` attribute into its text content, and the
//! byline (`<h3>`) now HTML-encodes its angle brackets (`&lt; Piro &gt;`)
//! rather than using literal ones.

use scraper::{ElementRef, Html, Selector};

use crate::domain::Rant;
use crate::scraper::date;

/// `https://megatokyo.com/strip/<n>` — same page [`crate::scraper::rants`]
/// itself parses, and (incidentally) the page whose comic image the
/// original .NET client linked to as a strip's "read this strip" URL.
pub fn strip_page_url(number: i32) -> String {
    format!("https://megatokyo.com/strip/{number}")
}

pub fn parse(html: &str) -> Vec<Rant> {
    let document = Html::parse_document(html);
    let rant_sel = Selector::parse(r#"div.mainrant[id^="rant"]"#).unwrap();

    document.select(&rant_sel).filter_map(parse_one).collect()
}

fn parse_one(container: ElementRef) -> Option<Rant> {
    let number: i32 = container
        .value()
        .attr("id")?
        .strip_prefix("rant")?
        .parse()
        .ok()?;

    let author = text_of(&container, "h3")?
        .trim()
        .trim_start_matches('<')
        .trim_end_matches('>')
        .trim()
        .to_string();

    let title_anchor = first_match(&container, "h4 a")?;
    let title: String = title_anchor
        .text()
        .collect::<String>()
        .trim()
        .trim_matches('"')
        .to_string();

    let image_src = first_match(&container, ".rantimage img")?
        .value()
        .attr("src")?
        .to_string();
    let url = format!("https://megatokyo.com/{image_src}");

    let date_text = text_of(&container, "p.date")?;
    let date_part = date_text
        .split_once(" - ")
        .map(|(_, d)| d)
        .unwrap_or(&date_text);
    let publish_date = date::parse(date_part.trim())?;

    let content = first_match(&container, ".rantbody")?.inner_html();

    Some(Rant {
        number,
        author,
        title,
        url,
        publish_date,
        content,
    })
}

fn first_match<'a>(container: &ElementRef<'a>, selector: &str) -> Option<ElementRef<'a>> {
    container.select(&Selector::parse(selector).unwrap()).next()
}

fn text_of(container: &ElementRef, selector: &str) -> Option<String> {
    Some(first_match(container, selector)?.text().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> String {
        std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/strip_1619.html"
        ))
        .unwrap()
    }

    #[test]
    fn parses_both_rants_on_the_fixture_page() {
        let rants = parse(&fixture());
        assert_eq!(rants.len(), 2);
        let numbers: Vec<i32> = rants.iter().map(|r| r.number).collect();
        assert_eq!(numbers, vec![1106, 1107]);
    }

    #[test]
    fn parses_author_title_url_date_and_content_of_the_first_rant() {
        let rants = parse(&fixture());
        let rant = rants.iter().find(|r| r.number == 1106).unwrap();
        assert_eq!(rant.author, "Piro");
        assert_eq!(rant.title, "Clearing of the Air");
        assert_eq!(rant.url, "https://megatokyo.com/rantimgs/1106.png");
        assert_eq!(rant.publish_date, "2023-09-27T00:00:00Z");
        assert!(rant.content.contains("twitter thread"));
    }
}
