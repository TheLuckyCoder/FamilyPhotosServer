use std::fs;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

#[derive(Clone)]
pub struct FileStorage {
    base_path: PathBuf,
}

impl FileStorage {
    pub fn new(path: String) -> FileStorage {
        let base_path = Path::new(path.as_str()).to_owned();

        if !base_path.exists() {
            fs::create_dir_all(base_path.as_path()).expect("Could not create the base storage path");
        } else {
            assert!(base_path.is_dir());
        }

        FileStorage {
            base_path
        }
    }

    pub fn resolve<P: AsRef<Path>>(&self, path: P) -> PathBuf {
        let mut new_path = self.base_path.clone();
        new_path.push(path);
        new_path
    }

    pub fn move_file<P: AsRef<Path>>(&self, src_path: P, dest_path: P) -> bool {
        fs::rename(self.resolve(src_path), self.resolve(dest_path)).is_ok()
    }

    pub fn exists_file<P: AsRef<Path>>(&self, path: P) -> bool {
        self.resolve(path).exists()
    }

    pub fn delete_file<P: AsRef<Path>>(&self, path: P) -> bool {
        fs::remove_file(self.resolve(path)).is_ok()
    }
}