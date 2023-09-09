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

    photos.par_iter().for_each(|photo| {
        let photo_path = app_state.storage.resolve_photo(photo.partial_path());
        let thumbnail_path = app_state
            .storage
            .resolve_thumbnail(photo.partial_thumbnail_path());

        if photo_path.exists() && !thumbnail_path.exists() {
            generate_thumbnail(photo_path, thumbnail_path);
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
            let thumbnail_path = app_state
                .storage
                .resolve_thumbnail(photo.partial_thumbnail_path());

            if photo_path.exists() {
                app_state
                    .thumbnail_manager
                    .request_thumbnail(photo.id, photo_path, thumbnail_path)
                    .await;
            }
        }
    });

    Ok(())
}
