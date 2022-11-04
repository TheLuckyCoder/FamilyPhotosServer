use std::cmp::max;
use std::fs;
use std::path::Path;
use std::process::Command;

use actix_files::file_extension_to_mime;
use image::imageops::FilterType;
use image::DynamicImage;

const THUMBNAIL_TARGET_SIZE: u32 = 400;

fn generate_heic_thumbnail(load_path: &Path, save_path: &Path) -> std::io::Result<()> {
    Command::new("heif-thumbnailer")
        .arg("-s")
        .arg(THUMBNAIL_TARGET_SIZE.to_string())
        .arg(load_path)
        .arg(save_path)
        .spawn()?
        .wait()?;

    Ok(())
}

fn generate_video_frame<P: AsRef<Path>, R: AsRef<Path>>(load_path: P, save_path: R) -> Option<()> {
    log::info!(
        "Generating thumbnail for video {}",
        load_path.as_ref().display()
    );
    let intermediate_path = save_path
        .as_ref()
        .to_str()?
        .rsplit_once('.')
        .map(|(before, _after)| before.to_string() + ".jpg")?;

    let mut child = Command::new("ffmpegthumbnailer")
        .arg("-i")
        .arg(load_path.as_ref().as_os_str())
        .arg("-o")
        .arg(&intermediate_path)
        .arg("-s")
        .arg((THUMBNAIL_TARGET_SIZE + 150).to_string())
        .arg("-a") // Make it square
        .spawn()
        .ok()?;

    child.wait().ok()?;

    if let Ok(img) = image::open(&intermediate_path) {
        fs::remove_file(intermediate_path).ok()?;
        img.save(save_path).ok()
    } else {
        None
    }
}

pub fn generate_thumbnail<P, R>(load_path: P, save_path: R) -> bool
where
    P: AsRef<Path>,
    R: AsRef<Path>,
{
    let ext = load_path.as_ref().extension().unwrap().to_ascii_lowercase();
    let mime = file_extension_to_mime(ext.to_str().unwrap());
    if mime.type_() == "video" {
        return generate_video_frame(&load_path, &save_path).is_some();
    }

    if ext == "heic" || ext == "heif" {
        return match generate_heic_thumbnail(load_path.as_ref(), save_path.as_ref()) {
            Ok(_) => true,
            Err(e) => {
                log::error!("Error generating heic thumbnail {e}");
                false
            }
        };
    }

    if let Ok(img) = image::open(&load_path) {
        return resize_and_save_image(&save_path, img);
    }

    false
}

fn resize_and_save_image<R>(save_path: R, img: DynamicImage) -> bool
where
    R: AsRef<Path>,
{
    let (width, height) = resize_dimensions_fill(img.width(), img.height());

    let thumbnail = img.resize_exact(width, height, FilterType::Nearest);
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
