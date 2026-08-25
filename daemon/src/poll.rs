//! Backfill + periodic poll loop — reimplements the original .NET
//! `WebSiteParser`/`FeedManager`'s intent, not its literal logic: the
//! original's `FeedManager.LoadAsync` only added a feed-detected item to its
//! notification list when the item was *already* in the database (`if
//! (await mediator.Send(new GetStripQuery(strip.Number)) != null)`), which
//! looks backwards for "notify on new content" and is not reproduced here.
//! This instead treats every feed item newer than the last check as
//! something to backfill.
//!
//! No desktop notifications are sent from here: the daemon may run headless
//! on a remote server (see the plan's "Déploiement distant"). Clients poll
//! `/status` and notify themselves — see the `gui --background` issue.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use megatokyo_core::domain::{Checking, Rant};
use megatokyo_core::scraper::{self, strips::UnresolvedStrip};
use megatokyo_core::{feed, store::Store};

use crate::control::AppState;

/// Real strip-image probe base — see `scraper::strips::resolve`'s own
/// hardcoded default. Threaded through explicitly (rather than each
/// function calling `scraper::strips::resolve` directly) so tests can
/// substitute a mock server, same seam as `Translator::with_endpoint`.
const STRIP_IMAGE_BASE_URL: &str = "https://megatokyo.com/strips";

/// Real per-rant redirect base — see [`resolve_and_store_rants`]'s doc
/// comment on why a rant number resolves through this URL rather than one
/// of its own.
const RANT_LINK_BASE_URL: &str = "https://megatokyo.com/rant";

/// One full pass: backfill if the store is empty, then a feed diff either
/// way (a fresh backfill's own scrape can itself lag behind the feed by the
/// time it finishes, so this isn't an `else`).
pub async fn run_once(client: &reqwest::Client, state: &AppState) {
    state.backfilling.store(true, Ordering::Relaxed);
    if let Err(err) = backfill_if_empty(client, &state.store).await {
        log::warn!("backfill failed: {err}");
    }
    if let Err(err) = backfill_rants(client, &state.store).await {
        log::warn!("rant backfill failed: {err}");
    }
    state.backfilling.store(false, Ordering::Relaxed);

    if let Err(err) = check_feed(client, &state.store).await {
        log::warn!("feed check failed: {err}");
    }
}

/// Runs [`run_once`] immediately, then again every
/// `state.config`'s `poll_interval_minutes` (re-read fresh each time round
/// the loop, so a change made via `POST /config` takes effect on the very
/// next sleep rather than needing a restart), and immediately whenever
/// `state.check_requested` is notified (`POST /check`) — whichever comes
/// first. A `poll_in_progress` guard (mirroring the original's
/// `_workInProgress` flag) means an in-flight cycle just keeps running
/// rather than being interrupted or double-started by an overlapping
/// trigger.
pub async fn run_loop(client: reqwest::Client, state: Arc<AppState>) {
    let poll_in_progress = AtomicBool::new(false);
    loop {
        if poll_in_progress
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            run_once(&client, &state).await;
            poll_in_progress.store(false, Ordering::SeqCst);
        }
        let interval =
            std::time::Duration::from_secs(state.config.read().await.poll_interval_minutes * 60);
        tokio::select! {
            _ = tokio::time::sleep(interval) => {}
            _ = state.check_requested.notified() => {}
        }
    }
}

async fn backfill_if_empty(
    client: &reqwest::Client,
    store: &Store,
) -> Result<(), Box<dyn std::error::Error>> {
    if store.has_any_strip()? {
        return Ok(());
    }
    log::info!("no strips in the database yet, running the initial backfill");
    backfill(client, store).await
}

async fn backfill(
    client: &reqwest::Client,
    store: &Store,
) -> Result<(), Box<dyn std::error::Error>> {
    backfill_at(client, store, scraper::ARCHIVE_URL, STRIP_IMAGE_BASE_URL).await
}

/// [`backfill`]'s actual logic, parameterized on the archive and
/// strip-image URLs so tests can point it at a local mock server instead
/// of the real site.
async fn backfill_at(
    client: &reqwest::Client,
    store: &Store,
    archive_url: &str,
    strip_image_base_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let archive_html = client.get(archive_url).send().await?.text().await?;

    for chapter in scraper::chapters::parse(&archive_html) {
        store.upsert_chapter(&chapter)?;
    }

    let unresolved = scraper::strips::parse(&archive_html);
    log::info!("backfilling {} strips", unresolved.len());
    resolve_and_store_strips(client, store, unresolved, strip_image_base_url).await;
    Ok(())
}

/// Up to this many strip-image probes in flight at once during a backfill.
/// The original .NET scraper probed sequentially (part of why the old
/// UWP client's first launch was so slow, per the plan) — each probe is 1-3
/// independent HEAD requests to megatokyo.com, so bounded concurrency here
/// turns a ~1600-strip cold backfill from "one HTTP round-trip at a time"
/// into a handful of seconds, without hammering the site with 1600+
/// simultaneous connections either.
const BACKFILL_CONCURRENCY: usize = 16;

/// Resolves each strip's image extension and stores it — skips strips
/// already in the database (matches the original's `stripsInDatabase`
/// filter: no point re-probing megatokyo.com for a strip we already have).
/// `strip_image_base_url` is [`STRIP_IMAGE_BASE_URL`] in production, a mock
/// server in tests.
async fn resolve_and_store_strips(
    client: &reqwest::Client,
    store: &Store,
    unresolved: Vec<UnresolvedStrip>,
    strip_image_base_url: &str,
) {
    let to_resolve: Vec<UnresolvedStrip> = unresolved
        .into_iter()
        .filter(|strip| !matches!(store.strip_by_number(strip.number), Ok(Some(_))))
        .collect();

    let semaphore = Arc::new(tokio::sync::Semaphore::new(BACKFILL_CONCURRENCY));
    let mut tasks = tokio::task::JoinSet::new();
    for strip in to_resolve {
        let client = client.clone();
        let semaphore = semaphore.clone();
        let base_url = strip_image_base_url.to_string();
        tasks.spawn(async move {
            let _permit = semaphore.acquire_owned().await;
            let number = strip.number;
            (
                number,
                scraper::strips::resolve_against(&client, strip, &base_url).await,
            )
        });
    }

    while let Some(result) = tasks.join_next().await {
        match result {
            Ok((number, Some(resolved))) => {
                if let Err(err) = store.upsert_strip(&resolved) {
                    log::warn!("could not store strip {number}: {err}");
                }
            }
            Ok((number, None)) => log::warn!("could not resolve an image for strip {number}"),
            Err(join_err) => log::warn!("strip resolve task failed: {join_err}"),
        }
    }
}

/// Backfills every rant that has ever existed, not just whatever the RSS
/// feed still carries (5 at a time, verified live — see
/// `scraper::rant_archive`'s doc comment). Runs every cycle, same as
/// `check_feed` below, rather than gating on "the rants table is empty":
/// a daemon that had already run *before* this existed has a handful of
/// rants from the old feed-only path, which would make an empty-table
/// check wrongly conclude "already fully backfilled" and skip it forever.
/// [`resolve_and_store_rants`] already skips numbers already in the
/// store, so a cycle after the first (full) one costs one archive fetch
/// plus a fast local lookup per number — no HTTP fetches at all once
/// nothing new has been added to the archive.
async fn backfill_rants(
    client: &reqwest::Client,
    store: &Store,
) -> Result<(), Box<dyn std::error::Error>> {
    backfill_rants_at(
        client,
        store,
        scraper::rant_archive::RANT_ARCHIVE_URL,
        RANT_LINK_BASE_URL,
    )
    .await
}

/// [`backfill_rants`]'s actual logic, parameterized on the rant-archive and
/// per-rant-link URLs so tests can point it at a local mock server instead
/// of the real site.
async fn backfill_rants_at(
    client: &reqwest::Client,
    store: &Store,
    rant_archive_url: &str,
    rant_link_base_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let html = client.get(rant_archive_url).send().await?.text().await?;
    let numbers = scraper::rant_archive::parse_numbers(&html);
    resolve_and_store_rants(client, store, numbers, rant_link_base_url).await;
    Ok(())
}

/// Same bounded-concurrency shape as [`resolve_and_store_strips`]: each
/// task only fetches and parses (network + CPU, both `Send`), and the
/// result is stored back on the caller's thread afterwards — `Store`'s
/// `Connection` lives behind a plain `Mutex`, not an `Arc`, so it can't be
/// moved into a spawned task, only borrowed sequentially here. A `/rant/<n>`
/// fetch resolves (via redirect) to a strip page that can host up to two
/// rants, so a number already picked up as a side effect of an earlier
/// fetch in this same pass is skipped rather than re-fetched.
async fn resolve_and_store_rants(
    client: &reqwest::Client,
    store: &Store,
    numbers: Vec<i32>,
    rant_link_base_url: &str,
) {
    let semaphore = Arc::new(tokio::sync::Semaphore::new(BACKFILL_CONCURRENCY));
    let mut tasks = tokio::task::JoinSet::new();
    for number in numbers {
        // Already picked up by an earlier fetch in this same pass (see the
        // doc comment above) — no point re-fetching its strip page.
        if matches!(store.rant_by_number(number), Ok(Some(_))) {
            continue;
        }
        let client = client.clone();
        let semaphore = semaphore.clone();
        let link = format!("{rant_link_base_url}/{number}");
        tasks.spawn(async move {
            let _permit = semaphore.acquire_owned().await;
            (number, fetch_rants_at(&client, &link).await)
        });
    }

    while let Some(result) = tasks.join_next().await {
        match result {
            Ok((_, Ok(rants))) => store_rants(store, rants),
            Ok((number, Err(err))) => log::warn!("could not fetch rant {number}: {err}"),
            Err(join_err) => log::warn!("rant resolve task failed: {join_err}"),
        }
    }
}

/// Stores every rant in `rants`, logging (not failing) on a per-rant store
/// error — shared by [`resolve_and_store_rants`] and [`check_feed`], which
/// both end up with an already-fetched `Vec<Rant>` to persist the same way.
fn store_rants(store: &Store, rants: Vec<Rant>) {
    for rant in rants {
        if let Err(err) = store.upsert_rant(&rant) {
            log::warn!("could not store rant {}: {err}", rant.number);
        }
    }
}

/// Diffs the RSS feed against the stored `checking` checkpoint, backfills
/// any strip/rant newer than the checkpoint, and advances it.
async fn check_feed(
    client: &reqwest::Client,
    store: &Store,
) -> Result<(), Box<dyn std::error::Error>> {
    check_feed_at(
        client,
        store,
        feed::FEED_URL,
        scraper::ARCHIVE_URL,
        STRIP_IMAGE_BASE_URL,
    )
    .await
}

/// [`check_feed`]'s actual logic, parameterized on the feed and archive
/// URLs so tests can point it at a local mock server instead of the real
/// site.
async fn check_feed_at(
    client: &reqwest::Client,
    store: &Store,
    feed_url: &str,
    archive_url: &str,
    strip_image_base_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let items = feed::fetch_at(client, feed_url).await?;
    let mut checking = store.get_checking()?;
    let last_check = checking.last_check.clone().unwrap_or_default();
    let new_items = new_items(&items, &last_check);

    if !new_items.is_empty() {
        log::info!("{} new feed item(s) since the last check", new_items.len());
        // A new strip/rant re-triggers an archive rescrape rather than
        // trying to scrape just the one item from the feed alone: the feed
        // only carries a number and a title, not the chapter/category a
        // strip needs — archive.php is the only place that mapping lives.
        // resolve_and_store_strips already skips strips already in the
        // database, so this stays cheap in the common one-new-strip case.
        if let Err(err) = backfill_at(client, store, archive_url, strip_image_base_url).await {
            log::warn!("could not rescrape the archive after a feed change: {err}");
        }
        for item in &new_items {
            if item.kind == feed::FeedItemKind::Rant {
                match fetch_rants_at(client, &item.link).await {
                    Ok(rants) => store_rants(store, rants),
                    Err(err) => log::warn!(
                        "could not fetch rant {} at {}: {err}",
                        item.number,
                        item.link
                    ),
                }
            }
        }
    }

    apply_checkpoint(&mut checking, &new_items);
    store.save_checking(&checking)?;
    Ok(())
}

/// Advances `checking`'s per-kind last-seen number to the highest number
/// seen across `new_items` of that kind, and stamps `last_check` to now.
/// Takes the max rather than the last item processed: `new_items` is
/// newest-first (the feed's own order, see `feed.rs`'s
/// `items_stay_ordered_newest_first_as_the_feed_provides_them`), so a
/// last-write-wins overwrite would record the *oldest* item of a multi-item
/// batch instead of the newest. Pure, so it's unit-testable without
/// touching the network.
fn apply_checkpoint(checking: &mut Checking, new_items: &[&feed::FeedItem]) {
    for item in new_items {
        match item.kind {
            feed::FeedItemKind::Strip => {
                checking.last_strip_number = checking.last_strip_number.max(item.number)
            }
            feed::FeedItemKind::Rant => {
                checking.last_rant_number = checking.last_rant_number.max(item.number)
            }
        }
    }
    checking.last_check = Some(chrono::Utc::now().to_rfc3339());
}

/// Rants aren't addressable by their own page: `https://megatokyo.com/rant/<n>`
/// (a feed item's `link`, or a number from `scraper::rant_archive`) 301-
/// redirects to whichever strip page actually hosts it (verified live —
/// see `feed::FeedItem::link`'s doc comment). `reqwest::Client` follows
/// redirects by default, so fetching `link` directly lands on the right
/// strip page without this daemon ever having to work out which strip
/// that is itself. Fetch-only (no store access) so callers can run this
/// concurrently across many links, see [`resolve_and_store_rants`].
async fn fetch_rants_at(client: &reqwest::Client, link: &str) -> Result<Vec<Rant>, reqwest::Error> {
    let html = client.get(link).send().await?.text().await?;
    Ok(scraper::rants::parse(&html))
}

/// Feed items whose `published_at` is strictly after `last_check` — parsed
/// as actual instants (via `DateTime::parse_from_rfc3339`), not compared as
/// raw strings: megatokyo's feed keeps each `pubDate`'s original `-08:00`
/// offset while `last_check` is always stored in UTC (see [`check_feed`]),
/// so a plain string comparison would misorder them whenever the wall-clock
/// digits happen to disagree with the actual instant (e.g. a `-08:00` 09:00
/// pubDate is later in absolute time than a `+00:00` 10:00 checkpoint, but
/// sorts earlier as a bare string).
fn new_items<'a>(items: &'a [feed::FeedItem], last_check: &str) -> Vec<&'a feed::FeedItem> {
    items
        .iter()
        .filter(|item| is_newer(&item.published_at, last_check))
        .collect()
}

fn is_newer(published_at: &str, last_check: &str) -> bool {
    let Ok(published) = chrono::DateTime::parse_from_rfc3339(published_at) else {
        return false;
    };
    match chrono::DateTime::parse_from_rfc3339(last_check) {
        Ok(last) => published > last,
        // No valid checkpoint yet (first run: `last_check` is `""`) —
        // everything currently in the feed counts as new.
        Err(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use megatokyo_core::feed::{FeedItem, FeedItemKind};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn item(published_at: &str) -> FeedItem {
        FeedItem {
            kind: FeedItemKind::Strip,
            number: 1,
            title: "Sample".to_string(),
            published_at: published_at.to_string(),
            link: "https://megatokyo.com/strip/1".to_string(),
        }
    }

    /// `core`'s own `strip_1619.html` fixture (two rants, #1106 and #1107)
    /// — reused here rather than duplicated, since it's exactly what a
    /// strip page's HTML looks like from `fetch_rants_at`'s point of view.
    fn strip_1619_fixture() -> String {
        std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../core/tests/fixtures/strip_1619.html"
        ))
        .unwrap()
    }

    fn sample_rant(number: i32) -> Rant {
        Rant {
            number,
            author: "Piro".to_string(),
            title: "Placeholder".to_string(),
            url: String::new(),
            publish_date: String::new(),
            content: String::new(),
        }
    }

    /// One chapter, two strips — small on purpose so a test backfill only
    /// ever probes a couple of strip-image URLs, not the ~1600 in `core`'s
    /// full `archive.html` fixture.
    const MINI_ARCHIVE_HTML: &str = r#"<div class="content"><h2><a id="C-1">Chapter 1: &quot;Test&quot;</a></h2><ul><li><a title="August 14th, 2000" name="1" href="./strip/1">0001 - First Strip</a></li><li><a title="August 15th, 2000" name="2" href="./strip/2">0002 - Second Strip</a></li></ul></div>"#;

    fn feed_xml(items_xml: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<rss version="2.0"><channel>
<title>Megatokyo Comics and News</title>
<link>https://megatokyo.com</link>
<description>News and Comics from Megatokyo.</description>
{items_xml}
</channel></rss>"#
        )
    }

    fn feed_item_xml(title: &str, link: &str, pub_date: &str) -> String {
        format!(
            r#"<item><title>{title}</title><link>{link}</link><guid isPermaLink="true">{link}</guid><pubDate>{pub_date}</pubDate></item>"#
        )
    }

    #[test]
    fn everything_is_new_when_there_is_no_checkpoint_yet() {
        let items = vec![item("2023-09-27T00:00:00Z")];
        assert_eq!(new_items(&items, "").len(), 1);
    }

    #[test]
    fn only_items_strictly_after_the_checkpoint_are_new() {
        let items = vec![item("2023-09-27T00:00:00Z"), item("2023-09-26T00:00:00Z")];
        let result = new_items(&items, "2023-09-26T12:00:00Z");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].published_at, "2023-09-27T00:00:00Z");
    }

    #[test]
    fn compares_actual_instants_not_the_raw_string_offsets() {
        // -08:00 09:00 is 17:00 UTC — later than a +00:00 10:00 checkpoint,
        // even though "09" < "10" as bare text.
        assert!(is_newer(
            "2026-08-21T09:00:00-08:00",
            "2026-08-21T10:00:00+00:00"
        ));
    }

    #[test]
    fn apply_checkpoint_advances_the_last_number_seen_per_kind() {
        let mut checking = Checking::default();
        let strip = item("2026-01-01T00:00:00Z");
        let mut rant = item("2026-01-02T00:00:00Z");
        rant.kind = FeedItemKind::Rant;
        rant.number = 42;

        apply_checkpoint(&mut checking, &[&strip, &rant]);

        assert_eq!(checking.last_strip_number, 1);
        assert_eq!(checking.last_rant_number, 42);
        assert!(checking.last_check.is_some());
    }

    #[test]
    fn apply_checkpoint_keeps_the_highest_number_when_new_items_are_newest_first() {
        // Same order the real feed provides (newest-first): the checkpoint
        // must end up at the newest strip (1619) even though it's not the
        // last item processed.
        let mut checking = Checking::default();
        let mut newest = item("2026-01-02T00:00:00Z");
        newest.number = 1619;
        let mut oldest = item("2026-01-01T00:00:00Z");
        oldest.number = 1618;

        apply_checkpoint(&mut checking, &[&newest, &oldest]);

        assert_eq!(checking.last_strip_number, 1619);
    }

    #[test]
    fn apply_checkpoint_stamps_last_check_even_with_no_new_items() {
        let mut checking = Checking::default();
        apply_checkpoint(&mut checking, &[]);
        assert!(checking.last_check.is_some());
        assert_eq!(checking.last_strip_number, 0);
        assert_eq!(checking.last_rant_number, 0);
    }

    #[tokio::test]
    async fn fetch_rants_at_parses_the_rants_off_the_fetched_page() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/strip/1619"))
            .respond_with(ResponseTemplate::new(200).set_body_string(strip_1619_fixture()))
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        let rants = fetch_rants_at(&client, &format!("{}/strip/1619", server.uri()))
            .await
            .unwrap();

        let numbers: Vec<i32> = rants.iter().map(|r| r.number).collect();
        assert_eq!(numbers, vec![1106, 1107]);
    }

    /// Direct regression test for the bug class behind #37 (a rant already
    /// in the store was still assumed unbackfilled): a number already known
    /// must not be re-fetched at all.
    #[tokio::test]
    async fn resolve_and_store_rants_skips_numbers_already_in_the_store() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/9999"))
            .respond_with(ResponseTemplate::new(200))
            .expect(0)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/1200"))
            .respond_with(ResponseTemplate::new(200).set_body_string(strip_1619_fixture()))
            .expect(1)
            .mount(&server)
            .await;

        let store = Store::open_in_memory().unwrap();
        store.upsert_rant(&sample_rant(9999)).unwrap();

        let client = reqwest::Client::new();
        resolve_and_store_rants(&client, &store, vec![9999, 1200], &server.uri()).await;

        // The already-known number was never touched...
        assert_eq!(
            store.rant_by_number(9999).unwrap().unwrap().title,
            "Placeholder"
        );
        // ...while the unknown one's page (hosting two rants) got fetched
        // and both stored.
        assert!(store.rant_by_number(1106).unwrap().is_some());
        assert!(store.rant_by_number(1107).unwrap().is_some());
    }

    #[tokio::test]
    async fn check_feed_at_backfills_new_content_and_advances_the_checkpoint() {
        let server = MockServer::start().await;
        let rant_link = format!("{}/rant-page", server.uri());

        let feed_body = feed_xml(&format!(
            "{}{}",
            feed_item_xml(
                r#"Comic [2] "Second Strip""#,
                "https://megatokyo.com/strip/2",
                "Mon, 01 Jan 2026 00:00:00 +0000",
            ),
            feed_item_xml(
                r#"Rant [1200] "Test Rant""#,
                &rant_link,
                "Mon, 01 Jan 2026 00:00:00 +0000",
            ),
        ));

        Mock::given(method("GET"))
            .and(path("/feed.xml"))
            .respond_with(ResponseTemplate::new(200).set_body_string(feed_body))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/archive.php"))
            .respond_with(ResponseTemplate::new(200).set_body_string(MINI_ARCHIVE_HTML))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/rant-page"))
            .respond_with(ResponseTemplate::new(200).set_body_string(strip_1619_fixture()))
            .mount(&server)
            .await;
        // Strip 1 (< 1081) probes .gif first — resolves. Strip 2 is left
        // unmocked (every extension 404s), exercising the "could not
        // resolve" branch of resolve_and_store_strips alongside the
        // success one.
        Mock::given(method("HEAD"))
            .and(path("/strips/0001.gif"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let store = Store::open_in_memory().unwrap();
        let client = reqwest::Client::new();

        check_feed_at(
            &client,
            &store,
            &format!("{}/feed.xml", server.uri()),
            &format!("{}/archive.php", server.uri()),
            &format!("{}/strips", server.uri()),
        )
        .await
        .unwrap();

        let checking = store.get_checking().unwrap();
        assert_eq!(checking.last_strip_number, 2);
        assert_eq!(checking.last_rant_number, 1200);
        assert!(checking.last_check.is_some());

        assert!(store.strip_by_number(1).unwrap().is_some());
        assert!(store.strip_by_number(2).unwrap().is_none());
        assert!(store.rant_by_number(1106).unwrap().is_some());
        assert!(store.rant_by_number(1107).unwrap().is_some());
    }

    #[tokio::test]
    async fn check_feed_at_skips_the_backfill_when_nothing_is_new() {
        let server = MockServer::start().await;

        let feed_body = feed_xml(&feed_item_xml(
            r#"Comic [1] "First Strip""#,
            "https://megatokyo.com/strip/1",
            "Mon, 01 Jan 2026 00:00:00 +0000",
        ));
        Mock::given(method("GET"))
            .and(path("/feed.xml"))
            .respond_with(ResponseTemplate::new(200).set_body_string(feed_body))
            .mount(&server)
            .await;
        // Must not be called: nothing in the feed is newer than the
        // checkpoint set below.
        Mock::given(method("GET"))
            .and(path("/archive.php"))
            .respond_with(ResponseTemplate::new(200).set_body_string(MINI_ARCHIVE_HTML))
            .expect(0)
            .mount(&server)
            .await;

        let store = Store::open_in_memory().unwrap();
        store
            .save_checking(&Checking {
                last_check: Some("2027-01-01T00:00:00Z".to_string()),
                last_strip_number: 0,
                last_rant_number: 0,
            })
            .unwrap();

        let client = reqwest::Client::new();
        check_feed_at(
            &client,
            &store,
            &format!("{}/feed.xml", server.uri()),
            &format!("{}/archive.php", server.uri()),
            &format!("{}/strips", server.uri()),
        )
        .await
        .unwrap();

        assert_eq!(store.get_checking().unwrap().last_strip_number, 0);
    }
}
