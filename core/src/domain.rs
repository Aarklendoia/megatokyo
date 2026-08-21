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
