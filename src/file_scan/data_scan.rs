use std::borrow::Borrow;
use std::ffi::OsStr;
use std::fs;
use std::time::Instant;

use rayon::prelude::*;
use time::PrimitiveDateTime;
use tokio::task;
use tracing::{debug, info, warn};
use walkdir::{DirEntry, WalkDir};

use crate::file_scan::timestamp;
use crate::model::photo::{Photo, PhotoBody};
use crate::{AppState, FileStorage, User};

pub struct DataScan {
    results: Vec<(User, Vec<PhotoBody>)>,
}

impl DataScan {
    pub fn run(app_state: AppState) {
        task::spawn(async move {
            let users: Vec<User> = app_state
                .users_repo
                .get_users()
                .await
                .expect("Could not load users");

            let instant = Instant::now();
            let data_scan = Self::scan(users, app_state.storage.borrow());
            data_scan.update_database(&app_state).await;

            debug!(
                "Photos scanning completed in {} seconds",
                instant.elapsed().as_secs()
            );
        });
    }

    fn scan(users: Vec<User>, storage: &FileStorage) -> Self {
        debug!(
            "Started scanning user's photos: {:?}",
            users
                .iter()
                .map(|user| user.user_name.clone())
                .collect::<Vec<_>>()
        );

        let results = users
            .into_par_iter()
            .map(|user| Self::scan_user_photos(storage, user))
            .collect::<Vec<_>>();

        Self { results }
    }

    fn scan_user_photos(storage: &FileStorage, user: User) -> (User, Vec<PhotoBody>) {
        let mut photos = Vec::with_capacity(8192 * 4);

        let user_path = storage.resolve(&user.user_name);
        if !user_path.exists() {
            fs::create_dir(user_path).unwrap()
        } else {
            let json_extension = Some(OsStr::new("json"));
            let walk_dir = WalkDir::new(user_path).max_depth(2);

            for entry in walk_dir.into_iter().filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_dir() || path.extension() == json_extension {
                    continue;
                }

                if let Some(photo) = Self::parse_image(user.user_name.clone(), entry) {
                    photos.push(photo)
                }
            }
        }

        info!("Finished scanning for {}", user.user_name);

        (user, photos)
    }

    pub fn parse_image(user_name: String, entry: DirEntry) -> Option<PhotoBody> {
        let path = entry.path();

        let timestamp = timestamp::get_timestamp_for_path(path);

        match timestamp {
            Some(date_time) => Some(PhotoBody {
                user_name: user_name.clone(),
                name: entry.file_name().to_string_lossy().to_string(),
                created_at: PrimitiveDateTime::new(date_time.date(), date_time.time()),
                file_size: fs::metadata(path).map_or(0i64, |data| data.len() as i64),
                folder: if entry.depth() == 2 {
                    Some(path.parent()?.file_name()?.to_string_lossy().to_string())
                } else {
                    None
                },
            }),
            None => {
                warn!("No timestamp: {}", path.display());
                None
            }
        }
    }

    async fn update_database(self, app_state: &AppState) {
        let storage = app_state.storage.borrow();
        let photos_repo = app_state.photos_repo.borrow();

        let existing_photos: Vec<Photo> = photos_repo
            .get_photos()
            .await
            .expect("Failed to get photos");
        let existing_photos_names: Vec<String> = existing_photos
            .iter()
            .map(|photo| photo.full_name())
            .collect();

        for (user, mut found_photos) in self.results {
            info!(
                "Scanned {} photos in user {}",
                found_photos.len(),
                user.user_name
            );

            // Add any photo that was not already in the db
            // Keep only new photos
            found_photos.retain(|photo| !existing_photos_names.contains(&photo.full_name()));

            if !found_photos.is_empty() {
                info!(
                    "Adding {} new photos to user {}",
                    found_photos.len(),
                    user.user_name
                );

                for chunk in found_photos.chunks(512) {
                    photos_repo
                        .insert_photos(chunk)
                        .await
                        .expect("Failed to insert photos");
                }
            }

            let removed_photos = existing_photos
                .iter()
                .filter(|photo| {
                    photo.user_name == user.user_name
                        && !storage
                            .resolve(format!("{}/{}", user.user_name, photo.full_name()))
                            .exists()
                })
                .map(|photo| photo.id)
                .collect::<Vec<i64>>();

            if !removed_photos.is_empty() {
                info!(
                    "Removing {} photos from user {}",
                    removed_photos.len(),
                    user.user_name
                );

                // TODO: Improve performance
                for removed_photo_id in removed_photos {
                    app_state
                        .photos_repo
                        .delete_photo(removed_photo_id)
                        .await
                        .expect("Failed to delete photo");
                }
            }
        }
    }
}
