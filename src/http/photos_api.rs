use crate::http::status_error::StatusError;
use crate::http::users_api::{AuthContext, RequireAuth};
use crate::http::utils::{file_to_response, AxumResult};
use crate::http::AppState;
use crate::model::photo::PhotoBody;
use crate::thumbnail::generate_thumbnail;
use crate::utils::{internal_error, primitive_date_time_serde, read_exif};
use axum::{
    extract::Multipart,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use futures_util::TryStreamExt;
use std::string::ToString;
use tokio::io::AsyncWriteExt;
use tokio::{fs, task};
use tracing::{error, info};

const PUBLIC_USER_NAME: &str = "public";

pub fn router(app_state: AppState) -> Router {
    let user_router = Router::new()
        .route("/", get(photos_list))
        .route("/download/:photo_id", get(download_photo))
        .route("/thumbnail/:photo_id", get(thumbnail_photo))
        .route("/exif/:photo_id", get(get_photo_exif))
        .route("/upload", post(upload_photo))
        .route("/delete/:photo_id", delete(delete_photo))
        .route("/change_location/:photo_id", post(change_photo_location))
        .with_state(app_state.clone());

    let public_router = Router::new()
        .route("/", get(public_photos_list))
        .route("/upload", post(public_upload_photo))
        .with_state(app_state);

    Router::new()
        .nest("/photos", user_router)
        .nest("/public_photos", public_router)
        .route_layer(RequireAuth::login())
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadData {
    #[serde(with = "primitive_date_time_serde")]
    time_created: time::PrimitiveDateTime,
    file_size: usize,
    folder_name: Option<String>,
}

async fn base_upload_photo(
    AppState {
        storage,
        users_repo: _users_repo,
        photos_repo,
    }: AppState,
    user_name: String,
    query: UploadData,
    mut payload: Multipart,
) -> AxumResult<impl IntoResponse> {
    let mut new_photo: Option<PhotoBody> = None;

    while let Some(mut field) = payload.next_field().await? {
        let file_name = field.file_name().unwrap_or_else(|| field.name().unwrap());

        new_photo = Some(PhotoBody {
            user_name: user_name.clone(),
            name: file_name.to_string(),
            created_at: query.time_created,
            file_size: query.file_size as i64,
            folder: query.folder_name.clone(),
        });

        let folder = match query.folder_name.clone() {
            None => String::new(),
            Some(folder) => folder + "/",
        };

        let filepath = storage.resolve(format!("{}/{}{}", user_name, folder, file_name));
        info!("Uploading file to {}", filepath.to_string_lossy());

        let mut file = fs::File::create(filepath)
            .await
            .map_err(|_| StatusError::create("Failed creating photo file"))?;

        // Field in turn is stream of *Bytes* object
        while let Some(chunk) = field.try_next().await? {
            // filesystem operations are blocking, we have to use thread pool
            file = file
                .write_all(&chunk)
                .await
                .map(|_| file)
                .map_err(internal_error)?;
        }
    }

    let photo = photos_repo.insert_photo(&new_photo.unwrap()).await?;

    Ok(Json(photo))
}

// region Specific User

pub async fn photos_list(
    State(state): State<AppState>,
    auth: AuthContext,
) -> AxumResult<impl IntoResponse> {
    let user = auth.current_user.ok_or(StatusCode::BAD_REQUEST)?;

    Ok(Json(state.photos_repo.get_photos_by_user(user.id).await?))
}

pub async fn thumbnail_photo(
    State(state): State<AppState>,
    Path(photo_id): Path<i64>,
    auth: AuthContext,
) -> impl IntoResponse {
    auth.current_user.ok_or(StatusCode::BAD_REQUEST)?;
    let AppState {
        storage,
        users_repo: _users_repo,
        photos_repo,
    } = state;

    let photo = photos_repo.get_photo(photo_id).await?;

    let photo_path = storage.resolve(photo.partial_path().map_err(StatusError::create)?);
    let thumbnail_path = storage.resolve_thumbnail(photo.partial_thumbnail_path());
    let photo_path_clone = photo_path.clone();
    let thumbnail_path_clone = thumbnail_path.clone();

    let thumbnail_generated = thumbnail_path.exists()
        || task::spawn_blocking(move || generate_thumbnail(photo_path_clone, thumbnail_path_clone))
            .await
            .map_err(internal_error)?;

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

pub async fn download_photo(
    State(state): State<AppState>,
    Path(photo_id): Path<i64>,
    auth: AuthContext,
) -> impl IntoResponse {
    auth.current_user.ok_or(StatusCode::BAD_REQUEST)?;

    let photo = state.photos_repo.get_photo(photo_id).await?;

    let photo_path = state
        .storage
        .resolve(photo.partial_path().map_err(StatusError::create)?);

    file_to_response(&photo_path).await
}

pub async fn get_photo_exif(
    State(state): State<AppState>,
    Path(photo_id): Path<i64>,
    auth: AuthContext,
) -> impl IntoResponse {
    auth.current_user.ok_or(StatusCode::BAD_REQUEST)?;

    let photo = state.photos_repo.get_photo(photo_id).await?;

    let path = state
        .storage
        .resolve(photo.partial_path().map_err(StatusError::create)?);
    let exif = task::spawn_blocking(move || read_exif(path)).await.unwrap();

    match exif {
        Some(exif) => Ok(Json(exif)),
        None => Err(StatusError::new_status(
            "Exif data not found",
            StatusCode::NOT_FOUND,
        )),
    }
}

pub async fn upload_photo(
    State(state): State<AppState>,
    Query(query): Query<UploadData>,
    auth: AuthContext,
    payload: Multipart,
) -> impl IntoResponse {
    let user = auth.current_user.ok_or(StatusCode::BAD_REQUEST)?;

    base_upload_photo(state, user.id, query, payload).await
}

pub async fn delete_photo(
    State(state): State<AppState>,
    Path(photo_id): Path<i64>,
) -> impl IntoResponse {
    let photo = state.photos_repo.get_photo(photo_id).await?;
    let path = photo.partial_path().map_err(StatusError::create)?;

    match state.storage.delete_file(path) {
        Ok(_) => match state.photos_repo.delete_photo(photo_id).await {
            Ok(_count) => Ok("{\"deleted\": true}".to_string()),
            _ => Err(StatusError::create("Failed to remove photo from database")),
        },
        Err(e) => Err(StatusError::create(format!("Failed to delete file: {e}"))),
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeLocationQuery {
    target_user_name: Option<String>,
    target_folder_name: Option<String>,
}

pub async fn change_photo_location(
    State(state): State<AppState>,
    Path(photo_id): Path<i64>,
    Query(query): Query<ChangeLocationQuery>,
    auth: AuthContext,
) -> AxumResult<impl IntoResponse> {
    auth.current_user.ok_or(StatusCode::BAD_REQUEST)?;

    let storage = state.storage;
    let photo = state.photos_repo.get_photo(photo_id).await?;

    let target_user_name = query
        .target_user_name
        .unwrap_or(PUBLIC_USER_NAME.to_string());

    let changed_photo = {
        let mut new = photo.clone();
        new.user_name = target_user_name;
        new.folder = query.target_folder_name.clone();
        new
    };

    let source_path = photo.partial_path().map_err(StatusError::create)?;
    let destination_path = changed_photo.partial_path().map_err(StatusError::create)?;

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

    info!("Moved photo from {source_path} to {destination_path} successfully");

    Ok(Json(changed_photo))
}

// endregion Specific User

// region Public

pub async fn public_photos_list(
    State(state): State<AppState>,
    auth: AuthContext,
) -> AxumResult<impl IntoResponse> {
    auth.current_user.ok_or(StatusCode::BAD_REQUEST)?;

    Ok(Json(
        state
            .photos_repo
            .get_photos_by_user(PUBLIC_USER_NAME)
            .await?,
    ))
}

pub async fn public_upload_photo(
    State(state): State<AppState>,
    Query(query): Query<UploadData>,
    payload: Multipart,
) -> impl IntoResponse {
    base_upload_photo(state, PUBLIC_USER_NAME.to_string(), query, payload).await
}

// endregion Public
