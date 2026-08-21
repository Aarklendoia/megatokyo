//! SQLite-backed storage for chapters, strips, rants, cached translations and
//! the "last check" bookkeeping the poll loop uses to detect new content.
//!
//! One `rusqlite::Connection` behind a `Mutex`: the daemon's HTTP handlers
//! run one-thread-per-connection (see `daemon::control`) and the poll loop
//! runs concurrently, but the expected load (a handful of personal/family
//! clients, see the plan's "Déploiement distant") never justifies a
//! connection pool — SQLite itself serializes writers regardless, and a
//! single `Mutex` keeps that explicit rather than hidden behind pool
//! contention.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rusqlite::{params, Connection, OptionalExtension};

use crate::domain::{Chapter, Checking, Rant, Strip};

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
}

pub type Result<T> = std::result::Result<T, StoreError>;

pub struct Store {
    conn: Mutex<Connection>,
}

impl Store {
    /// `$XDG_DATA_HOME/megatokyo/megatokyo.db`, falling back to
    /// `~/.local/share/megatokyo/megatokyo.db` per the XDG base dir spec.
    pub fn default_db_path() -> PathBuf {
        let data_home = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/share")
            });
        data_home.join("megatokyo").join("megatokyo.db")
    }

    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let conn = Connection::open(path)?;
        Self::migrate(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// In-memory database — used by tests only, never by the daemon binary.
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        Self::migrate(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn migrate(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS chapters (
                category TEXT PRIMARY KEY,
                number INTEGER NOT NULL,
                title TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS strips (
                number INTEGER PRIMARY KEY,
                category TEXT NOT NULL,
                title TEXT NOT NULL,
                url TEXT NOT NULL,
                publish_date TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_strips_category ON strips(category);
            CREATE TABLE IF NOT EXISTS rants (
                number INTEGER PRIMARY KEY,
                author TEXT NOT NULL,
                title TEXT NOT NULL,
                url TEXT NOT NULL,
                publish_date TEXT NOT NULL,
                content TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS translations (
                rant_number INTEGER NOT NULL,
                lang TEXT NOT NULL,
                content TEXT NOT NULL,
                PRIMARY KEY (rant_number, lang)
            );
            CREATE TABLE IF NOT EXISTS checking (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                last_check TEXT,
                last_strip_number INTEGER NOT NULL DEFAULT 0,
                last_rant_number INTEGER NOT NULL DEFAULT 0
            );
            ",
        )?;
        Ok(())
    }

    // -- chapters ---------------------------------------------------------

    pub fn upsert_chapter(&self, chapter: &Chapter) -> Result<()> {
        self.conn.lock().unwrap().execute(
            "INSERT INTO chapters (category, number, title) VALUES (?1, ?2, ?3)
             ON CONFLICT(category) DO UPDATE SET number = excluded.number, title = excluded.title",
            params![chapter.category, chapter.number, chapter.title],
        )?;
        Ok(())
    }

    pub fn all_chapters(&self) -> Result<Vec<Chapter>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT category, number, title FROM chapters ORDER BY number")?;
        let chapters = stmt
            .query_map([], |row| {
                Ok(Chapter {
                    category: row.get(0)?,
                    number: row.get(1)?,
                    title: row.get(2)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(chapters)
    }

    pub fn chapter_by_category(&self, category: &str) -> Result<Option<Chapter>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT category, number, title FROM chapters WHERE category = ?1",
            params![category],
            |row| {
                Ok(Chapter {
                    category: row.get(0)?,
                    number: row.get(1)?,
                    title: row.get(2)?,
                })
            },
        )
        .optional()
        .map_err(StoreError::from)
    }

    // -- strips -------------------------------------------------------------

    pub fn upsert_strip(&self, strip: &Strip) -> Result<()> {
        self.conn.lock().unwrap().execute(
            "INSERT INTO strips (number, category, title, url, publish_date)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(number) DO UPDATE SET
                category = excluded.category, title = excluded.title,
                url = excluded.url, publish_date = excluded.publish_date",
            params![
                strip.number,
                strip.category,
                strip.title,
                strip.url,
                strip.publish_date
            ],
        )?;
        Ok(())
    }

    pub fn all_strips(&self) -> Result<Vec<Strip>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT number, category, title, url, publish_date FROM strips ORDER BY number",
        )?;
        let strips = stmt
            .query_map([], Self::row_to_strip)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(strips)
    }

    pub fn strips_by_category(&self, category: &str) -> Result<Vec<Strip>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT number, category, title, url, publish_date FROM strips
             WHERE category = ?1 ORDER BY number",
        )?;
        let strips = stmt
            .query_map(params![category], Self::row_to_strip)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(strips)
    }

    pub fn strip_by_number(&self, number: i32) -> Result<Option<Strip>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT number, category, title, url, publish_date FROM strips WHERE number = ?1",
            params![number],
            Self::row_to_strip,
        )
        .optional()
        .map_err(StoreError::from)
    }

    pub fn has_any_strip(&self) -> Result<bool> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM strips", [], |row| row.get(0))?;
        Ok(count > 0)
    }

    fn row_to_strip(row: &rusqlite::Row) -> rusqlite::Result<Strip> {
        Ok(Strip {
            number: row.get(0)?,
            category: row.get(1)?,
            title: row.get(2)?,
            url: row.get(3)?,
            publish_date: row.get(4)?,
        })
    }

    // -- rants ----------------------------------------------------------

    pub fn upsert_rant(&self, rant: &Rant) -> Result<()> {
        self.conn.lock().unwrap().execute(
            "INSERT INTO rants (number, author, title, url, publish_date, content)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(number) DO UPDATE SET
                author = excluded.author, title = excluded.title, url = excluded.url,
                publish_date = excluded.publish_date, content = excluded.content",
            params![
                rant.number,
                rant.author,
                rant.title,
                rant.url,
                rant.publish_date,
                rant.content
            ],
        )?;
        Ok(())
    }

    pub fn all_rants(&self) -> Result<Vec<Rant>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT number, author, title, url, publish_date, content FROM rants ORDER BY number DESC",
        )?;
        let rants = stmt
            .query_map([], Self::row_to_rant)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rants)
    }

    pub fn rant_by_number(&self, number: i32) -> Result<Option<Rant>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT number, author, title, url, publish_date, content FROM rants WHERE number = ?1",
            params![number],
            Self::row_to_rant,
        )
        .optional()
        .map_err(StoreError::from)
    }

    fn row_to_rant(row: &rusqlite::Row) -> rusqlite::Result<Rant> {
        Ok(Rant {
            number: row.get(0)?,
            author: row.get(1)?,
            title: row.get(2)?,
            url: row.get(3)?,
            publish_date: row.get(4)?,
            content: row.get(5)?,
        })
    }

    // -- translations -----------------------------------------------------

    pub fn get_translation(&self, rant_number: i32, lang: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT content FROM translations WHERE rant_number = ?1 AND lang = ?2",
            params![rant_number, lang],
            |row| row.get(0),
        )
        .optional()
        .map_err(StoreError::from)
    }

    pub fn save_translation(&self, rant_number: i32, lang: &str, content: &str) -> Result<()> {
        self.conn.lock().unwrap().execute(
            "INSERT INTO translations (rant_number, lang, content) VALUES (?1, ?2, ?3)
             ON CONFLICT(rant_number, lang) DO UPDATE SET content = excluded.content",
            params![rant_number, lang, content],
        )?;
        Ok(())
    }

    // -- checking -----------------------------------------------------------

    pub fn get_checking(&self) -> Result<Checking> {
        let conn = self.conn.lock().unwrap();
        let checking = conn
            .query_row(
                "SELECT last_check, last_strip_number, last_rant_number FROM checking WHERE id = 1",
                [],
                |row| {
                    Ok(Checking {
                        last_check: row.get(0)?,
                        last_strip_number: row.get(1)?,
                        last_rant_number: row.get(2)?,
                    })
                },
            )
            .optional()?;
        Ok(checking.unwrap_or_default())
    }

    pub fn save_checking(&self, checking: &Checking) -> Result<()> {
        self.conn.lock().unwrap().execute(
            "INSERT INTO checking (id, last_check, last_strip_number, last_rant_number)
             VALUES (1, ?1, ?2, ?3)
             ON CONFLICT(id) DO UPDATE SET
                last_check = excluded.last_check,
                last_strip_number = excluded.last_strip_number,
                last_rant_number = excluded.last_rant_number",
            params![
                checking.last_check,
                checking.last_strip_number,
                checking.last_rant_number
            ],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_chapter() -> Chapter {
        Chapter {
            number: 13,
            category: "redemption".to_string(),
            title: "Redemption".to_string(),
        }
    }

    fn sample_strip() -> Strip {
        Strip {
            number: 1619,
            category: "redemption".to_string(),
            title: "Beautiful".to_string(),
            url: "https://megatokyo.com/strips/1619.png".to_string(),
            publish_date: "2025-12-28T00:00:00Z".to_string(),
        }
    }

    fn sample_rant() -> Rant {
        Rant {
            number: 1106,
            author: "Fred".to_string(),
            title: "A rant".to_string(),
            url: "https://megatokyo.com/rantimages/1106.jpg".to_string(),
            publish_date: "2025-12-28T00:00:00Z".to_string(),
            content: "<p>hello</p>".to_string(),
        }
    }

    #[test]
    fn upserts_and_reads_back_a_chapter() {
        let store = Store::open_in_memory().unwrap();
        store.upsert_chapter(&sample_chapter()).unwrap();
        assert_eq!(store.all_chapters().unwrap(), vec![sample_chapter()]);
        assert_eq!(
            store.chapter_by_category("redemption").unwrap(),
            Some(sample_chapter())
        );
        assert_eq!(store.chapter_by_category("nope").unwrap(), None);
    }

    #[test]
    fn upsert_chapter_updates_in_place_rather_than_duplicating() {
        let store = Store::open_in_memory().unwrap();
        store.upsert_chapter(&sample_chapter()).unwrap();
        let mut renamed = sample_chapter();
        renamed.title = "Redemption (renamed)".to_string();
        store.upsert_chapter(&renamed).unwrap();
        assert_eq!(store.all_chapters().unwrap(), vec![renamed]);
    }

    #[test]
    fn upserts_and_reads_back_a_strip() {
        let store = Store::open_in_memory().unwrap();
        assert!(!store.has_any_strip().unwrap());
        store.upsert_strip(&sample_strip()).unwrap();
        assert!(store.has_any_strip().unwrap());
        assert_eq!(store.strip_by_number(1619).unwrap(), Some(sample_strip()));
        assert_eq!(
            store.strips_by_category("redemption").unwrap(),
            vec![sample_strip()]
        );
        assert_eq!(store.strip_by_number(9999).unwrap(), None);
    }

    #[test]
    fn upserts_and_reads_back_a_rant() {
        let store = Store::open_in_memory().unwrap();
        store.upsert_rant(&sample_rant()).unwrap();
        assert_eq!(store.rant_by_number(1106).unwrap(), Some(sample_rant()));
        assert_eq!(store.all_rants().unwrap(), vec![sample_rant()]);
    }

    #[test]
    fn translation_cache_round_trips_per_language() {
        let store = Store::open_in_memory().unwrap();
        store.upsert_rant(&sample_rant()).unwrap();
        assert_eq!(store.get_translation(1106, "fr").unwrap(), None);
        store.save_translation(1106, "fr", "<p>bonjour</p>").unwrap();
        assert_eq!(
            store.get_translation(1106, "fr").unwrap(),
            Some("<p>bonjour</p>".to_string())
        );
        assert_eq!(store.get_translation(1106, "de").unwrap(), None);
    }

    #[test]
    fn checking_defaults_to_zero_then_persists_updates() {
        let store = Store::open_in_memory().unwrap();
        assert_eq!(store.get_checking().unwrap(), Checking::default());
        let updated = Checking {
            last_check: Some("2026-08-21T00:00:00Z".to_string()),
            last_strip_number: 1619,
            last_rant_number: 1106,
        };
        store.save_checking(&updated).unwrap();
        assert_eq!(store.get_checking().unwrap(), updated);
    }
}
