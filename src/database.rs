use std::fs::create_dir_all;

use indexmap::IndexMap;
use serde::{Serialize, de::DeserializeOwned};

pub trait KeyValueStore {
    fn insert<T: Serialize>(&mut self, key: &str, data: &T) -> Result<(), anyhow::Error>;
    fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T>;
    fn remove(&mut self, key: &str) -> bool;
}

pub fn init(uri: Option<&str>) -> Store {
    if let Some(u) = uri {
        create_dir_all(u).unwrap_or_else(|e| panic!("Unable to create directory {u} {e}"));
        return Store::Dir(filesystem::FileSystem {
            directory: u.to_string(),
        });
    }
    Store::Memory(IndexMap::new())
}

#[derive(Clone)]
pub enum Store {
    Dir(filesystem::FileSystem),
    Memory(IndexMap<String, String>),
}

impl Store {
    pub fn in_memory() -> Self {
        Self::Memory(Default::default())
    }
}

impl KeyValueStore for Store {
    fn insert<T: Serialize>(&mut self, key: &str, data: &T) -> Result<(), anyhow::Error> {
        match self {
            Store::Dir(f) => f.insert(key, data),
            Store::Memory(f) => {
                f.insert(
                    key.to_string(),
                    serde_json::to_string(data).expect("Unable to insert in memory"),
                );
                Ok(())
            }
        }
    }

    fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        match self {
            Store::Dir(f) => f.get(key),
            Store::Memory(f) => f.get(key).map(|s| serde_json::from_str(s).unwrap()),
        }
    }

    fn remove(&mut self, key: &str) -> bool {
        match self {
            Store::Dir(f) => f.remove(key),
            Store::Memory(f) => f.shift_remove(key).is_some(),
        }
    }
}

mod filesystem {
    use std::{
        fs::{File, remove_file},
        path::{Path, PathBuf},
    };

    use anyhow::Context;
    use serde::{Serialize, de::DeserializeOwned};

    use super::KeyValueStore;

    #[derive(Clone)]
    pub struct FileSystem {
        pub directory: String,
    }

    impl FileSystem {
        /// Escapes the key so it always maps to a single file inside the directory
        fn path(&self, key: &str) -> PathBuf {
            let file_name = key
                .replace('%', "%25")
                .replace('/', "%2F")
                .replace('\\', "%5C");
            Path::new(&self.directory).join(file_name)
        }
    }

    impl KeyValueStore for FileSystem {
        fn insert<T: Serialize>(&mut self, key: &str, data: &T) -> Result<(), anyhow::Error> {
            let path = self.path(key);
            let file = File::options()
                .create(true)
                .truncate(true)
                .write(true)
                .open(&path)
                .with_context(|| format!("Can not write file {}", path.display()))?;
            serde_json::to_writer(file, data)?;
            Ok(())
        }

        fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
            let path = self.path(key);
            let file = File::options().read(true).open(&path).ok()?;
            serde_json::from_reader(file).ok()
        }

        fn remove(&mut self, key: &str) -> bool {
            remove_file(self.path(key)).is_ok()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filesystem_key_with_slash() {
        let dir = std::env::temp_dir().join("hvents_test_filesystem_key_with_slash");
        let mut store = init(dir.to_str());
        let key = "run every a/b \\ 100%";
        store.insert(key, &"data").unwrap();
        assert_eq!(store.get::<String>(key).as_deref(), Some("data"));
        assert!(store.remove(key));
        assert_eq!(store.get::<String>(key), None);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
