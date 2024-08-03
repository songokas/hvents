use core::fmt::Display;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChangedEvent {
    pub path: PathBuf,
    #[serde(default)]
    pub when: WatchKind,
}

impl FileChangedEvent {
    pub fn matches(&self, path: &Path, kind: WatchKind) -> bool {
        self.path == path && self.when == kind
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Copy)]
#[serde(rename_all = "snake_case")]
pub enum WatchKind {
    Written,
    #[default]
    Created,
    Removed,
}

impl Display for WatchKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WatchKind::Written => write!(f, "written"),
            WatchKind::Created => write!(f, "created"),
            WatchKind::Removed => write!(f, "removed"),
        }
    }
}
