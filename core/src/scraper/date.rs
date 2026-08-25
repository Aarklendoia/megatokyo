//! Parses the English long-form dates megatokyo.com uses in two slightly
//! different shapes: `"August 14th, 2000"` (archive strip listing, no
//! weekday) and `"Wednesday - September 27, 2023"` (rant byline, weekday
//! prefix — callers strip everything up to the last `" - "` before calling
//! [`parse`]). No date crate: this is the only place megatokyo-core needs
//! date arithmetic, and it never needs anything beyond "turn this into an
//! ISO 8601 UTC-midnight string to store and compare".

/// Parses `"<Month> <Day><ordinal suffix>, <Year>"` (the comma and ordinal
/// suffix are both optional — either shape is accepted) into an ISO 8601
/// string (`YYYY-MM-DDT00:00:00Z`, UTC midnight — megatokyo.com doesn't
/// publish a time of day for strips/rants, so this is a date, not a instant).
pub fn parse(text: &str) -> Option<String> {
    let cleaned = strip_ordinal_suffixes(text).replace(',', "");
    let parts: Vec<&str> = cleaned.split_whitespace().collect();
    let [month_name, day, year] = parts.as_slice() else {
        return None;
    };
    let month = month_number(month_name)?;
    let day: u32 = day.parse().ok()?;
    let year: i32 = year.parse().ok()?;
    // Validates the day against the actual days in `month`/`year` (leap
    // years included) rather than a flat 1..=31 range, which would accept
    // e.g. "February 30" or "April 31".
    chrono::NaiveDate::from_ymd_opt(year, month, day)?;
    Some(format!("{year:04}-{month:02}-{day:02}T00:00:00Z"))
}

/// Removes "th"/"st"/"nd"/"rd" immediately following a run of digits (e.g.
/// `14th` -> `14`). Scoped to right after digits so it can't accidentally
/// eat letters out of a month name.
fn strip_ordinal_suffixes(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_ascii_digit() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            out.extend(&chars[start..i]);
            if i + 1 < chars.len() {
                let suffix: String = chars[i..i + 2].iter().collect::<String>().to_lowercase();
                if matches!(suffix.as_str(), "th" | "st" | "nd" | "rd") {
                    i += 2;
                }
            }
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

fn month_number(name: &str) -> Option<u32> {
    const MONTHS: [&str; 12] = [
        "january",
        "february",
        "march",
        "april",
        "may",
        "june",
        "july",
        "august",
        "september",
        "october",
        "november",
        "december",
    ];
    let lower = name.to_ascii_lowercase();
    MONTHS
        .iter()
        .position(|m| *m == lower)
        .map(|i| i as u32 + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_archive_style_dates_with_ordinal_suffix_and_comma() {
        assert_eq!(
            parse("August 14th, 2000"),
            Some("2000-08-14T00:00:00Z".to_string())
        );
        assert_eq!(
            parse("September 1st, 2000"),
            Some("2000-09-01T00:00:00Z".to_string())
        );
        assert_eq!(
            parse("September 22nd, 2000"),
            Some("2000-09-22T00:00:00Z".to_string())
        );
        assert_eq!(
            parse("October 23rd, 2000"),
            Some("2000-10-23T00:00:00Z".to_string())
        );
    }

    #[test]
    fn parses_rant_style_dates_after_the_weekday_prefix_is_stripped() {
        assert_eq!(
            parse("September 27, 2023"),
            Some("2023-09-27T00:00:00Z".to_string())
        );
    }

    #[test]
    fn rejects_garbage() {
        assert_eq!(parse("not a date"), None);
        assert_eq!(parse(""), None);
        assert_eq!(parse("Marchtober 40, 2000"), None);
    }

    #[test]
    fn rejects_a_day_that_does_not_exist_in_the_given_month() {
        assert_eq!(parse("February 30, 2000"), None);
        assert_eq!(parse("April 31, 2000"), None);
        // 2000 is a leap year: February 29 is valid there...
        assert_eq!(
            parse("February 29, 2000"),
            Some("2000-02-29T00:00:00Z".to_string())
        );
        // ...but not in 2001.
        assert_eq!(parse("February 29, 2001"), None);
    }
}
