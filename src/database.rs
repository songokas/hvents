use std::fs::create_dir_all;

use serde::{de::DeserializeOwned, Serialize};

pub trait KeyValueStore {
    fn insert<T: Serialize>(&self, key: &str, data: &T) -> Result<(), anyhow::Error>;
    fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T>;
    fn remove(&self, key: &str) -> bool;
}

pub fn init(uri: Option<&str>) -> impl KeyValueStore {
    if let Some(u) = uri {
        create_dir_all(u).unwrap_or_else(|e| panic!("Unable to create directory {u} {e}"));
        return Store::Dir(filesystem::FileSystem {
            directory: u.to_string(),
        });
    }
    Store::Null
}

pub enum Store {
    Dir(filesystem::FileSystem),
    Null,
}

impl KeyValueStore for Store {
    fn insert<T: Serialize>(&self, key: &str, data: &T) -> Result<(), anyhow::Error> {
        match self {
            Store::Dir(f) => f.insert(key, data),
            Store::Null => Ok(()),
        }
    }

    fn get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        match self {
            Store::Dir(f) => f.get(key),
            Store::Null => None,
        }
    }

    fn remove(&self, key: &str) -> bool {
        match self {
            Store::Dir(f) => f.remove(key),
            Store::Null => false,
        }
    }
}

mod filesystem {
    use std::{
        fs::{remove_file, File},
        path::Path,
    };

    use anyhow::Context;
    use serde::{de::DeserializeOwned, Serialize};

    use super::KeyValueStore;

    pub struct FileSystem {
        pub directory: String,
    }

    impl KeyValueStore for FileSystem {
        fn insert<T: Serialize>(&self, key: &str, data: &T) -> Result<(), anyhow::Error> {
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

        fn remove(&self, key: &str) -> bool {
            let path = Path::new(&self.directory).join(key);
            remove_file(path).is_ok()
        }
    }
}
