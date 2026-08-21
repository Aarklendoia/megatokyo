//! Scrapes `megatokyo.com` for chapters, strips and rants. Split into one
//! submodule per concern, mirroring the original .NET `StripsParser`/
//! `ChaptersParser`/`RantsParser` — but built against the site's *current*
//! markup, re-verified live rather than assumed to still match the ~2022
//! original (it has drifted in small ways, documented in each submodule).

pub mod chapters;
pub mod date;
pub mod rants;
pub mod strips;

pub const ARCHIVE_URL: &str = "https://megatokyo.com/archive.php";
