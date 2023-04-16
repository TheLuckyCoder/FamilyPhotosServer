use actix_files::file_extension_to_mime;
use exif::{Field, In, Tag, Value};
use lazy_static::lazy_static;
use regex::Regex;
use serde::Deserialize;
use std::fs;
use std::io::BufReader;
use std::path::Path;
use std::str::from_utf8;
use time::macros::format_description;
use time::{OffsetDateTime, PrimitiveDateTime};

pub fn get_timestamp_for_path<P: AsRef<Path>>(path: P) -> Option<PrimitiveDateTime> {
    get_json_timestamp(path.as_ref())
        .map_or_else(
            || get_exif_timestamp(path.as_ref()),
            |json_timestamp| {
                OffsetDateTime::from_unix_timestamp(json_timestamp as i64)
                    .map(|parsed| convert_to_primitive_time(&parsed))
                    .ok()
            },
        )
        .or_else(|| get_regex_timestamp(path))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GooglePhotoJsonData {
    creation_time: Option<u64>,
    photo_taken_time: Option<u64>,
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
            if let Some(d) = is_datetime(f, Tag::DateTimeOriginal) {
                return Some(d);
            } else if let Some(d) = is_datetime(f, Tag::DateTime) {
                return Some(d);
            } else if let Some(d) = is_datetime(f, Tag::DateTimeDigitized) {
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
            return Some(convert_to_primitive_time(&parsed_time));
        }
    }

    None
}

fn convert_to_primitive_time(date_time: &OffsetDateTime) -> PrimitiveDateTime {
    PrimitiveDateTime::new(date_time.date(), date_time.time())
}

#[cfg(test)]
mod tests {
    use time::macros::datetime;

    use super::*;

    #[test]
    fn test_regex_timestamp() {
        let expected_date = Some(datetime!(2016-09-22 16:04:30));
        assert_eq!(
            get_regex_timestamp("IMG_20160922_160430.jpg"),
            expected_date
        );

        assert_eq!(
            get_regex_timestamp("IMG_2016-09-22-16-04-30.jpg"),
            expected_date
        );

        assert_eq!(
            get_regex_timestamp("20160922.jpg"),
            Some(datetime!(2016-09-22 00:00:00))
        );

        assert_eq!(
            get_regex_timestamp("random-1474560270000.jpg"),
            expected_date
        );

        assert_eq!(
            get_regex_timestamp("random-IMG_2016-09-22-160430 (5).jpg"),
            expected_date
        );
    }
}
