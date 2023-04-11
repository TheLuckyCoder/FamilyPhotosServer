use std::borrow::Borrow;
use std::cmp::min;
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
    pub fn run(app_state: AppState) {
        actix_web::rt::spawn(async move {
            let instant = Instant::now();
            let data_scan = DataScan::scan(&app_state).await;
            data_scan.update_database(&app_state).await;

            log::debug!(
                "Photos scanning completed in {} seconds",
                instant.elapsed().as_secs()
            );
        });
    }

    async fn scan(app_state: &AppState) -> DataScan {
        let db = app_state.db.clone();
        let storage = app_state.storage.borrow();

        let users: Vec<User> = match db.send(GetUsers).await {
            Ok(Ok(users)) => users,
            _ => panic!("Could not load users"),
        };
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

        DataScan { results }
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
                        |_| Self::get_exif_timestamp(path),
                        |t| {
                            if let Ok(parsed) = OffsetDateTime::from_unix_timestamp(t as i64) {
                                Some(DataScan::convert_to_primitive_time(&parsed))
                            } else {
                                None
                            }
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
                    eprintln!("No timestamp: {}", entry.path().to_string_lossy());
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

    fn get_json_timestamp(path: &Path) -> std::io::Result<u64> {
        let json_file_name = path
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .trim_end_matches("-editat")
            .trim_end_matches("(1)")
            .to_string()
            + "."
            + path.extension().unwrap().to_str().unwrap()
            + ".json";

        let file = fs::File::open(json_file_name)?;
        let reader = BufReader::new(file);
        let json: GooglePhotoJsonData =
            serde_json::from_reader(reader).expect("JSON was ill-formatted");

        if json.creation_time.is_some() && json.photo_taken_time.is_some() {
            Ok(min(
                json.creation_time.unwrap(),
                json.photo_taken_time.unwrap(),
            ))
        } else {
            Ok(json
                .creation_time
                .unwrap_or_else(|| json.photo_taken_time.unwrap()))
        }
    }

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

    fn is_datetime(f: &Field, tag: Tag) -> Option<PrimitiveDateTime> {
        let format = format_description!("[year]:[month]:[day] [hour]:[minute]:[second]");

        if f.tag == tag {
            Self::single_ascii(&f.value).and_then(|s| PrimitiveDateTime::parse(s, &format).ok())
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

    fn get_regex_timestamp(path: &Path) -> Option<PrimitiveDateTime> {
        lazy_static! {
            static ref DATE_HOUR_PATTERN: Regex = Regex::new("([0-9]{8}).([0-9]{6})").unwrap();
            static ref DATE_HOUR_STRIP_PATTERN: Regex = Regex::new("([0-9]{4}-[0-9]{2}-[0-9]{2}-[0-9]{2}-[0-9]{2}-[0-9]{2})").unwrap(); // 2016-09-22-16-19-41
            static ref DATE_PATTERN: Regex = Regex::new("([0-9]{8})").unwrap();
            static ref MILLIS_PATTERN: Regex = Regex::new(".*([0-9]{13})").unwrap();
        }

        let name_os = path.file_stem().unwrap().to_os_string();
        let name = name_os.to_str().unwrap();

        if let Some(capture) = DATE_HOUR_PATTERN.captures(name) {
            let date: &str = &capture[1];
            let time: &str = &capture[2];

            if let Ok(parsed_time) = PrimitiveDateTime::parse(
                format!("{date} {time}").as_str(),
                format_description!("[year][month][day] [hour][minute][second]"),
            ) {
                return Some(parsed_time);
            }
        }

        if let Some(capture) = DATE_HOUR_STRIP_PATTERN.captures(name) {
            if let Ok(t) = PrimitiveDateTime::parse(
                &capture[1],
                format_description!("[year]-[month]-[day]-[hour]-[minute]-[second]"),
            ) {
                return Some(t);
            }
        }

        if let Some(capture) = DATE_PATTERN.captures(name) {
            let date = &capture[1];
            if let Ok(t) = PrimitiveDateTime::parse(
                format!("{date} 000000").as_str(),
                format_description!("[year][month][day] [hour][minute][second]"),
            ) {
                return Some(t);
            }
        }

        if let Some(capture) = MILLIS_PATTERN.captures(name) {
            let millis: i64 = capture[1].parse().ok()?;
            let seconds = millis / 1000;
            if let Ok(parsed_time) = OffsetDateTime::from_unix_timestamp(seconds) {
                return Some(DataScan::convert_to_primitive_time(&parsed_time));
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
    use super::*;
    use std::path::PathBuf;
    use time::macros::datetime;

    #[test]
    fn test_regex_timestamp() {
        let path1 = PathBuf::from("IMG_20160922_160430.jpg");
        let path2 = PathBuf::from("IMG_2016-09-22-16-04-30.jpg");
        let path3 = PathBuf::from("20160922.jpg");
        let path4 = PathBuf::from("random-1474560270000.jpg");

        let expected_date = Some(datetime!(2016-09-22 16:04:30));
        assert_eq!(DataScan::get_regex_timestamp(&path1), expected_date);

        assert_eq!(DataScan::get_regex_timestamp(&path2), expected_date);

        assert_eq!(
            DataScan::get_regex_timestamp(&path3),
            Some(datetime!(2016-09-22 00:00:00))
        );

        assert_eq!(DataScan::get_regex_timestamp(&path4), expected_date);
    }
}
