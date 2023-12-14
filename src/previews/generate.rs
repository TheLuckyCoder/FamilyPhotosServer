use std::path::Path;
use std::process::Command;
use std::time::Duration;

use anyhow::Context;
use mime_guess::MimeGuess;
use tracing::{info, warn};
use wait_timeout::ChildExt;

const PREVIEW_TARGET_SIZE: u32 = 400;

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
        .arg(&format!("{PREVIEW_TARGET_SIZE}"));

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

    let mut child = Command::new("convert")
        .arg("-auto-orient")
        .arg(load_path.as_ref())
        .arg("-thumbnail")
        .arg(format!("{PREVIEW_TARGET_SIZE}x{PREVIEW_TARGET_SIZE}^"))
        .arg(save_path.as_ref())
        .spawn()
        .unwrap();

    match child.wait_timeout(Duration::from_secs(5)) {
        Ok(status) => Ok(status.map_or(false, |s| s.success())),
        Err(e) => {
            child.kill().unwrap();
            Err(e)
        }
    }
    .unwrap();

    true
}
