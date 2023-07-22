pub use generate::*;
use rayon::prelude::*;

use crate::http::AppState;
use crate::model::photo::{Photo, PhotoBase};
use crate::utils::file_storage::FileStorage;

mod generate;

pub async fn generate_all_foreground(app_state: &AppState) -> Result<(), String> {
    let photos: Vec<Photo> = app_state
        .photos_repo
        .get_photos()
        .await
        .map_err(|_| "Could not load photos".to_string())?;

    photos.par_iter().for_each(|photo| {
        create_thumbnail(&app_state.storage, photo);
    });

    Ok(())
}

pub async fn generate_all_background(app_state: AppState) -> Result<(), String> {
    let storage = app_state.storage;

    let photos: Vec<Photo> = app_state
        .photos_repo
        .get_photos()
        .await
        .map_err(|_| "Could not load photos".to_string())?;

    // We only use one thread for this as we don't want to take up the whole CPU
    rayon::spawn(move || {
        for photo in photos {
            create_thumbnail(&storage, &photo);
        }
    });

    Ok(())
}

fn create_thumbnail(storage: &FileStorage, photo: &Photo) {
    let photo_path = storage.resolve_photo(photo.partial_path());
    let thumbnail_path = storage.resolve_thumbnail(photo.partial_thumbnail_path());

    if photo_path.exists() && !thumbnail_path.exists() {
        generate_thumbnail(photo_path, thumbnail_path);
    }
}
