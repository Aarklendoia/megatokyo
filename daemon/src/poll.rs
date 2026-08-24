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

use megatokyo_core::scraper::{self, strips::UnresolvedStrip};
use megatokyo_core::{feed, store::Store};

use crate::control::AppState;

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
    let archive_html = client
        .get(scraper::ARCHIVE_URL)
        .send()
        .await?
        .text()
        .await?;

    for chapter in scraper::chapters::parse(&archive_html) {
        store.upsert_chapter(&chapter)?;
    }

    let unresolved = scraper::strips::parse(&archive_html);
    log::info!("backfilling {} strips", unresolved.len());
    resolve_and_store_strips(client, store, unresolved).await;
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
async fn resolve_and_store_strips(
    client: &reqwest::Client,
    store: &Store,
    unresolved: Vec<UnresolvedStrip>,
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
        tasks.spawn(async move {
            let _permit = semaphore.acquire_owned().await;
            let number = strip.number;
            (number, scraper::strips::resolve(&client, strip).await)
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
    let html = client
        .get(scraper::rant_archive::RANT_ARCHIVE_URL)
        .send()
        .await?
        .text()
        .await?;
    let numbers = scraper::rant_archive::parse_numbers(&html);
    resolve_and_store_rants(client, store, numbers).await;
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
async fn resolve_and_store_rants(client: &reqwest::Client, store: &Store, numbers: Vec<i32>) {
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
        tasks.spawn(async move {
            let _permit = semaphore.acquire_owned().await;
            let link = format!("https://megatokyo.com/rant/{number}");
            (number, fetch_rants_at(&client, &link).await)
        });
    }

    while let Some(result) = tasks.join_next().await {
        match result {
            Ok((_, Ok(rants))) => {
                for rant in rants {
                    if let Err(err) = store.upsert_rant(&rant) {
                        log::warn!("could not store rant {}: {err}", rant.number);
                    }
                }
            }
            Ok((number, Err(err))) => log::warn!("could not fetch rant {number}: {err}"),
            Err(join_err) => log::warn!("rant resolve task failed: {join_err}"),
        }
    }
}

/// Diffs the RSS feed against the stored `checking` checkpoint, backfills
/// any strip/rant newer than the checkpoint, and advances it.
async fn check_feed(
    client: &reqwest::Client,
    store: &Store,
) -> Result<(), Box<dyn std::error::Error>> {
    let items = feed::fetch(client).await?;
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
        if let Err(err) = backfill(client, store).await {
            log::warn!("could not rescrape the archive after a feed change: {err}");
        }
        for item in &new_items {
            if item.kind == feed::FeedItemKind::Rant {
                match fetch_rants_at(client, &item.link).await {
                    Ok(rants) => {
                        for rant in rants {
                            if let Err(err) = store.upsert_rant(&rant) {
                                log::warn!("could not store rant {}: {err}", rant.number);
                            }
                        }
                    }
                    Err(err) => log::warn!(
                        "could not fetch rant {} at {}: {err}",
                        item.number,
                        item.link
                    ),
                }
            }
        }
    }

    for item in &new_items {
        match item.kind {
            feed::FeedItemKind::Strip => checking.last_strip_number = item.number,
            feed::FeedItemKind::Rant => checking.last_rant_number = item.number,
        }
    }
    checking.last_check = Some(chrono::Utc::now().to_rfc3339());
    store.save_checking(&checking)?;
    Ok(())
}

/// Rants aren't addressable by their own page: `https://megatokyo.com/rant/<n>`
/// (a feed item's `link`, or a number from `scraper::rant_archive`) 301-
/// redirects to whichever strip page actually hosts it (verified live —
/// see `feed::FeedItem::link`'s doc comment). `reqwest::Client` follows
/// redirects by default, so fetching `link` directly lands on the right
/// strip page without this daemon ever having to work out which strip
/// that is itself. Fetch-only (no store access) so callers can run this
/// concurrently across many links, see [`resolve_and_store_rants`].
async fn fetch_rants_at(
    client: &reqwest::Client,
    link: &str,
) -> Result<Vec<megatokyo_core::domain::Rant>, reqwest::Error> {
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

    fn item(published_at: &str) -> FeedItem {
        FeedItem {
            kind: FeedItemKind::Strip,
            number: 1,
            title: "Sample".to_string(),
            published_at: published_at.to_string(),
            link: "https://megatokyo.com/strip/1".to_string(),
        }
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
}
