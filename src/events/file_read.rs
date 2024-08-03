use std::{fs::File, path::PathBuf};

use serde::{Deserialize, Serialize};

use super::data::{Data, DataType};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileReadEvent(FileReadConfig);

impl FileReadEvent {
    pub fn read(&self) -> Result<Data, anyhow::Error> {
        let (file, data_type) = match &self.0 {
            FileReadConfig::File(f) => (f, DataType::default()),
            FileReadConfig::Config(f) => (&f.file, f.data_type),
        };
        let h = File::open(file)?;
        Data::from_reader(h, data_type)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum FileReadConfig {
    File(PathBuf),
    Config(FileRead),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FileRead {
    file: PathBuf,
    #[serde(default)]
    data_type: DataType,
}
