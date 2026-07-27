//! Domain types for the data layer.

// schema modules: fields are deserialized, garde-validated and ts-rs-exported,
// but not all are read by Rust yet (overlay merge + game runtime pending)
// TODO: remove these as soon as all fields are used
#[allow(dead_code)]
mod common;
#[allow(dead_code)]
mod game;
mod media;
#[allow(dead_code)]
mod overlay;
#[allow(dead_code)]
mod pack;
#[allow(dead_code)]
mod question;
#[allow(dead_code)]
mod tag;

pub use common::*;
pub use game::*;
pub use media::*;
pub use overlay::*;
pub use pack::*;
pub use question::*;
pub use tag::*;

use std::collections::HashMap;

/// One item with its source file path.
#[derive(Debug, Clone)]
pub struct Entry<T> {
    pub file: String,
    pub item: T,
}

/// Registry of items keyed by their string ID.
pub type Registry<T> = HashMap<String, Entry<T>>;

/// Per-locale translation overlays.
#[derive(Debug, Clone, Default)]
pub struct LocaleOverlays {
    pub questions: Registry<QuestionOverlay>,
    pub packs: Registry<PackOverlay>,
    pub tags: Registry<TagOverlay>,
    pub games: Registry<GameConfigOverlay>,
}

pub type Overlays = HashMap<String, LocaleOverlays>;

/// The full loaded dataset, ready for cross-file checks and querying.
#[derive(Debug, Clone)]
pub struct Dataset {
    pub data_dir: String,
    pub questions: Registry<Question>,
    pub packs: Registry<Pack>,
    pub tags: Registry<Tag>,
    pub overlays: Overlays,
    pub games: Registry<GameConfig>,
    pub issues: Vec<LoadIssue>,
}

/// Non-fatal diagnostic from the data loader.
#[derive(Debug, Clone)]
pub struct LoadIssue {
    pub file: String,
    pub message: String,
    pub path: Option<String>,
}

impl LoadIssue {
    pub fn msg(file: &str, message: String) -> Self {
        Self {
            file: file.to_owned(),
            message,
            path: None,
        }
    }
}

impl std::fmt::Display for LoadIssue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.path {
            Some(p) => write!(f, "{} at {}: {}", self.file, p, self.message),
            None => write!(f, "{}: {}", self.file, self.message),
        }
    }
}
