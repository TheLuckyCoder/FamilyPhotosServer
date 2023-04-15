use crate::utils::file_storage::FileStorage;
use exif::In;
use serde::Serialize;
use std::fs;
use std::io::BufReader;
use std::path::Path;

pub mod data_scan;
pub mod env_reader;
pub mod file_storage;
pub mod password_hash;
pub mod primitive_date_time_serde;

#[derive(Clone)]
pub struct AppState {
    pub db: actix::Addr<crate::db::DbActor>,
    pub storage: FileStorage,
}

#[derive(Debug, Serialize)]
pub struct ExifField {
    tag: String,
    value: String,
}

pub fn read_exif<P: AsRef<Path>>(absolute_path: P) -> Option<Vec<ExifField>> {
    let file = fs::File::open(absolute_path).ok()?;
    let mut bufreader = BufReader::new(&file);
    let reader = exif::Reader::new()
        .read_from_container(&mut bufreader)
        .ok()?;

    let mut exif_data = vec![];

    for f in reader.fields() {
        if f.ifd_num == In::PRIMARY {
            exif_data.push(ExifField {
                tag: f.tag.to_string(),
                value: f.value.display_as(f.tag).to_string(),
            });
        }
    }

    Some(exif_data)
}
