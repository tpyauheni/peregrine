use std::{
    error::Error,
    fmt::Debug,
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::LazyLock,
};

use atomic_write_file::AtomicWriteFile;
use platform_dirs::AppDirs;
use postcard::{from_bytes, to_allocvec};
use serde::{Serialize, de::DeserializeOwned};
use server::AccountCredentials;

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

impl Storage {
    pub const SESSION_CREDENTIALS_FILE: &str = "session.bin";

    fn get_path<P: AsRef<Path>>(&self, original_path: P) -> Result<PathBuf, Box<dyn Error>> {
        let path = self.base_path.join(original_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(path.canonicalize().unwrap_or(path))
    }

    fn raw_store<P: AsRef<Path>>(
        &self,
        file_path: P,
        data: &impl Serialize,
    ) -> Result<(), Box<dyn Error>> {
        let path = self.get_path(file_path)?;
        println!("Storing data to file {:?}", path.as_path());
        let bytes = to_allocvec(data)?;
        let mut file = AtomicWriteFile::options().open(path)?;
        file.write_all(&bytes)?;
        file.commit()?;
        Ok(())
    }

    fn raw_load<P: AsRef<Path>, T: DeserializeOwned>(
        &self,
        file_path: P,
    ) -> Result<T, Box<dyn Error>> {
        let path = self.get_path(file_path)?;
        println!("Loading data from file {:?}", path.as_path());
        let mut bytes: Vec<u8> = vec![];
        File::options()
            .read(true)
            .open(path)?
            .read_to_end(&mut bytes)?;
        let data = from_bytes(&bytes)?;
        Ok(data)
    }

    fn raw_remove<P: AsRef<Path>>(&self, file_path: P) -> Result<(), Box<dyn Error>> {
        let path = self.get_path(file_path)?;
        Ok(std::fs::remove_file(path)?)
    }

    fn store<P: AsRef<Path> + Debug>(&self, file_path: &P, data: &impl Serialize) -> bool {
        if let Err(err) = self.raw_store(file_path, data) {
            eprintln!("Unexpected error while trying to store data to file {file_path:?}: {err:?}");
            false
        } else {
            true
        }
    }

    fn load<P: AsRef<Path> + Debug, T: DeserializeOwned>(&self, file_path: &P) -> Option<T> {
        match self.raw_load(file_path) {
            Ok(data) => Some(data),
            Err(err) => {
                eprintln!(
                    "Unexpected error while trying to load data from file {file_path:?}: {err:?}"
                );
                None
            }
        }
    }

    fn remove<P: AsRef<Path> + Debug>(&self, file_path: &P) -> bool {
        if let Err(err) = self.raw_remove(file_path) {
            eprintln!("Unexpected error while trying to remove file {file_path:?}: {err:?}");
            false
        } else {
            true
        }
    }

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
