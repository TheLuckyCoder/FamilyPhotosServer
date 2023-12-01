use crate::previews::generate::generate_preview;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::task;
use tokio::time::sleep;

#[derive(Default, Clone)]
pub struct PreviewsManager {
    previews_being_processed: Arc<RwLock<HashSet<i64>>>,
}

impl PreviewsManager {
    pub async fn request_previews(
        &self,
        photo_id: i64,
        photo_path: PathBuf,
        preview_path: PathBuf,
    ) -> bool {
        while self
            .previews_being_processed
            .read()
            .await
            .contains(&photo_id)
        {
            sleep(Duration::from_millis(1)).await;
        }

        if !preview_path.exists() {
            self.previews_being_processed.write().await.insert(photo_id);

            let result = task::spawn_blocking(move || generate_preview(photo_path, preview_path))
                .await
                .unwrap_or(false);

            self.previews_being_processed
                .write()
                .await
                .remove(&photo_id);

            result
        } else {
            true
        }
    }
}
