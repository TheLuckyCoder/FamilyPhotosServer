use crate::thumbnail::generate::generate_thumbnail;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::task;
use tokio::time::sleep;

#[derive(Default, Clone)]
pub struct ThumbnailManager {
    thumbnails_being_processed: Arc<RwLock<HashSet<i64>>>,
}

impl ThumbnailManager {
    pub async fn request_thumbnail(
        &self,
        photo_id: i64,
        photo_path: PathBuf,
        thumbnail_path: PathBuf,
    ) -> bool {
        while self
            .thumbnails_being_processed
            .read()
            .await
            .contains(&photo_id)
        {
            sleep(Duration::from_millis(1)).await;
        }

        if !thumbnail_path.exists() {
            self.thumbnails_being_processed
                .write()
                .await
                .insert(photo_id);

            let result =
                task::spawn_blocking(move || generate_thumbnail(photo_path, thumbnail_path))
                    .await
                    .unwrap_or(false);

            self.thumbnails_being_processed
                .write()
                .await
                .remove(&photo_id);

            result
        } else {
            true
        }
    }
}
