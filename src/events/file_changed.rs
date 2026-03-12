use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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

#[derive(
    Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Copy, EnumString, Display,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum WatchKind {
    Written,
    #[default]
    Created,
    Removed,
}
