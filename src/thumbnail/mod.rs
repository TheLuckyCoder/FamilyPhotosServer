use crate::db::Handler;
pub use generate::*;

use crate::db::photos_db::GetPhotos;
use crate::db::users_db::GetUsers;
use crate::http::AppState;
use crate::model::photo::Photo;
use crate::model::user::User;

mod generate;

pub async fn generate_background(app_state: &AppState) -> Result<(), String> {
    let pool = &app_state.pool;
    let storage = app_state.storage.clone();

    let users: Vec<User> = match pool.send(GetUsers).await {
        Ok(users) => users,
        _ => return Err("Could not load users".to_string()),
    };

    let photos: Vec<Photo> = match pool.send(GetPhotos::All).await {
        Ok(photos) => photos,
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
