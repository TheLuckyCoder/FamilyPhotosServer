use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct FileStorage {
    storage_folder: PathBuf,
    thumbnail_folder: PathBuf,
}

impl FileStorage {
    pub fn new(storage_path: String, thumbnail_path: Option<String>) -> FileStorage {
        let base_folder = PathBuf::from(storage_path);

        let thumbnail_folder = thumbnail_path.map(PathBuf::from).unwrap_or_else(|| {
            let mut path = base_folder.clone();
            path.push(".thumbnail");
            path
        });

        if !base_folder.exists() {
            fs::create_dir_all(base_folder.as_path())
                .expect("Could not create the base storage path");
        } else {
            assert!(base_folder.is_dir());
        }

        if !thumbnail_folder.exists() {
            fs::create_dir_all(&thumbnail_folder).unwrap_or_else(|_| {
                panic!(
                    "Failed to create thumbnail folder at {}",
                    thumbnail_folder.display()
                )
            });
        } else {
            assert!(thumbnail_folder.is_dir());
        }

        FileStorage {
            storage_folder: base_folder,
            thumbnail_folder,
        }
    }

    pub fn resolve<P: AsRef<Path>>(&self, relative: P) -> PathBuf {
        let mut new_path = self.storage_folder.clone();
        new_path.push(relative);
        new_path
    }

    pub fn resolve_thumbnail<P: AsRef<Path>>(&self, relative: P) -> PathBuf {
        let mut new_path = self.thumbnail_folder.clone();
        new_path.push(relative);
        new_path
    }

    pub fn move_file<P1: AsRef<Path>, P2: AsRef<Path>>(
        &self,
        src_relative: P1,
        dest_relative: P2,
    ) -> std::io::Result<()> {
        fs::rename(self.resolve(src_relative), self.resolve(dest_relative))
    }

    pub fn delete_file<P: AsRef<Path>>(&self, relative: P) -> std::io::Result<()> {
        fs::remove_file(self.resolve(relative))
    }
}
