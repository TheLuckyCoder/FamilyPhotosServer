use std::path::Path;

pub fn generate_thumbnail<Q, R>(load_path: Q, save_path: R) -> bool
where
    Q: AsRef<Path>,
    R: AsRef<Path>,
{
    if let Ok(img) = image::open(load_path) {
        let thumbnail = img.thumbnail(400, 400);
        return thumbnail.save(save_path).is_ok();
    }

    false
}
