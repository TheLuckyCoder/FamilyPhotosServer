use crate::db::photos_db::GetPhoto;
use crate::db::users_db::GetUser;
use crate::db::{Handler, Pool};
use crate::http::status_error::StatusError;
use crate::model::photo::Photo;
use crate::model::user::User;
use axum::body::StreamBody;
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use tokio::fs;
use tokio_util::io::ReaderStream;
use tracing::debug;

pub type AxumResult<T> = axum::response::Result<T>;

pub async fn get_user(pool: &Pool, user_id: i64) -> AxumResult<User> {
    pool.send(GetUser::Id(user_id))
        .await
        .map_err(|_| StatusError::new_status("No such user", StatusCode::NOT_FOUND))
}

pub async fn get_user_and_photo(
    pool: &Pool,
    user_id: i64,
    photo_id: i64,
) -> AxumResult<(User, Photo)> {
    let photo = pool
        .send(GetPhoto { id: photo_id })
        .await
        .map_err(|_| StatusError::new_status("No such photo", StatusCode::NOT_FOUND))?;

    Ok((get_user(pool, user_id).await?, photo))
}

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
