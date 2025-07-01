use std::{path::PathBuf, sync::LazyLock};

use shared::storage::{GeneralStorage, RawStorage};

pub static STORAGE_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    let mut path = PathBuf::new();
    path.push("peregrine_server");
    path
});

pub struct ServerStorage {
    base_path: PathBuf,
}

impl Default for ServerStorage {
    fn default() -> Self {
        Self {
            base_path: STORAGE_PATH.to_path_buf(),
        }
    }
}

impl RawStorage for ServerStorage {
    fn get_base_path(&self) -> &PathBuf {
        &self.base_path
    }
}

impl GeneralStorage for ServerStorage {}

pub static STORAGE: LazyLock<ServerStorage> = LazyLock::new(Default::default);
