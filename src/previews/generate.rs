use std::cmp::max;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use anyhow::Context;
use exif::{In, Tag};
use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::DynamicImage;
use mime_guess::MimeGuess;
use tracing::{error, info, warn};
use wait_timeout::ChildExt;

const PREVIEW_TARGET_SIZE: u32 = 500;

fn generate_heic_preview(load_path: &Path, save_path: &Path) -> anyhow::Result<bool> {
    let mut child = Command::new("heif-previewer")
        .arg("-s")
        .arg(stringify!(PREVIEW_TARGET_SIZE))
        .arg(load_path)
        .arg(save_path)
        .spawn()
        .context("Failed to start heif-previewer command")?;

    match child.wait_timeout(Duration::from_secs(5)) {
        Ok(status) => Ok(status.map_or(false, |s| s.success())),
        Err(e) => {
            child.kill()?;
            Err(e).context("Error while rung heif-previwer")
        }
    }
}

fn generate_video_frame<P: AsRef<Path>, R: AsRef<Path>>(
    load_path: P,
    save_path: R,
) -> anyhow::Result<()> {
    let mut command = Command::new("ffmpegthumbnailer");
    command
        .arg("-i")
        .arg(load_path.as_ref())
        .arg("-o")
        .arg(save_path.as_ref())
        .arg("-s")
        .arg(stringify!(PREVIEW_TARGET_SIZE));

    let mut child = command.spawn()?;

    match child.wait_timeout(Duration::from_secs(15)) {
        Ok(status) => status.map(|_| ()).context("Failed to get exit status")?,
        Err(e) => {
            child.kill()?;
            return Err(e).context("ffmpegthumbnailer run error");
        }
    }

    Ok(())
}

pub fn generate_preview<P, R>(load_path: P, save_path: R) -> bool
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
                "Generated preview for video: {}",
                load_path.as_ref().display()
            ),
            Err(error) => warn!(
                "Preview generation failed for video: {}\nCause: {error}",
                load_path.as_ref().display()
            ),
        }

        return result.is_ok();
    }

    if ext == "heic" || ext == "heif" {
        return match generate_heic_preview(load_path.as_ref(), save_path.as_ref()) {
            Ok(result) => result,
            Err(e) => {
                error!("Error generating heic/heif preview: {e}");
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

    let file = File::open(path).ok()?;
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

    let mut preview = img.resize_exact(width, height, FilterType::Nearest);
    match orientation {
        Some(3) => preview = preview.rotate180(),
        Some(6) => preview = preview.rotate90(),
        Some(8) => preview = preview.rotate270(),
        _ => {}
    };

    match save_path.as_ref().to_string_lossy().rsplit_once('.') {
        None => false,
        Some((_before, after)) => {
            if after == "jpg" || after == "jpeg" {
                let file = File::create(save_path).unwrap();
                let writer = BufWriter::new(file);
                JpegEncoder::new_with_quality(writer, 70)
                    .encode_image(&preview)
                    .is_ok()
            } else {
                preview.save(save_path).is_ok()
            }
        }
    }
}

#[inline]
fn resize_dimensions_fill(width: u32, height: u32) -> (u32, u32) {
    let wratio = PREVIEW_TARGET_SIZE as f64 / width as f64;
    let hratio = PREVIEW_TARGET_SIZE as f64 / height as f64;
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
