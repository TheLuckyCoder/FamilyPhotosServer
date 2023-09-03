use axum::body::StreamBody;
use axum::extract::multipart;
use axum::http::header;
use axum::response::IntoResponse;
use futures_util::TryStreamExt;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio_util::io::ReaderStream;
use tracing::debug;

use crate::http::utils::status_error::StatusError;
use crate::utils::internal_error;

pub mod status_error;

pub type AxumResult<T> = axum::response::Result<T>;

pub async fn file_to_response(photo_path: &std::path::Path) -> AxumResult<impl IntoResponse> {
    let mime = mime_guess::from_path(photo_path)
        .first_or_octet_stream()
        .as_ref()
        .to_string();

    let stream = ReaderStream::new(
        fs::File::open(&photo_path)
            .await
            .map_err(|e| StatusError::create(e.to_string()))?,
    );
    // convert the `Stream` into an `axum::body::HttpBody`
    let body = StreamBody::new(stream);

    debug!("Mime: {}", mime);
    let headers = [
        (header::CONTENT_TYPE, mime),
        (
            header::CONTENT_DISPOSITION,
            format!(
                "attachment; filename=\"{}\"",
                photo_path
                    .file_name()
                    .expect("Photo must have a name")
                    .to_string_lossy()
            ),
        ),
    ];

    Ok((headers, body))
}

pub async fn write_field_to_file<'a, 'b>(
    mut field: multipart::Field<'a>,
    file_path: &'b std::path::Path,
) -> AxumResult<impl IntoResponse> {
    let mut file = fs::File::create(file_path)
        .await
        .map_err(|_| StatusError::create("Failed creating photo file"))?;

    while let Some(chunk) = field.try_next().await? {
        file.write_all(&chunk).await.map_err(internal_error)?;
    }

    Ok(())
}
