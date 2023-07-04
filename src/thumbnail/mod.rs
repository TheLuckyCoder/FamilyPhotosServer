pub use generate::*;

use crate::http::AppState;
use crate::model::photo::{Photo, PhotoBase};

mod generate;

pub async fn generate_background(app_state: AppState) -> Result<(), String> {
    let storage = app_state.storage;

    let photos: Vec<Photo> = app_state
        .photos_repo
        .get_photos()
        .await
        .map_err(|_| "Could not load photos".to_string())?;

    // We only use one thread for this as we don't want to take up the whole CPU
    rayon::spawn(move || {
        for photo in photos {
            let photo_path = storage.resolve(photo.partial_path().unwrap());
            let thumbnail_path = storage.resolve_thumbnail(photo.partial_thumbnail_path());

            if photo_path.exists() && !thumbnail_path.exists() {
                generate_thumbnail(photo_path, thumbnail_path);
            }
        }
    });

    Ok(())
}
