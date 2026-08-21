//! Parses the chapter list out of `archive.php`.
//!
//! Verified live against the current site (2026-08): each section is a
//! `<div class="content">` starting with `<h2><a id="CATEGORY">Label</a></h2>`
//! — this differs from the original .NET scraper's XPath, which expected the
//! anchor's id to live on an ancestor `<div>` rather than on the `<a>` inside
//! the `<h2>`. Chapters proper look like `Chapter 1: &quot;Title&quot;`
//! (decoded to a literal `"` by the HTML parser); `Chapter 0` is the
//! prologue and carries no quoted title; every other section (`One Shot
//! Episode`, `Grand Theft Colo`, ...) is a bare label with no `Chapter`
//! prefix and gets `number: 0`.

use scraper::{Html, Selector};

use crate::domain::Chapter;

pub fn parse(html: &str) -> Vec<Chapter> {
    let document = Html::parse_document(html);
    let content_sel = Selector::parse("div.content").unwrap();
    let anchor_sel = Selector::parse("h2 > a[id]").unwrap();

    document
        .select(&content_sel)
        .filter_map(|content| {
            let anchor = content.select(&anchor_sel).next()?;
            let category = anchor.value().attr("id")?.to_string();
            let text: String = anchor.text().collect();
            Some(parse_chapter_label(category, text.trim()))
        })
        .collect()
}

fn parse_chapter_label(category: String, label: &str) -> Chapter {
    if let Some(rest) = label.strip_prefix("Chapter ") {
        if rest == "0" {
            return Chapter {
                number: 0,
                category,
                title: "Prologue".to_string(),
            };
        }
        if let Some((number_str, quoted_title)) = rest.split_once(':') {
            if let Ok(number) = number_str.trim().parse() {
                let title = quoted_title.trim().trim_matches('"').to_string();
                return Chapter {
                    number,
                    category,
                    title,
                };
            }
        }
    }
    Chapter {
        number: 0,
        category,
        title: label.to_string(),
    }
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
    fn parses_the_prologue_as_chapter_zero() {
        let chapters = parse(&fixture());
        let prologue = chapters.iter().find(|c| c.category == "C-0").unwrap();
        assert_eq!(prologue.number, 0);
        assert_eq!(prologue.title, "Prologue");
    }

    #[test]
    fn parses_numbered_chapters_with_quoted_titles() {
        let chapters = parse(&fixture());
        let chapter_13 = chapters.iter().find(|c| c.category == "C-13").unwrap();
        assert_eq!(chapter_13.number, 13);
        assert_eq!(chapter_13.title, "Redemption");
    }

    #[test]
    fn parses_non_chapter_sections_as_number_zero() {
        let chapters = parse(&fixture());
        let ose = chapters.iter().find(|c| c.category == "OSE").unwrap();
        assert_eq!(ose.number, 0);
        assert_eq!(ose.title, "One Shot Episode");
    }

    #[test]
    fn finds_every_known_section_from_the_live_fixture() {
        let chapters = parse(&fixture());
        let categories: Vec<&str> = chapters.iter().map(|c| c.category.as_str()).collect();
        for expected in [
            "C-0", "C-1", "C-2", "C-3", "C-4", "C-5", "C-6", "C-7", "C-8", "C-9", "C-10", "C-11",
            "C-12", "C-13", "OSE", "GTC", "CIR", "NNM", "UNM", "FMP", "END", "B34CH", "EVN", "DPD",
            "SGD", "GST",
        ] {
            assert!(
                categories.contains(&expected),
                "missing category {expected}"
            );
        }
    }
}
