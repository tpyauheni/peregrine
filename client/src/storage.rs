use std::{path::PathBuf, sync::LazyLock};

use platform_dirs::AppDirs;
use server::AccountCredentials;

use shared::{
    crypto::{
        CryptoAlgorithms,
        x3dh::{self, X3DhReceiverKeysPrivate, X3DhReceiverKeysPublic},
    },
    storage::{GeneralStorage, RawStorage},
};

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
    ($vis:vis [ $store_fn:ident, $load_fn:ident, $remove_fn:ident $(,)? ], $file_path:expr, $type:ty, [ $($arg_name:ident : $arg_type:ty),* ] $(,)?) => {
        $vis fn $store_fn(&self, $($arg_name: $arg_type,)* data: $type) -> bool {
            self.store(&$file_path, &data)
        }

        $vis fn $load_fn(&self, $($arg_name: $arg_type),*) -> Option<$type> {
            self.load(&$file_path)
        }

        $vis fn $remove_fn(&self, $($arg_name: $arg_type),*) -> bool {
            self.remove(&$file_path)
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
    storage_file!(
        pub [
            store_session_credentials,
            load_session_credentials,
            remove_session_credentials,
        ],
        "session.bin",
        AccountCredentials,
        [],
    );
    storage_file!(
        pub [
            store_x3dh_data,
            load_x3dh_data,
            remove_x3dh_data,
        ],
        format!("cryptoidentity_{algorithms}.bin"),
        (X3DhReceiverKeysPrivate, X3DhReceiverKeysPublic),
        [algorithms: &CryptoAlgorithms],
    );
    storage_file!(
        pub [
            store_dm_key_box,
            load_dm_key,
            remove_dm_key,
        ],
        format!("dm{other_user_id}.bin"),
        (CryptoAlgorithms, Box<[u8]>),
        [other_user_id: u64],
    );
    storage_file!(
        pub [
            store_group_key_box,
            load_group_key,
            remove_group_key,
        ],
        format!("group{group_id}.bin"),
        (CryptoAlgorithms, Box<[u8]>),
        [group_id: u64],
    );

    pub fn x3dh_data(
        &self,
        algorithms: &CryptoAlgorithms,
    ) -> (X3DhReceiverKeysPrivate, X3DhReceiverKeysPublic) {
        if let Some(data) = self.load_x3dh_data(algorithms) {
            data
        } else {
            let data = x3dh::generate_receiver_keys(algorithms).unwrap();
            self.store_x3dh_data(algorithms, data.clone());
            data
        }
    }

    pub fn store_dm_key(&self, other_user_id: u64, data: (CryptoAlgorithms, &[u8])) -> bool {
        self.store_dm_key_box(other_user_id, (data.0, Box::from(data.1)))
    }

    pub fn store_group_key(&self, group_id: u64, data: (CryptoAlgorithms, &[u8])) -> bool {
        self.store_group_key_box(group_id, (data.0, Box::from(data.1)))
    }
}

pub static STORAGE: LazyLock<Storage> = LazyLock::new(Default::default);
