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

use crate::http::users_api::{AuthContext, RequireAuth};
use crate::http::utils::status_error::StatusError;
use crate::http::utils::{file_to_response, write_field_to_file, AxumResult};
use crate::http::AppState;
use crate::model::photo::{Photo, PhotoBase, PhotoBody};
use crate::model::user::{User, PUBLIC_USER_ID};
use crate::utils::{internal_error, primitive_date_time_serde, read_exif};

pub fn router(app_state: AppState) -> Router {
    let user_router = Router::new()
        .route("/", get(photos_list))
        .route("/download/:photo_id", get(download_photo))
        .route("/thumbnail/:photo_id", get(thumbnail_photo))
        .route("/exif/:photo_id", get(get_photo_exif))
        .route("/upload", post(upload_photo))
        .route("/delete/:photo_id", delete(delete_photo))
        .route("/change_location/:photo_id", post(change_photo_location))
        .with_state(app_state.clone())
        .route_layer(RequireAuth::login());

    let public_router = Router::new()
        .route("/", get(public_photos_list))
        .route("/upload", post(public_upload_photo))
        .with_state(app_state)
        .route_layer(RequireAuth::login());

    Router::new()
        .nest("/photos", user_router)
        .nest("/public_photos", public_router)
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct UploadData {
    #[serde(with = "primitive_date_time_serde")]
    time_created: time::PrimitiveDateTime,
    file_size: usize,
    folder_name: Option<String>,
}

fn check_has_access(user: &User, photo: &Photo) -> Result<(), ErrorResponse> {
    if photo.user_id() == &user.id || photo.user_id() == PUBLIC_USER_ID {
        Ok(())
    } else {
        Err(StatusError::new_status(
            "You don't have access to this resource",
            StatusCode::FORBIDDEN,
        ))
    }
}

async fn base_upload_photo(
    AppState {
        storage,
        users_repo: _users_repo,
        photos_repo,
        thumbnail_manager,
    }: AppState,
    user_name: String,
    query: UploadData,
    mut payload: Multipart,
) -> AxumResult<impl IntoResponse> {
    let field = payload
        .next_field()
        .await?
        .ok_or_else(|| StatusError::new_status("Multipart is empty", StatusCode::BAD_REQUEST))?;

    let file_name = field.file_name().unwrap_or_else(|| field.name().unwrap());

    let new_photo_body = PhotoBody::new(
        user_name.clone(),
        file_name.to_string(),
        query.time_created,
        query.file_size as i64,
        query.folder_name.clone(),
    );

    let photo_path = storage.resolve_photo(new_photo_body.partial_path());
    if let Some(parent) = photo_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).await.map_err(internal_error)?;
        }
    }

    info!("Uploading file to {}", photo_path.to_string_lossy());

    match write_field_to_file(field, &photo_path).await {
        Ok(_) => {}
        Err(e) => {
            // Upload failed, delete the file
            let _ = fs::remove_file(photo_path).await;
            return Err(e);
        }
    }

    match photos_repo.insert_photo(&new_photo_body).await {
        Ok(photo) => {
            let thumbnail_path = storage.resolve_thumbnail(photo.partial_thumbnail_path());
            let photo_id = photo.id;

            // Start generating the thumbnail
            task::spawn(async move {
                thumbnail_manager
                    .request_thumbnail(photo_id, photo_path, thumbnail_path)
                    .await;
            });

            Ok(Json(photo))
        }
        Err(e) => {
            // Insertion failed, delete the file
            let _ = fs::remove_file(photo_path).await;
            Err(e)
        }
    }
}

// region Specific User

async fn photos_list(
    State(state): State<AppState>,
    auth: AuthContext,
) -> AxumResult<impl IntoResponse> {
    let user = auth.current_user.ok_or(StatusCode::BAD_REQUEST)?;

    Ok(Json(state.photos_repo.get_photos_by_user(user.id).await?))
}

async fn thumbnail_photo(
    State(state): State<AppState>,
    Path(photo_id): Path<i64>,
    auth: AuthContext,
) -> impl IntoResponse {
    let AppState {
        storage,
        users_repo: _users_repo,
        photos_repo,
        thumbnail_manager,
    } = state;

    let user = auth.current_user.ok_or(StatusCode::BAD_REQUEST)?;
    let photo = photos_repo.get_photo(photo_id).await?;

    check_has_access(&user, &photo)?;

    let photo_path = storage.resolve_photo(photo.partial_path());
    let thumbnail_path = storage.resolve_thumbnail(photo.partial_thumbnail_path());

    let thumbnail_generated = thumbnail_manager
        .request_thumbnail(photo_id, photo_path.clone(), thumbnail_path.clone())
        .await;

    let path = if thumbnail_generated {
        thumbnail_path
    } else {
        error!(
            "Failed to generate thumbnail for photo {}: {}",
            photo_id,
            photo_path.display()
        );
        photo_path
    };

    file_to_response(&path).await
}

async fn download_photo(
    State(state): State<AppState>,
    Path(photo_id): Path<i64>,
    auth: AuthContext,
) -> impl IntoResponse {
    let user = auth.current_user.ok_or(StatusCode::BAD_REQUEST)?;
    let photo = state.photos_repo.get_photo(photo_id).await?;

    check_has_access(&user, &photo)?;

    let photo_path = state.storage.resolve_photo(photo.partial_path());

    file_to_response(&photo_path).await
}

async fn get_photo_exif(
    State(state): State<AppState>,
    Path(photo_id): Path<i64>,
    auth: AuthContext,
) -> impl IntoResponse {
    let user = auth.current_user.ok_or(StatusCode::BAD_REQUEST)?;
    let photo = state.photos_repo.get_photo(photo_id).await?;

    check_has_access(&user, &photo)?;

    let path = state.storage.resolve_photo(photo.partial_path());
    let exif = task::spawn_blocking(move || read_exif(path)).await.unwrap();

    match exif {
        Some(exif) => Ok(Json(exif)),
        None => Err(StatusError::new_status(
            "Exif data not found",
            StatusCode::NOT_FOUND,
        )),
    }
}

async fn upload_photo(
    State(state): State<AppState>,
    Query(query): Query<UploadData>,
    auth: AuthContext,
    payload: Multipart,
) -> impl IntoResponse {
    let user = auth.current_user.ok_or(StatusCode::BAD_REQUEST)?;

    base_upload_photo(state, user.id, query, payload).await
}

async fn delete_photo(
    State(state): State<AppState>,
    Path(photo_id): Path<i64>,
    auth: AuthContext,
) -> impl IntoResponse {
    let user = auth.current_user.ok_or(StatusCode::BAD_REQUEST)?;
    let photo = state.photos_repo.get_photo(photo_id).await?;

    check_has_access(&user, &photo)?;

    let _ = state.storage.delete_file(photo.partial_thumbnail_path());

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
    auth: AuthContext,
) -> AxumResult<impl IntoResponse> {
    let storage = state.storage;
    let user = auth.current_user.ok_or(StatusCode::BAD_REQUEST)?;
    let photo = state.photos_repo.get_photo(photo_id).await?;

    check_has_access(&user, &photo)?;

    let target_user_name = query.target_user_name.unwrap_or(PUBLIC_USER_ID.to_string());

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
        .map_err(|e| {
            StatusError::new_status(
                std::format!("Failed moving the photo: {e}"),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?;

    state
        .photos_repo
        .update_photo(&changed_photo)
        .await
        .map_err(|_| StatusError::create("Something went wrong moving the photo"))?;

    Ok(Json(changed_photo))
}

// endregion Specific User

// region Public

async fn public_photos_list(State(state): State<AppState>) -> AxumResult<impl IntoResponse> {
    Ok(Json(
        state.photos_repo.get_photos_by_user(PUBLIC_USER_ID).await?,
    ))
}

async fn public_upload_photo(
    State(state): State<AppState>,
    Query(query): Query<UploadData>,
    payload: Multipart,
) -> impl IntoResponse {
    base_upload_photo(state, PUBLIC_USER_ID.to_string(), query, payload).await
}

// endregion Public
