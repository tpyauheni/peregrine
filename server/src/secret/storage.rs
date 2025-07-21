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

impl ServerStorage {
    pub fn store_dm_file(&self, message_id: u64, data: &[u8]) {
        self.store(&format!("dm_file{message_id}.bin"), &data);
    }

    pub fn store_group_file(&self, message_id: u64, data: &[u8]) {
        self.store(&format!("group_file{message_id}.bin"), &data);
    }

    pub fn load_dm_file(&self, message_id: u64) -> Option<Box<[u8]>> {
        self.load(&format!("dm_file{message_id}.bin"))
    }

    pub fn load_group_file(&self, message_id: u64) -> Option<Box<[u8]>> {
        self.load(&format!("group_file{message_id}.bin"))
    }
}

pub static STORAGE: LazyLock<ServerStorage> = LazyLock::new(Default::default);
