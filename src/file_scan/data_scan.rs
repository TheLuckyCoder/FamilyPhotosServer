use std::borrow::Borrow;
use std::ffi::OsStr;
use std::fs;
use std::time::Instant;

use rayon::prelude::*;
use time::PrimitiveDateTime;
use tokio::task;
use tracing::{debug, info, warn};
use walkdir::{DirEntry, WalkDir};

use crate::db::photos_db::{DeletePhotos, GetPhotos, InsertPhotos};
use crate::db::Handler;
use crate::file_scan::timestamp;
use crate::model::photo::Photo;
use crate::{AppState, FileStorage, GetUsers, User};

pub struct DataScan {
    results: Vec<(User, Vec<Photo>)>,
}

impl DataScan {
    pub async fn run(app_state: AppState) {
        let users: Vec<User> = match app_state.pool.send(GetUsers).await {
            Ok(users) => users,
            _ => panic!("Could not load users"),
        };

        task::spawn(async move {
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

    fn scan_user_photos(storage: &FileStorage, user: User) -> (User, Vec<Photo>) {
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

                if let Some(photo) = Self::parse_image(user.id, entry) {
                    photos.push(photo)
                }
            }
        }

        info!("Finished scanning for {}", user.user_name);

        (user, photos)
    }

    pub fn parse_image(user_id: i64, entry: DirEntry) -> Option<Photo> {
        let path = entry.path();

        let timestamp = timestamp::get_timestamp_for_path(path);

        match timestamp {
            Some(date_time) => Some(Photo {
                id: 0,
                owner: user_id,
                name: entry.file_name().to_string_lossy().to_string(),
                time_created: PrimitiveDateTime::new(date_time.date(), date_time.time()),
                file_size: fs::metadata(path).map_or(0i64, |data| data.len() as i64),
                folder: if entry.depth() == 2 {
                    Some(path.parent()?.file_name()?.to_string_lossy().to_string())
                } else {
                    None
                },
                caption: None,
            }),
            None => {
                warn!("No timestamp: {}", path.display());
                None
            }
        }
    }

    async fn update_database(self, app_state: &AppState) {
        let pool = &app_state.pool;
        let storage = app_state.storage.borrow();

        let existing_photos: Vec<Photo> = pool.send(GetPhotos::All).await.unwrap();
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
                    pool.send(InsertPhotos(Vec::from(chunk)))
                        .await
                        .expect("Failed to insert photos");
                }
            }

            let removed_photos = existing_photos
                .iter()
                .filter(|photo| {
                    photo.owner == user.id
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
                pool.send(DeletePhotos {
                    ids: removed_photos,
                })
                .await
                .unwrap();
            }
        }
    }
}
