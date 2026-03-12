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
        path::Path,
    };

    use anyhow::Context;
    use serde::{Serialize, de::DeserializeOwned};

    use super::KeyValueStore;

    #[derive(Clone)]
    pub struct FileSystem {
        pub directory: String,
    }

    impl KeyValueStore for FileSystem {
        fn insert<T: Serialize>(&mut self, key: &str, data: &T) -> Result<(), anyhow::Error> {
            let path = Path::new(&self.directory).join(key);
            let file = File::options()
                .create(true)
                .truncate(true)
                .write(true)
                .open(&path)
                .with_context(|| "Can not write file {path}")?;
            serde_json::to_writer(file, data)?;
            Ok(())
        }

        fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
            let path = Path::new(&self.directory).join(key);
            let file = File::options().read(true).open(&path).ok()?;
            serde_json::from_reader(file).ok()
        }

        fn remove(&mut self, key: &str) -> bool {
            let path = Path::new(&self.directory).join(key);
            remove_file(path).is_ok()
        }
    }
}
