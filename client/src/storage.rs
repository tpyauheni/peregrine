use std::{path::PathBuf, sync::LazyLock};

use platform_dirs::AppDirs;
use server::AccountCredentials;

use shared::storage::{GeneralStorage, RawStorage};

pub static FALLBACK_DATA_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    let mut path = PathBuf::new();
    path.push("peregrine");
    path
});

pub struct Storage {
    base_path: PathBuf,
}

impl Default for Storage {
    fn default() -> Self {
        let data_dir = AppDirs::new(Some("peregrine"), false)
            .map_or(FALLBACK_DATA_PATH.to_path_buf(), |dirs| dirs.data_dir);
        Self {
            base_path: data_dir,
        }
    }
}

macro_rules! storage_file {
    ($vis:vis [ $store_fn:ident, $load_fn:ident, $remove_fn:ident $(,)? ], $file_path:literal, $type:ty $(,)?) => {
        const FILE_PATH: &str = $file_path;

        $vis fn $store_fn(&self, data: $type) -> bool {
            self.store(&Self::FILE_PATH, &data)
        }

        $vis fn $load_fn(&self) -> Option<$type> {
            self.load(&Self::FILE_PATH)
        }

        $vis fn $remove_fn(&self) -> bool {
            self.remove(&Self::FILE_PATH)
        }
    };
}

impl RawStorage for Storage {
    fn get_base_path(&self) -> &PathBuf {
        &self.base_path
    }
}

impl GeneralStorage for Storage {}

impl Storage {
    pub const SESSION_CREDENTIALS_FILE: &str = "session.bin";

    storage_file!(
        pub [
            store_session_credentials,
            load_session_credentials,
            remove_session_credentials,
        ],
        "session.bin",
        AccountCredentials,
    );
}

pub static STORAGE: LazyLock<Storage> = LazyLock::new(Default::default);
