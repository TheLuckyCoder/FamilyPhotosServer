use rayon::prelude::*;
use tokio::task;

use generate::*;
pub use manager::*;

use crate::http::AppState;
use crate::model::photo::{Photo, PhotoBase};

mod generate;
mod manager;

pub async fn generate_all_foreground(app_state: &AppState) -> Result<(), String> {
    let photos: Vec<Photo> = app_state
        .photos_repo
        .get_photos()
        .await
        .map_err(|_| "Could not load photos".to_string())?;

    photos.into_par_iter().for_each(|photo| {
        let photo_path = app_state.storage.resolve_photo(photo.partial_path());
        let preview_path = app_state
            .storage
            .resolve_preview(photo.partial_preview_path());

        if photo_path.exists() && !preview_path.exists() {
            generate_preview(photo_path, preview_path);
        }
    });

    Ok(())
}

pub async fn generate_all_background(app_state: AppState) -> Result<(), String> {
    let photos: Vec<Photo> = app_state
        .photos_repo
        .get_photos()
        .await
        .map_err(|_| "Could not load photos".to_string())?;

    // We only use one thread for this as we don't want to take up the whole CPU
    task::spawn(async move {
        for photo in photos {
            let photo_path = app_state.storage.resolve_photo(photo.partial_path());
            let preview_path = app_state
                .storage
                .resolve_preview(photo.partial_preview_path());

            if photo_path.exists() {
                app_state
                    .preview_manager
                    .request_previews(photo.id, photo_path, preview_path)
                    .await;
            }
        }
    });

    Ok(())
}
