use std::{fs::File, path::PathBuf};

use serde::{Deserialize, Serialize};

use super::data::{Data, DataType, Metadata};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileReadEvent {
    pub file: PathBuf,
    #[serde(default)]
    pub data_type: DataType,
}

impl FileReadEvent {
    pub fn read(&self) -> Result<(Data, Metadata), anyhow::Error> {
        let h = File::open(&self.file)?;
        Ok((Data::from_reader(h, self.data_type)?, Metadata::default()))
    }
}
