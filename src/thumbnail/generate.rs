use std::cmp::max;
use std::fs;
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use exif::{In, Tag};
use image::imageops::FilterType;
use image::DynamicImage;
use mime_guess::MimeGuess;
use tracing::{error, info, warn};
use wait_timeout::ChildExt;

const THUMBNAIL_TARGET_SIZE: u32 = 500;

fn generate_heic_thumbnail(load_path: &Path, save_path: &Path) -> std::io::Result<bool> {
    let mut child = Command::new("heif-thumbnailer")
        .arg("-s")
        .arg(THUMBNAIL_TARGET_SIZE.to_string())
        .arg(load_path)
        .arg(save_path)
        .spawn()?;

    match child.wait_timeout(Duration::from_secs(5)) {
        Ok(status) => Ok(status.map_or(false, |s| s.success())),
        Err(e) => {
            child.kill()?;
            child.wait()?;
            Err(e)
        }
    }
}

fn generate_video_frame<P: AsRef<Path>, R: AsRef<Path>>(
    load_path: P,
    save_path: R,
) -> Result<(), String> {
    let intermediate_path = save_path
        .as_ref()
        .to_str()
        .ok_or_else(|| "Failed to get string from path".to_string())?
        .rsplit_once('.')
        .map(|(before, _after)| before.to_string() + ".jpg")
        .ok_or_else(|| "Failed split path".to_string())?;

    let mut thumbnailer = ffthumb::Thumbnailer::builder()
        .time(5)
        .size(THUMBNAIL_TARGET_SIZE)
        .finalize();

    let input_path = load_path.as_ref().to_string_lossy();

    File::create(&intermediate_path)
        .map_err(|e| e.to_string())?
        .write_all(
            thumbnailer
                .generate(input_path.as_ref(), None, None)
                .map_err(|()| "ffmpeg-thumbnailer failed".to_string())?
                .as_slice(),
        )
        .map_err(|e| e.to_string())?;

    if let Ok(img) = image::open(&intermediate_path) {
        fs::remove_file(intermediate_path).map_err(|e| e.to_string())?;
        img.save(save_path).map_err(|e| e.to_string())
    } else {
        Err("Failed to open the file".to_string())
    }
}

pub fn generate_thumbnail<P, R>(load_path: P, save_path: R) -> bool
where
    P: AsRef<Path>,
    R: AsRef<Path>,
{
    let ext = load_path.as_ref().extension().unwrap().to_ascii_lowercase();

    let mime = MimeGuess::from_ext(ext.to_str().unwrap()).first_or_octet_stream();
    if mime.type_() == "video" {
        let result = generate_video_frame(&load_path, &save_path);

        match &result {
            Ok(_) => info!(
                "Generated thumbnail for video: {}",
                load_path.as_ref().display()
            ),
            Err(error_message) => warn!(
                "Thumbnail generation failed for video: {}\nCause: {}",
                load_path.as_ref().display(),
                error_message
            ),
        }

        return result.is_ok();
    }

    if ext == "heic" || ext == "heif" {
        return match generate_heic_thumbnail(load_path.as_ref(), save_path.as_ref()) {
            Ok(result) => result,
            Err(e) => {
                error!("Error generating heic/heif thumbnail: {e}");
                false
            }
        };
    }

    let exif_orientation = read_exif_orientation(load_path.as_ref());

    if let Ok(img) = image::open(&load_path) {
        return resize_and_save_image(&save_path, img, exif_orientation);
    }

    false
}

fn read_exif_orientation(path: &Path) -> Option<u32> {
    let mime = MimeGuess::from_ext(path.extension()?.to_str()?).first_or_octet_stream();
    if mime.type_() != "image" {
        return None;
    }

    let file = fs::File::open(path).ok()?;
    let mut bufreader = BufReader::new(&file);
    let reader = exif::Reader::new()
        .read_from_container(&mut bufreader)
        .ok()?;

    if let Some(orientation) = reader.get_field(Tag::Orientation, In::PRIMARY) {
        return orientation.value.get_uint(0);
    }

    None
}

fn resize_and_save_image<R>(save_path: R, img: DynamicImage, orientation: Option<u32>) -> bool
where
    R: AsRef<Path>,
{
    let (width, height) = resize_dimensions_fill(img.width(), img.height());

    let mut thumbnail = img.resize_exact(width, height, FilterType::Nearest);
    match orientation {
        Some(3) => thumbnail = thumbnail.rotate180(),
        Some(6) => thumbnail = thumbnail.rotate90(),
        Some(8) => thumbnail = thumbnail.rotate270(),
        _ => {}
    };
    thumbnail.save(save_path).is_ok()
}

#[inline]
fn resize_dimensions_fill(width: u32, height: u32) -> (u32, u32) {
    let wratio = THUMBNAIL_TARGET_SIZE as f64 / width as f64;
    let hratio = THUMBNAIL_TARGET_SIZE as f64 / height as f64;
    let ratio = f64::max(wratio, hratio);

    let nw = max((width as f64 * ratio).round() as u64, 1);
    let nh = max((height as f64 * ratio).round() as u64, 1);

    if nw > u64::from(u32::MAX) {
        let ratio = u32::MAX as f64 / width as f64;
        (u32::MAX, max((height as f64 * ratio).round() as u32, 1))
    } else if nh > u64::from(u32::MAX) {
        let ratio = u32::MAX as f64 / height as f64;
        (max((width as f64 * ratio).round() as u32, 1), u32::MAX)
    } else {
        (nw as u32, nh as u32)
    }
}
