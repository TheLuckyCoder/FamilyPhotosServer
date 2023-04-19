use crate::db::photos_db::{DeletePhoto, GetPhoto, GetPhotos, InsertPhoto, UpdatePhoto};
use crate::db::users_db::GetUser;
use crate::db::{internal_error, Handler, Pool};
use crate::http::status_error::StatusError;
use crate::http::{AppState, AxumResult};
use crate::model::photo::{Photo, PhotoBody};
use crate::model::user::User;
use crate::thumbnail::generate_thumbnail;
use crate::utils::file_storage::FileStorage;
use crate::utils::{primitive_date_time_serde, read_exif};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, post};
use axum::{
    body::StreamBody, extract::Multipart, http::header, response::IntoResponse, Json, Router,
};
use futures_util::{FutureExt, TryStreamExt};
use tokio::io::{AsyncRead, AsyncWriteExt};
use tokio::{fs, task};
use tokio_util::io::ReaderStream;

const PUBLIC_USER_ID: i64 = 1;

pub fn router(app_state: AppState) -> Router {
    let user_router = Router::new()
        .route("/:user_id", get(photos_list))
        .route("/:user_id/download/:photo_id", get(download_photo))
        .route("/:user_id/exif/:photo_id", get(get_photo_exif))
        .route("/:user_id/upload/:photo_id", post(upload_photo))
        .route("/:user_id/delete/:photo_id", delete(delete_photo))
        .route(
            "/:user_id/update_caption/:photo_id",
            post(update_photo_caption),
        )
        .route(
            "/:user_id/change_location/:photo_id",
            post(change_photo_location),
        )
        .with_state(app_state);

    Router::new().nest("/photos", user_router)
}

async fn get_user(pool: &Pool, user_id: i64) -> AxumResult<User> {
    pool.send(GetUser::Id(user_id))
        .await
        .map_err(|e| StatusError::new_status("No such user", StatusCode::NOT_FOUND))
}

async fn get_user_and_photo(pool: &Pool, user_id: i64, photo_id: i64) -> AxumResult<(User, Photo)> {
    let photo = pool
        .send(GetPhoto { id: photo_id })
        .await
        .map_err(|e| StatusError::new_status("No such photo", StatusCode::NOT_FOUND))?;

    Ok((get_user(pool, user_id).await?, photo))
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadData {
    #[serde(with = "primitive_date_time_serde")]
    time_created: time::PrimitiveDateTime,
    file_size: usize,
    folder_name: Option<String>,
}

async fn base_download_photo(
    state: AppState,
    user_id: i64,
    photo_id: i64,
) -> AxumResult<impl IntoResponse> {
    let (user, photo) = get_user_and_photo(&state.pool, user_id, photo_id).await?;

    let photo_path = state
        .storage
        .resolve(photo.partial_path(&user).map_err(StatusError::create)?);

    /* let file = NamedFile::open_async(storage.resolve(photo_path))
    .await
    .map_err(|_| StatusError::create("Could not open photo"))?
    .use_etag(false);*/

    let mime = mime_guess::from_path(&photo_path)
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

    log::debug!("Mime: {}", mime);
    let headers = [
        (header::CONTENT_TYPE, mime),
        (
            header::CONTENT_DISPOSITION,
            format!(
                "attachment; filename=\"{}\"",
                photo_path
                    .file_name()
                    .expect("File has no name")
                    .to_string_lossy()
            ),
        ),
    ];

    Ok((headers, body))
}

/*async fn base_thumbnail_photo(
    state: &NewAppState,
    user_id: i64,
    photo_id: i64,
) -> Result<NamedFile> {
    let db = state.db.clone();
    let storage = state.storage.borrow();

    let (user, photo) = get_user_and_photo(&db, user_id, photo_id).await?;

    let photo_path = storage.resolve(photo.partial_path(&user).map_err(StatusError::create)?);
    let thumbnail_path = storage.resolve_thumbnail(photo.partial_thumbnail_path());
    let photo_path_clone = photo_path.clone();
    let thumbnail_path_clone = thumbnail_path.clone();

    let thumbnail_generated = thumbnail_path.exists()
        || task::spawn_blocking(move || generate_thumbnail(photo_path_clone, thumbnail_path_clone))
            .await?;

    let path = if thumbnail_generated {
        thumbnail_path
    } else {
        log::error!(
            "Failed to generate thumbnail for photo {} - {}",
            photo_id,
            photo_path.display()
        );
        photo_path
    };

    let file = NamedFile::open_async(path)
        .await
        .map_err(|_| StatusError::create("Could not open photo"))?
        .use_etag(false);

    Ok(file)
}*/

async fn base_upload_photo(
    AppState { pool, storage }: AppState,
    user_id: i64,
    query: UploadData,
    mut payload: Multipart,
) -> AxumResult<impl IntoResponse> {
    let user = get_user(&pool, user_id).await?;

    let mut new_photo: Option<PhotoBody> = None;

    while let Some(mut field) = payload.next_field().await? {
        let file_name = field.file_name().unwrap_or_else(|| field.name().unwrap());

        new_photo = Some(PhotoBody {
            owner: user_id,
            name: file_name.to_string(),
            time_created: query.time_created,
            file_size: query.file_size as i64,
            folder: query.folder_name.clone(),
        });

        let folder = match query.folder_name.clone() {
            None => String::new(),
            Some(folder) => folder + "/",
        };

        let filepath = storage.resolve(format!("{}/{}{}", user.user_name, folder, file_name));
        log::info!("Uploading file to {}", filepath.to_string_lossy());

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

    let photo = pool
        .send(InsertPhoto(new_photo.unwrap()))
        .await
        .map_err(|_| StatusError::create("Something went wrong creating the photo"))?;

    if photo.owner != user_id {
        Err(StatusError::new_status(
            format!("Photo does not belong to user {user_id}"),
            StatusCode::BAD_REQUEST,
        ))
    } else {
        Ok(Json(photo))
    }
}

async fn base_update_photo_caption(
    pool: Pool,
    user_id: i64,
    photo_id: i64,
    new_caption: Option<String>,
) -> AxumResult<impl IntoResponse> {
    let (_, photo) = get_user_and_photo(&pool, user_id, photo_id).await?;

    let changed_photo = {
        let mut new = photo.clone();
        new.caption = new_caption;
        new
    };

    let updated_photo = pool
        .send(UpdatePhoto(changed_photo))
        .await
        .map_err(|e| StatusError::create("Failed to update the photo caption: {e}"))?;

    Ok(Json(updated_photo))
}

async fn base_delete_photo(
    AppState { pool, storage }: AppState,
    user_id: i64,
    photo_id: i64,
) -> impl IntoResponse {
    let (user, photo) = get_user_and_photo(&pool, user_id, photo_id).await?;

    let path = photo.partial_path(&user).map_err(StatusError::create)?;

    match storage.delete_file(path) {
        Ok(_) => match pool.send(DeletePhoto { id: photo_id }).await {
            Ok(_count) => Ok("{\"deleted\": true}".to_string()),
            _ => Err(StatusError::create("Failed to remove photo from db")),
        },
        Err(e) => Err(StatusError::create(format!("Failed to delete file: {e}"))),
    }
}

async fn base_get_photo_exif(state: AppState, user_id: i64, photo_id: i64) -> impl IntoResponse {
    let (user, photo) = get_user_and_photo(&state.pool, user_id, photo_id).await?;

    let path = state
        .storage
        .resolve(photo.partial_path(&user).map_err(StatusError::create)?);
    let exif = task::spawn_blocking(move || read_exif(path)).await.unwrap();

    match exif {
        Some(exif) => Ok(Json(exif)),
        None => Err(StatusError::new_status(
            "Exif data not found",
            StatusCode::NOT_FOUND,
        )),
    }
}

// region Specific User

pub async fn photos_list(
    State(state): State<AppState>,
    Path(user_id): Path<i64>,
) -> AxumResult<impl IntoResponse> {
    match state.pool.send(GetPhotos::Owner(user_id)).await {
        Ok(photos) => Ok(Json(photos)),
        Err(e) => Err(StatusError::create(e.to_string())),
    }
}

/*#[get("/{user_id}/thumbnail/{photo_id}")]
pub async fn thumbnail_photo(state: Data<AppState>, path: Path<(i64, i64)>) -> impl IntoResponse {
    let (user_id, photo_id) = path.into_inner();
    base_thumbnail_photo(state.get_ref(), user_id, photo_id).await
}*/

pub async fn download_photo(
    State(state): State<AppState>,
    Path((user_id, photo_id)): Path<(i64, i64)>,
) -> impl IntoResponse {
    base_download_photo(state, user_id, photo_id).await
}

pub async fn get_photo_exif(
    State(state): State<AppState>,
    Path((user_id, photo_id)): Path<(i64, i64)>,
) -> impl IntoResponse {
    base_get_photo_exif(state, user_id, photo_id).await
}

pub async fn upload_photo(
    State(state): State<AppState>,
    Path(user_id): Path<i64>,
    query: Query<UploadData>,
    payload: Multipart,
) -> impl IntoResponse {
    base_upload_photo(state, user_id, query.0, payload).await
}

pub async fn delete_photo(
    State(state): State<AppState>,
    Path((user_id, photo_id)): Path<(i64, i64)>,
) -> impl IntoResponse {
    base_delete_photo(state, user_id, photo_id).await
}

pub async fn update_photo_caption(
    State(state): State<AppState>,
    Path((user_id, photo_id)): Path<(i64, i64)>,
    Query(query): Query<Option<String>>,
) -> impl IntoResponse {
    base_update_photo_caption(state.pool, user_id, photo_id, query).await
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeLocationQuery {
    target_user_id: Option<i64>,
    target_folder_name: Option<String>,
}

pub async fn change_photo_location(
    State(state): State<AppState>,
    Path((user_id, photo_id)): Path<(i64, i64)>,
    query: Query<ChangeLocationQuery>,
) -> AxumResult<impl IntoResponse> {
    let pool = state.pool;
    let storage = state.storage;
    let (user, photo) = get_user_and_photo(&pool, user_id, photo_id).await?;

    let target_user_id = query.target_user_id.unwrap_or(PUBLIC_USER_ID);
    let target_user = get_user(&pool, target_user_id).await?;

    let changed_photo = {
        let mut new = photo.clone();
        new.owner = target_user_id;
        new.folder = query.target_folder_name.clone();
        new
    };

    let source_path = photo.partial_path(&user).map_err(StatusError::create)?;
    let destination_path = changed_photo
        .partial_path(&target_user)
        .map_err(StatusError::create)?;

    storage
        .move_file(&source_path, &destination_path)
        .map_err(|e| {
            StatusError::new_status(
                std::format!("Failed moving the photo: {e}"),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?;

    let updated_photo = pool
        .send(UpdatePhoto(changed_photo))
        .await
        .map_err(|_| StatusError::create("Something went wrong moving the photo"))?;

    log::info!("Moved photo from {source_path} to {destination_path} successfully");

    Ok(Json(updated_photo))
}

// endregion Specific User

// region Public

pub async fn public_photos_list(State(state): State<AppState>) -> AxumResult<impl IntoResponse> {
    match state.pool.send(GetPhotos::Owner(PUBLIC_USER_ID)).await {
        Ok(photos) => Ok(Json(photos)),
        Err(e) => Err(StatusError::create(e.to_string())),
    }
}

/*#[get("/thumbnail/{photo_id}")]
pub async fn public_thumbnail_photo(state: Data<AppState>, photo_id: Path<i64>) -> impl IntoResponse {
    base_thumbnail_photo(state.get_ref(), PUBLIC_USER_ID, photo_id.into_inner()).await
}*/

/*#[get("/download/{photo_id}")]
pub async fn public_download_photo(
    State(state): State<NewAppState>,
    Path(photo_id): Path<i64>,
) -> impl IntoResponse {
    base_download_photo(state, PUBLIC_USER_ID, photo_id).await
}

#[post("/upload")]
pub async fn public_upload_photo(
    State(state): State<NewAppState>,
    query: Query<UploadData>,
    payload: Multipart,
) -> impl IntoResponse {
    base_upload_photo(state.get_ref(), PUBLIC_USER_ID, query.into_inner(), payload).await
}*/
