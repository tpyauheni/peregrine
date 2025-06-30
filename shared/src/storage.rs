use std::{error::Error, fmt::Debug, fs::{self, File}, io::{Read, Write}, path::{Path, PathBuf}};

use atomic_write_file::AtomicWriteFile;
use postcard::{from_bytes, to_allocvec};
use serde::{de::DeserializeOwned, Serialize};

pub trait RawStorage {
    fn get_base_path(&self) -> &PathBuf;

    fn get_path<P: AsRef<Path>>(&self, original_path: P) -> Result<PathBuf, Box<dyn Error>> {
        let path = self.get_base_path().join(original_path);
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
}

pub trait GeneralStorage : RawStorage {
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
}
