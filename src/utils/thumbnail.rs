use std::cmp::max;
use std::path::Path;
use std::process::Command;

use actix_files::file_extension_to_mime;
use image::imageops::FilterType;
use image::{DynamicImage, RgbImage};
use libheif_rs::{ColorSpace, HeifContext, RgbChroma};

const TARGET_WIDTH: u32 = 400;
const TARGET_HEIGHT: u32 = 400;

fn read_heic_image<P: AsRef<Path>>(path: P) -> Option<DynamicImage> {
    let ctx = HeifContext::read_from_file(path.as_ref().to_str()?).ok()?;
    let handle = ctx.primary_image_handle().ok()?;
    let decoded_image = handle.decode(ColorSpace::Rgb(RgbChroma::Rgb), false).ok()?;
    let plane = decoded_image.planes().interleaved?;

    let mut buffer = RgbImage::new(handle.width(), handle.height());
    buffer.copy_from_slice(plane.data);
    Some(DynamicImage::from(buffer))
}

fn generate_video_frame<P: AsRef<Path>, R: AsRef<Path>>(load_path: P, save_path: R) -> bool {
    let intermediate_path = load_path
        .as_ref()
        .to_str()
        .unwrap()
        .rsplit_once('0')
        .map(|(before, _after)| before.to_string() + ".jpeg")
        .unwrap();

    let mut child = Command::new("ffmpegthumbnailer")
        .arg("-i")
        .arg(load_path.as_ref().as_os_str())
        .arg("-o")
        .arg(&intermediate_path)
        .arg("-s")
        .arg(TARGET_WIDTH.to_string())
        .arg("-a") // Make it square
        .spawn()
        .unwrap();

    child.wait().expect("Failed to wait on ffmpegthumbnailer");

    if let Ok(img) = image::open(intermediate_path) {
        save_image(&save_path, img)
    } else {
        false
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
        generate_video_frame(&load_path, &save_path);
    }

    if ext == "heic" || ext == "heif" {
        if let Some(img) = read_heic_image(&load_path) {
            return save_image(&save_path, img);
        }
        return false;
    }

    if let Ok(img) = image::open(&load_path) {
        return save_image(&save_path, img);
    }

    false
}

fn save_image<R>(save_path: R, img: DynamicImage) -> bool
where
    R: AsRef<Path>,
{
    let (width2, height2) =
        resize_dimensions_fill(img.width(), img.height(), TARGET_WIDTH, TARGET_HEIGHT);

    let thumbnail = img.resize_exact(width2, height2, FilterType::Nearest);
    /*let (iwidth, iheight) = intermediate.dimensions();
    let ratio = u64::from(iwidth) * u64::from(NHEIGHT);
    let nratio = u64::from(NWIDTH) * u64::from(iheight);

    if nratio > ratio {
        intermediate.crop(0, (iheight - NHEIGHT) / 2, NWIDTH, NHEIGHT)
    } else {
        intermediate.crop((iwidth - NWIDTH) / 2, 0, NWIDTH, NHEIGHT)
    }*/
    // let thumbnail = img.resize_to_fill(400, 400, FilterType::Lanczos3);
    thumbnail.save(save_path).is_ok()
}

#[inline]
fn resize_dimensions_fill(width: u32, height: u32, nwidth: u32, nheight: u32) -> (u32, u32) {
    let wratio = nwidth as f64 / width as f64;
    let hratio = nheight as f64 / height as f64;
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
