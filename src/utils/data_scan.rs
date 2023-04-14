use std::borrow::Borrow;
use std::ffi::OsStr;
use std::fs;
use std::io::BufReader;
use std::path::Path;
use std::str::from_utf8;
use std::time::Instant;

use actix_files::file_extension_to_mime;
use exif::{Field, In, Tag, Value};
use lazy_static::lazy_static;
use rayon::prelude::*;
use regex::Regex;
use serde::Deserialize;
use time::macros::format_description;
use time::{OffsetDateTime, PrimitiveDateTime};
use walkdir::WalkDir;

use crate::db::photos_db::{DeletePhotos, GetPhotos, InsertPhotos};
use crate::model::photo::Photo;
use crate::{AppState, FileStorage, GetUsers, User};

pub struct DataScan {
    results: Vec<(User, Vec<Photo>)>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GooglePhotoJsonData {
    creation_time: Option<u64>,
    photo_taken_time: Option<u64>,
}

impl DataScan {
    pub async fn run(app_state: AppState) {
        let db = app_state.db.borrow();
        let users: Vec<User> = match db.send(GetUsers).await {
            Ok(Ok(users)) => users,
            _ => panic!("Could not load users"),
        };

        actix_web::rt::spawn(async move {
            let instant = Instant::now();
            let data_scan = Self::scan(users, app_state.storage.borrow());
            data_scan.update_database(&app_state).await;

            log::debug!(
                "Photos scanning completed in {} seconds",
                instant.elapsed().as_secs()
            );
        });
    }

    fn scan(users: Vec<User>, storage: &FileStorage) -> Self {
        log::debug!(
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
            let walk_dir = WalkDir::new(user_path).max_depth(2).contents_first(true);

            for result in walk_dir.into_iter() {
                if result.is_err() {
                    continue;
                }
                let entry = result.unwrap();
                let path = entry.path();
                if path.is_dir() || path.extension() == Some(OsStr::new("json")) {
                    continue;
                }

                let timestamp = Self::get_json_timestamp(path)
                    .map_or_else(
                        || Self::get_exif_timestamp(path),
                        |json_timestamp| {
                            OffsetDateTime::from_unix_timestamp(json_timestamp as i64)
                                .map(|parsed| Self::convert_to_primitive_time(&parsed))
                                .ok()
                        },
                    )
                    .or_else(|| Self::get_regex_timestamp(path));

                if let Some(date_time) = timestamp {
                    photos.push(Photo {
                        id: 0,
                        owner: user.id,
                        name: entry.file_name().to_string_lossy().to_string(),
                        time_created: PrimitiveDateTime::new(date_time.date(), date_time.time()),
                        file_size: fs::metadata(path).map_or(0i64, |data| data.len() as i64),
                        folder: if entry.depth() == 2 {
                            Some(
                                path.parent()
                                    .unwrap()
                                    .file_name()
                                    .unwrap()
                                    .to_string_lossy()
                                    .to_string(),
                            )
                        } else {
                            None
                        },
                        caption: None,
                    })
                } else {
                    eprintln!("No timestamp: {}", entry.path().display());
                }
            }
        }

        log::info!("Finished scanning for {}", user.user_name);

        (user, photos)
    }

    async fn update_database(self, app_state: &AppState) {
        let db = app_state.db.clone();
        let storage = app_state.storage.borrow();

        let existing_photos: Vec<Photo> = db.send(GetPhotos::All).await.ok().unwrap().unwrap();
        let existing_photos_names: Vec<String> = existing_photos
            .iter()
            .map(|photo| photo.full_name())
            .collect();

        for (user, mut found_photos) in self.results {
            log::info!(
                "Scanned {} photos in user {}",
                found_photos.len(),
                user.user_name
            );

            // Add any photo that was not already in the database
            // Keep only new photos
            found_photos.retain(|photo| !existing_photos_names.contains(&photo.full_name()));

            if !found_photos.is_empty() {
                log::info!(
                    "Adding {} new photos to user {}",
                    found_photos.len(),
                    user.user_name
                );

                for chunk in found_photos.chunks(512) {
                    db.send(InsertPhotos(Vec::from(chunk)))
                        .await
                        .unwrap()
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
                log::info!(
                    "Removing {} photos from user {}",
                    removed_photos.len(),
                    user.user_name
                );
                db.send(DeletePhotos {
                    ids: removed_photos,
                })
                .await
                .unwrap()
                .unwrap();
            }
        }
    }

    fn get_json_timestamp(path: &Path) -> Option<u64> {
        let json_file_name = std::format!(
            "{}.{}.json",
            path.file_stem()?
                .to_string_lossy()
                .trim_end_matches("(1)")
                .trim(),
            path.extension()?.to_str()?
        );

        let file = fs::File::open(&json_file_name).ok()?;
        let reader = BufReader::new(file);
        let json = serde_json::from_reader::<_, GooglePhotoJsonData>(reader);

        match json {
            Ok(json_data) => json_data.photo_taken_time.or(json_data.creation_time),
            Err(e) => {
                log::error!("Failed parsing Json ({json_file_name}): {e}");
                None
            }
        }
    }

    fn is_datetime(f: &Field, tag: Tag) -> Option<PrimitiveDateTime> {
        let format = format_description!("[year]:[month]:[day] [hour]:[minute]:[second]");

        fn single_ascii(value: &Value) -> Option<&str> {
            match value {
                Value::Ascii(ref v) if v.len() == 1 => from_utf8(&v[0]).ok(),
                Value::Ascii(ref v) if v.len() > 1 => {
                    for t in &v[1..] {
                        if !t.is_empty() {
                            return None;
                        }
                    }
                    return from_utf8(&v[0]).ok();
                }
                _ => None,
            }
        }

        if f.tag == tag {
            single_ascii(&f.value).and_then(|s| PrimitiveDateTime::parse(s, &format).ok())
        } else {
            None
        }
    }

    fn get_exif_timestamp(path: &Path) -> Option<PrimitiveDateTime> {
        let mime = file_extension_to_mime(path.extension()?.to_str()?);
        if mime.type_() != "image" {
            return None;
        }

        let file = fs::File::open(path).ok()?;
        let mut bufreader = BufReader::new(&file);
        let reader = exif::Reader::new()
            .read_from_container(&mut bufreader)
            .ok()?;

        for f in reader.fields() {
            if f.ifd_num == In::PRIMARY {
                if let Some(d) = Self::is_datetime(f, Tag::DateTimeOriginal) {
                    return Some(d);
                } else if let Some(d) = Self::is_datetime(f, Tag::DateTime) {
                    return Some(d);
                } else if let Some(d) = Self::is_datetime(f, Tag::DateTimeDigitized) {
                    return Some(d);
                }
            }
        }

        None
    }

    fn get_regex_timestamp<P: AsRef<Path>>(path: P) -> Option<PrimitiveDateTime> {
        lazy_static! {
            static ref DATE_HOUR_PATTERN: Regex
                = Regex::new(r"(\d{4})\D*(\d{2})\D*(\d{2})\D*(\d{2})\D*(\d{2})\D*(\d{2})").unwrap(); // 2016-09-22-16-19-41
            static ref DATE_PATTERN: Regex = Regex::new(r"(\d{8})").unwrap();
            static ref MILLIS_PATTERN: Regex = Regex::new(r".*(\d{13})").unwrap();
        }

        let name = path.as_ref().file_stem()?.to_string_lossy().to_string();

        if let Some(capture) = DATE_HOUR_PATTERN.captures(&name) {
            let year = &capture[1];
            let month = &capture[2];
            let day = &capture[3];
            let hour = &capture[4];
            let minute = &capture[5];
            let second = &capture[6];

            let string = format!("{year}{month}{day}{hour}{minute}{second}");
            let result = PrimitiveDateTime::parse(
                string.as_str(),
                format_description!("[year][month][day][hour][minute][second]"),
            );

            if let Ok(parsed_time) = result {
                return Some(parsed_time);
            }
        }

        if let Some(capture) = DATE_PATTERN.captures(&name) {
            let date = &capture[1];
            if let Ok(parsed_time) = PrimitiveDateTime::parse(
                format!("{date} 000000").as_str(),
                format_description!("[year][month][day] [hour][minute][second]"),
            ) {
                return Some(parsed_time);
            }
        }

        if let Some(capture) = MILLIS_PATTERN.captures(&name) {
            let millis: i64 = capture[1].parse().ok()?;
            let seconds = millis / 1000;
            if let Ok(parsed_time) = OffsetDateTime::from_unix_timestamp(seconds) {
                return Some(Self::convert_to_primitive_time(&parsed_time));
            }
        }

        None
    }

    fn convert_to_primitive_time(date_time: &OffsetDateTime) -> PrimitiveDateTime {
        PrimitiveDateTime::new(date_time.date(), date_time.time())
    }
}

#[cfg(test)]
mod tests {
    use time::macros::datetime;

    use super::*;

    #[test]
    fn test_regex_timestamp() {
        let expected_date = Some(datetime!(2016-09-22 16:04:30));
        assert_eq!(
            DataScan::get_regex_timestamp("IMG_20160922_160430.jpg"),
            expected_date
        );

        assert_eq!(
            DataScan::get_regex_timestamp("IMG_2016-09-22-16-04-30.jpg"),
            expected_date
        );

        assert_eq!(
            DataScan::get_regex_timestamp("20160922.jpg"),
            Some(datetime!(2016-09-22 00:00:00))
        );

        assert_eq!(
            DataScan::get_regex_timestamp("random-1474560270000.jpg"),
            expected_date
        );

        assert_eq!(
            DataScan::get_regex_timestamp("random-IMG_2016-09-22-160430 (5).jpg"),
            expected_date
        );
    }
}
