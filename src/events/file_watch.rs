use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WatchEvent {
    pub path: PathBuf,
    #[serde(default)]
    pub action: WatchAction,
    #[serde(default)]
    pub recursive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum WatchAction {
    #[default]
    Start,
    Stop,
}
