use axum::body::Body;
use axum::extract::multipart;
use axum::http::header;
use axum::response::IntoResponse;
use futures_util::TryStreamExt;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio_util::io::ReaderStream;
use tracing::{debug, error};

use crate::http::utils::status_error::StatusError;
use crate::repo::users_repo::UsersRepository;
use crate::utils::internal_error;

pub mod status_error;

pub type AxumResult<T> = axum::response::Result<T>;

pub type AuthSession = axum_login::AuthSession<UsersRepository>;

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
    let body = Body::from_stream(stream);

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

///
/// Returns the amount of bytes written to disk
///
pub async fn write_field_to_file<'a, 'b>(
    mut field: multipart::Field<'a>,
    file_path: &'b std::path::Path,
) -> AxumResult<usize> {
    let mut file = fs::File::create(file_path).await.map_err(|e| {
        error!("Failed creating photo file: {e}");
        StatusError::create("Failed creating photo file")
    })?;

    let mut file_size = 0;

    while let Some(chunk) = field.try_next().await? {
        file_size += chunk.len();
        file.write_all(&chunk).await.map_err(internal_error)?;
    }

    Ok(file_size)
}
