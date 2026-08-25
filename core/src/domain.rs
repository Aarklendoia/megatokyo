use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chapter {
    pub number: i32,
    pub category: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Strip {
    pub number: i32,
    pub category: String,
    pub title: String,
    pub url: String,
    pub publish_date: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rant {
    pub number: i32,
    pub author: String,
    pub title: String,
    pub url: String,
    pub publish_date: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Checking {
    pub last_check: Option<String>,
    pub last_strip_number: i32,
    pub last_rant_number: i32,
}

/// A strip number the user starred, in `favorites`. There is no user
/// concept in the store — a daemon instance has one shared favorites list,
/// same as it has one shared `Checking` (see `store::Store`'s doc comment).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Favorite {
    pub strip_number: i32,
    pub added_at: String,
}

/// A browser's Web Push subscription (one per PWA install). Unlike
/// `Favorite`/`Checking`, this is inherently multi-row even though the
/// daemon still has no user concept: one shared API token fronts N distinct
/// per-browser subscriptions, keyed by `endpoint`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PushSubscription {
    pub endpoint: String,
    pub p256dh: String,
    pub auth: String,
    pub created_at: String,
}
