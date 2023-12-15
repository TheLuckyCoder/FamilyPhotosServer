use std::path::Path;
use std::process::Command;
use std::time::Duration;

use anyhow::Context;
use mime_guess::MimeGuess;
use wait_timeout::ChildExt;

const PREVIEW_TARGET_SIZE: u32 = 300;
const VIDEO_PREVIEW_TARGET_SIZE: &str = "500";

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
        .arg(VIDEO_PREVIEW_TARGET_SIZE);

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

pub fn generate_preview<P, R>(load_path: P, save_path: R) -> anyhow::Result<()>
where
    P: AsRef<Path>,
    R: AsRef<Path>,
{
    let ext = load_path
        .as_ref()
        .extension()
        .context("Path has no extension")?
        .to_ascii_lowercase();

    let mime =
        MimeGuess::from_ext(ext.to_str().context("Invalid exception")?).first_or_octet_stream();

    if mime.type_() == "video" {
        return generate_video_frame(&load_path, &save_path);
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
        Ok(status) => status.map(|_| ()).context("Failed to get exit status"),
        Err(e) => {
            child.kill()?;
            Err(e).context("ImageMagick run error")
        }
    }
}
