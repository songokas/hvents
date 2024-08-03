use std::{fs::File, io::Write, path::PathBuf};

use serde::{Deserialize, Serialize};

use super::Data;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileWriteEvent(FileWriteConfig);

impl FileWriteEvent {
    pub fn write(&self, data: &Data) -> Result<(), anyhow::Error> {
        let mut options = File::options();
        let mut truncate_options = || {
            options.write(true).truncate(true).create(true);
        };
        let file = match &self.0 {
            FileWriteConfig::File(f) => {
                truncate_options();
                f
            }
            FileWriteConfig::Config(f) => {
                match f.mode {
                    FileWriteMode::Append => {
                        options.append(true).create(true);
                    }
                    FileWriteMode::Truncate => truncate_options(),
                };
                &f.file
            }
        };
        let mut h = options.open(file)?;
        match data {
            Data::String(s) => h.write_all(s.as_bytes()).map_err(Into::into),
            Data::Bytes(s) => h.write_all(s).map_err(Into::into),
            Data::Json(v) => serde_json::to_writer(h, v).map_err(Into::into),
            Data::Empty => Ok(()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum FileWriteConfig {
    File(PathBuf),
    Config(FileWrite),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FileWrite {
    file: PathBuf,
    #[serde(default)]
    mode: FileWriteMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
enum FileWriteMode {
    Append,
    #[default]
    Truncate,
}

#[cfg(test)]
mod tests {
    use crate::events::file_read::FileReadEvent;

    use super::*;

    #[test]
    fn test_write_truncate() {
        let data = Data::String("hello".to_string());
        let json = r#""/tmp/_test_write_truncate""#;
        let event: FileWriteEvent = serde_json::from_str(json).unwrap();
        event.write(&data).unwrap();
        event.write(&data).unwrap();
        let json = r#"{"file":"/tmp/_test_write_truncate"}"#;
        let event: FileReadEvent = serde_json::from_str(json).unwrap();
        let content = event.read().unwrap();
        assert_eq!(data, content);
    }

    #[test]
    fn test_write_append() {
        let data = Data::String("hello".to_string());
        let json = r#"{"file":"/tmp/_test_write_append","mode":"truncate"}"#;
        let event: FileWriteEvent = serde_json::from_str(json).unwrap();
        event.write(&data).unwrap();
        let json = r#"{"file":"/tmp/_test_write_append","mode":"append"}"#;
        let event: FileWriteEvent = serde_json::from_str(json).unwrap();
        event.write(&data).unwrap();
        let json = r#"{"file":"/tmp/_test_write_append"}"#;
        let event: FileReadEvent = serde_json::from_str(json).unwrap();
        let content = event.read().unwrap();
        let expected = Data::String("hellohello".to_string());
        assert_eq!(expected, content);
    }
}
