pub use generate::*;

use crate::db::photos_db::GetPhotos;
use crate::db::users_db::GetUsers;
use crate::model::photo::Photo;
use crate::model::user::User;
use crate::utils::AppState;

mod generate;

pub async fn generate_background(app_state: &AppState) -> Result<(), String> {
    let db = app_state.db.clone();
    let storage = app_state.storage.clone();

    let users: Vec<User> = match db.send(GetUsers).await {
        Ok(Ok(users)) => users,
        _ => return Err("Could not load users".to_string()),
    };

    let photos: Vec<Photo> = match db.send(GetPhotos::All).await {
        Ok(Ok(photos)) => photos,
        _ => return Err("Could not load photos".to_string()),
    };

    // We only use one thread for this as we don't want to take up the whole CPU
    rayon::spawn(move || {
        for photo in photos {
            let user = users.iter().find(|user| user.id == photo.owner).unwrap();
            let photo_path = storage.resolve(photo.partial_path(user).unwrap());
            let thumbnail_path = storage.resolve_thumbnail(photo.partial_thumbnail_path());

            if photo_path.exists() && !thumbnail_path.exists() {
                generate_thumbnail(photo_path, thumbnail_path);
            }
        }
    });

    Ok(())
}
