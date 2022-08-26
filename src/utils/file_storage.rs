use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct FileStorage {
    base_path: PathBuf,
}

impl FileStorage {
    pub fn new(path: String) -> FileStorage {
        let base_path = Path::new(path.as_str()).to_owned();

        if !base_path.exists() {
            fs::create_dir_all(base_path.as_path())
                .expect("Could not create the base storage path");
        } else {
            assert!(base_path.is_dir());
        }

        FileStorage { base_path }
    }

    pub fn resolve<P: AsRef<Path>>(&self, relative: P) -> PathBuf {
        let mut new_path = self.base_path.clone();
        new_path.push(relative);
        new_path
    }

    pub fn move_file<P: AsRef<Path>>(&self, src_relative: P, dest_relative: P) -> bool {
        fs::rename(self.resolve(src_relative), self.resolve(dest_relative)).is_ok()
    }

    pub fn delete_file<P: AsRef<Path>>(&self, relative: P) -> bool {
        fs::remove_file(self.resolve(relative)).is_ok()
    }
}