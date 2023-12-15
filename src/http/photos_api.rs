use std::string::ToString;

use axum::response::ErrorResponse;
use axum::{
    extract::Multipart,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use tokio::{fs, task};
use tracing::{error, info};

use crate::http::utils::status_error::StatusError;
use crate::http::utils::{file_to_response, write_field_to_file, AuthSession, AxumResult};
use crate::http::AppState;
use crate::model::photo::{Photo, PhotoBase, PhotoBody};
use crate::model::user::{User, PUBLIC_USER_ID};
use crate::previews;
use crate::utils::{internal_error, primitive_date_time_serde, read_exif};

pub fn router(app_state: AppState) -> Router {
    Router::new()
        .route("/", get(photos_list))
        .route("/download/:photo_id", get(download_photo))
        .route("/preview/:photo_id", get(preview_photo))
        .route("/exif/:photo_id", get(get_photo_exif))
        .route("/upload", post(upload_photo))
        .route("/delete/:photo_id", delete(delete_photo))
        .route("/change_location/:photo_id", post(change_photo_location))
        .route("/favorite/:photo_id", post(add_favorite))
        .route("/favorite/:photo_id", delete(delete_favorite))
        .with_state(app_state)
}

fn check_has_access(user: Option<User>, photo: &Photo) -> Result<User, ErrorResponse> {
    let user = user.ok_or(StatusCode::UNAUTHORIZED)?;

    if photo.user_id() == &user.id || photo.user_id() == PUBLIC_USER_ID {
        Ok(user)
    } else {
        Err(StatusError::new_status(
            "You don't have access to this resource",
            StatusCode::FORBIDDEN,
        ))
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PhotosListQuery {
    #[serde(default)]
    public: bool,
}

async fn photos_list(
    State(state): State<AppState>,
    Query(query): Query<PhotosListQuery>,
    auth: AuthSession,
) -> AxumResult<impl IntoResponse> {
    let user = auth.user.ok_or(StatusCode::BAD_REQUEST)?;

    let user_id = if query.public {
        PUBLIC_USER_ID
    } else {
        user.id.as_str()
    };

    Ok(Json(state.photos_repo.get_photos_by_user(user_id).await?))
}

async fn preview_photo(
    State(state): State<AppState>,
    Path(photo_id): Path<i64>,
    auth: AuthSession,
) -> impl IntoResponse {
    let AppState {
        storage,
        users_repo: _users_repo,
        photos_repo,
    } = state;

    let photo = photos_repo.get_photo(photo_id).await?;
    check_has_access(auth.user, &photo)?;

    let photo_path = storage.resolve_photo(photo.partial_path());
    let preview_path = storage.resolve_preview(photo.partial_preview_path());

    let photo_path_clone = photo_path.clone();
    let preview_path_clone = preview_path.clone();
    let preview_generated = task::spawn_blocking(move || {
        previews::generate_preview(photo_path_clone, preview_path_clone)
    })
    .await
    .unwrap();

    let path = match preview_generated {
        Ok(_) => preview_path,
        Err(e) => {
            error!(
                "Preview generation failed for video: {}\nCause: {e}",
                photo_path.display()
            );
            photo_path
        }
    };

    file_to_response(&path).await
}

async fn download_photo(
    State(state): State<AppState>,
    Path(photo_id): Path<i64>,
    auth: AuthSession,
) -> impl IntoResponse {
    let photo = state.photos_repo.get_photo(photo_id).await?;
    check_has_access(auth.user, &photo)?;

    let photo_path = state.storage.resolve_photo(photo.partial_path());

    file_to_response(&photo_path).await
}

async fn get_photo_exif(
    State(state): State<AppState>,
    Path(photo_id): Path<i64>,
    auth: AuthSession,
) -> impl IntoResponse {
    let photo = state.photos_repo.get_photo(photo_id).await?;
    check_has_access(auth.user, &photo)?;

    let path = state.storage.resolve_photo(photo.partial_path());
    let exif = task::spawn_blocking(move || read_exif(path))
        .await
        .map_err(internal_error)?;

    match exif {
        Some(exif) => Ok(Json(exif)),
        None => Err(StatusError::new_status(
            "Exif data not found",
            StatusCode::NOT_FOUND,
        )),
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct UploadDataQuery {
    #[serde(with = "primitive_date_time_serde")]
    time_created: time::PrimitiveDateTime,
    folder_name: Option<String>,
    #[serde(default)]
    make_public: bool,
}

async fn upload_photo(
    State(state): State<AppState>,
    Query(query): Query<UploadDataQuery>,
    auth: AuthSession,
    mut payload: Multipart,
) -> AxumResult<impl IntoResponse> {
    let user = auth.user.ok_or(StatusCode::UNAUTHORIZED)?;

    let field = payload
        .next_field()
        .await?
        .ok_or_else(|| StatusError::new_status("Multipart is empty", StatusCode::BAD_REQUEST))?;

    let file_name = field
        .file_name()
        .or(field.name())
        .ok_or_else(|| StatusError::new_status("Multipart has no name", StatusCode::BAD_REQUEST))?;

    let mut new_photo_body = PhotoBody::new(
        if query.make_public {
            String::from(PUBLIC_USER_ID)
        } else {
            user.id
        },
        String::from(file_name),
        query.time_created,
        0, // To be set after it is written to disk
        query.folder_name,
    );

    let photo_path = state.storage.resolve_photo(new_photo_body.partial_path());
    if let Some(parent) = photo_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).await.map_err(internal_error)?;
        }
    }

    info!("Uploading file to {}", photo_path.display());

    match write_field_to_file(field, &photo_path).await {
        Ok(file_size) => new_photo_body.set_file_size(file_size as i64),
        Err(e) => {
            // Upload failed, delete the file
            let _ = fs::remove_file(photo_path).await;
            return Err(e);
        }
    }

    match state.photos_repo.insert_photo(&new_photo_body).await {
        Ok(photo) => Ok(Json(photo)),
        Err(e) => {
            // Insertion failed, delete the file
            let _ = fs::remove_file(photo_path).await;
            Err(e)
        }
    }
}

async fn delete_photo(
    State(state): State<AppState>,
    Path(photo_id): Path<i64>,
    auth: AuthSession,
) -> impl IntoResponse {
    let photo = state.photos_repo.get_photo(photo_id).await?;
    check_has_access(auth.user, &photo)?;

    let _ = state.storage.delete_file(photo.partial_preview_path());

    match state.storage.delete_file(photo.partial_path()) {
        Ok(_) => match state.photos_repo.delete_photo(photo_id).await {
            Ok(_count) => Ok("{\"deleted\": true}".to_string()),
            _ => Err(StatusError::create("Failed to remove photo from database")),
        },
        Err(e) => Err(StatusError::create(format!("Failed to delete file: {e}"))),
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChangeLocationQuery {
    target_user_name: Option<String>,
    target_folder_name: Option<String>,
}

async fn change_photo_location(
    State(state): State<AppState>,
    Path(photo_id): Path<i64>,
    Query(query): Query<ChangeLocationQuery>,
    auth: AuthSession,
) -> AxumResult<impl IntoResponse> {
    let storage = state.storage;
    let photo = state.photos_repo.get_photo(photo_id).await?;
    check_has_access(auth.user, &photo)?;

    let target_user_name = query
        .target_user_name
        .unwrap_or(String::from(PUBLIC_USER_ID));

    let changed_photo = Photo {
        id: photo.id(),
        user_id: target_user_name,
        name: photo.name().clone(),
        created_at: photo.created_at(),
        file_size: photo.file_size(),
        folder: query.target_folder_name.clone(),
    };

    let source_path = photo.partial_path();
    let destination_path = changed_photo.partial_path();

    info!("Moving photo from {source_path} to {destination_path}");

    storage
        .move_file(&source_path, &destination_path)
        .map_err(|e| StatusError::create(format!("Failed moving the photo: {e}")))?;

    state
        .photos_repo
        .update_photo(&changed_photo)
        .await
        .map_err(|_| StatusError::create("Something went wrong moving the photo"))?;

    Ok(Json(changed_photo))
}

async fn add_favorite(
    State(state): State<AppState>,
    Path(photo_id): Path<i64>,
    auth_session: AuthSession,
) -> AxumResult<impl IntoResponse> {
    let photo = state.photos_repo.get_photo(photo_id).await?;
    let user = check_has_access(auth_session.user, &photo)?;

    state.photos_repo.insert_favorite(photo_id, user.id).await
}

async fn delete_favorite(
    State(state): State<AppState>,
    Path(photo_id): Path<i64>,
    auth_session: AuthSession,
) -> AxumResult<impl IntoResponse> {
    let photo = state.photos_repo.get_photo(photo_id).await?;
    let user = check_has_access(auth_session.user, &photo)?;

    state.photos_repo.delete_favorite(photo_id, user.id).await
}
