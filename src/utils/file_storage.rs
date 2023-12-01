use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct FileStorage {
    storage_folder: PathBuf,
    preview_folder: PathBuf,
}

impl FileStorage {
    pub fn new(storage_path: String, preview_path: Option<String>) -> FileStorage {
        let base_folder = PathBuf::from(storage_path);

        let preview_folder = preview_path.map(PathBuf::from).unwrap_or_else(|| {
            let mut path = base_folder.clone();
            path.push(".preview");
            path
        });

        if !base_folder.exists() {
            fs::create_dir_all(base_folder.as_path())
                .expect("Could not create the base storage path");
        } else {
            assert!(base_folder.is_dir());
        }

        if !preview_folder.exists() {
            fs::create_dir_all(&preview_folder).unwrap_or_else(|_| {
                panic!(
                    "Failed to create preview folder at {}",
                    preview_folder.display()
                )
            });
        } else {
            assert!(preview_folder.is_dir());
        }

        FileStorage {
            storage_folder: base_folder,
            preview_folder,
        }
    }

    pub fn resolve_photo<P: AsRef<Path>>(&self, relative: P) -> PathBuf {
        self.storage_folder.join(relative.as_ref())
    }

    pub fn resolve_preview<P: AsRef<Path>>(&self, relative: P) -> PathBuf {
        self.preview_folder.join(relative.as_ref())
    }

    pub fn move_file<P1: AsRef<Path>, P2: AsRef<Path>>(
        &self,
        src_relative: P1,
        dest_relative: P2,
    ) -> std::io::Result<()> {
        let destination_path = self.resolve_photo(dest_relative);

        // Create parent directory if it doesn't exist
        if let Some(parent) = destination_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }

        fs::rename(self.resolve_photo(src_relative), destination_path)
    }

    pub fn delete_file<P: AsRef<Path>>(&self, relative: P) -> std::io::Result<()> {
        fs::remove_file(self.resolve_photo(relative))
    }
}
