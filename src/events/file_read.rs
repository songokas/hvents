use std::{fs::File, path::PathBuf};

use serde::{Deserialize, Serialize};

use super::data::{Data, DataType};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileReadEvent {
    pub file: PathBuf,
    #[serde(default)]
    pub data_type: DataType,
}

impl FileReadEvent {
    pub fn read(&self) -> Result<Data, anyhow::Error> {
        let h = File::open(&self.file)?;
        Data::from_reader(h, self.data_type)
    }
}
