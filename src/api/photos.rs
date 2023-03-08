use std::borrow::Borrow;
use std::fs::File;
use std::io::Write;

use actix::Addr;
use actix_files::NamedFile;
use actix_multipart::Multipart;
use actix_web::http::StatusCode;
use actix_web::web::{Data, Path, Query};
use actix_web::{delete, get, post, web, HttpResponse, Responder, Result};
use chrono::naive::serde::ts_milliseconds;
use futures_util::TryStreamExt as _;
use serde::Deserialize;

use crate::api::status_error::StatusError;
use crate::db::photos::{DeletePhoto, GetPhoto, GetPhotos, InsertPhoto, UpdatePhoto};
use crate::db::users::GetUser;
use crate::db::DbActor;
use crate::model::photo::{Photo, PhotoBody};
use crate::model::user::User;
use crate::thumbnail::generate_thumbnail;
use crate::utils::read_exif;
use crate::AppState;

const PUBLIC_USER_ID: i64 = 1;

async fn get_user(db: &Addr<DbActor>, user_id: i64) -> Result<User> {
    match db.send(GetUser::Id(user_id)).await {
        Ok(Ok(user)) => Ok(user),
        _ => Err(StatusError::new_status(
            "Could not find user",
            StatusCode::NOT_FOUND,
        )),
    }
}

async fn get_user_and_photo(
    db: &Addr<DbActor>,
    user_id: i64,
    photo_id: i64,
) -> Result<(User, Photo)> {
    let photo: Photo = match db.send(GetPhoto { id: photo_id }).await {
        Ok(Ok(photo)) => photo,
        _ => {
            return Err(StatusError::new_status(
                "Could not find photo",
                StatusCode::NOT_FOUND,
            ))
        }
    };

    Ok((get_user(db, user_id).await?, photo))
}

async fn base_download_photo(state: &AppState, user_id: i64, photo_id: i64) -> Result<NamedFile> {
    let db = state.db.clone();
    let storage = state.storage.borrow();

    let (user, photo) = get_user_and_photo(&db, user_id, photo_id).await?;

    let photo_path = photo.partial_path(&user).map_err(StatusError::create)?;

    let file = NamedFile::open_async(storage.resolve(photo_path))
        .await
        .map_err(|_| StatusError::create("Could not open photo"))?
        .use_etag(false);

    Ok(file)
}

async fn base_thumbnail_photo(state: &AppState, user_id: i64, photo_id: i64) -> Result<NamedFile> {
    let db = state.db.clone();
    let storage = state.storage.borrow();

    let (user, photo) = get_user_and_photo(&db, user_id, photo_id).await?;

    let photo_path = storage.resolve(photo.partial_path(&user).map_err(StatusError::create)?);
    let thumbnail_path = storage.resolve(photo.partial_thumbnail_path());
    let photo_path_clone = photo_path.clone();
    let thumbnail_path_clone = thumbnail_path.clone();

    let thumbnail_generated = thumbnail_path.exists()
        || web::block(move || generate_thumbnail(photo_path_clone, thumbnail_path_clone)).await?;

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
}

async fn base_upload_photo(
    state: &AppState,
    user_id: i64,
    query: UploadData,
    mut payload: Multipart,
) -> Result<HttpResponse> {
    let db = state.db.clone();
    let storage = state.storage.borrow();
    let user = get_user(&db, user_id).await?;

    let mut new_photo: Option<PhotoBody> = None;

    while let Some(mut field) = payload.try_next().await? {
        // A multipart/form-data stream has to contain `content_disposition`
        let content_disposition = field.content_disposition();

        let file_name = content_disposition
            .get_filename()
            .unwrap_or_else(|| content_disposition.get_name().unwrap());

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

        // File::create is blocking operation, use thread pool
        let mut f = web::block(|| File::create(filepath)).await??;

        // Field in turn is stream of *Bytes* object
        while let Some(chunk) = field.try_next().await? {
            // filesystem operations are blocking, we have to use thread pool
            f = web::block(move || f.write_all(&chunk).map(|_| f)).await??;
        }
    }

    match db.send(InsertPhoto(new_photo.unwrap())).await {
        Ok(Ok(photo)) => {
            if photo.owner != user_id {
                Err(StatusError::new_status(
                    format!("Photo does not belong to user {user_id}"),
                    StatusCode::BAD_REQUEST,
                ))
            } else {
                Ok(HttpResponse::Ok().json(photo))
            }
        }
        _ => Err(StatusError::create(
            "Something went wrong creating the photo",
        )),
    }
}

async fn base_update_photo_caption(
    state: &AppState,
    user_id: i64,
    photo_id: i64,
    new_caption: Option<String>,
) -> impl Responder {
    let db = state.db.clone();
    let (_, photo) = get_user_and_photo(&db, user_id, photo_id).await?;

    let changed_photo = {
        let mut new = photo.clone();
        new.caption = new_caption;
        new
    };

    match db.send(UpdatePhoto(changed_photo.clone())).await {
        Ok(Ok(_)) => {}
        _ => {
            return Err(StatusError::create(
                "Something went wrong updating the photo caption",
            ))
        }
    };

    Ok(HttpResponse::Ok().json(changed_photo))
}

async fn base_delete_photo(state: &AppState, user_id: i64, photo_id: i64) -> Result<String> {
    let db = state.db.clone();
    let storage = state.storage.borrow();

    let (user, photo) = get_user_and_photo(&db, user_id, photo_id).await?;

    if storage.delete_file(photo.partial_path(&user).map_err(StatusError::create)?) {
        match db.send(DeletePhoto { id: photo_id }).await {
            Ok(Ok(_count)) => Ok("{\"deleted\": true}".to_string()),
            _ => Err(StatusError::create("Failed to remove photo from database")),
        }
    } else {
        Err(StatusError::create("File could not be deleted"))
    }
}

async fn base_get_photo_exif(state: &AppState, user_id: i64, photo_id: i64) -> impl Responder {
    let db = state.db.clone();
    let storage = state.storage.borrow();

    let (user, photo) = get_user_and_photo(&db, user_id, photo_id).await?;

    let path = storage.resolve(photo.partial_path(&user).map_err(StatusError::create)?);
    let exif = web::block(move || read_exif(path)).await?;

    match exif {
        Some(exif) => Ok(HttpResponse::Ok().json(exif)),
        None => Err(StatusError::new_status(
            "Exif data not found",
            StatusCode::NOT_FOUND,
        )),
    }
}

// region Specific User

#[get("/{user_id}")]
pub async fn photos_list(state: Data<AppState>, user_id: Path<i64>) -> impl Responder {
    let db = state.get_ref().db.clone();

    match db.send(GetPhotos::Owner(user_id.into_inner())).await {
        Ok(Ok(photos)) => HttpResponse::Ok().json(photos),
        _ => HttpResponse::InternalServerError().json("Something went wrong"),
    }
}

#[get("/{user_id}/thumbnail/{photo_id}")]
pub async fn thumbnail_photo(state: Data<AppState>, path: Path<(i64, i64)>) -> impl Responder {
    let (user_id, photo_id) = path.into_inner();
    base_thumbnail_photo(state.get_ref(), user_id, photo_id).await
}

#[get("/{user_id}/download/{photo_id}")]
pub async fn download_photo(state: Data<AppState>, path: Path<(i64, i64)>) -> impl Responder {
    let (user_id, photo_id) = path.into_inner();
    base_download_photo(state.get_ref(), user_id, photo_id).await
}

#[get("/{user_id}/exif/{photo_id}")]
pub async fn get_photo_exif(state: Data<AppState>, path: Path<(i64, i64)>) -> impl Responder {
    let (user_id, photo_id) = path.into_inner();
    base_get_photo_exif(state.get_ref(), user_id, photo_id).await
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadData {
    #[serde(with = "ts_milliseconds")]
    time_created: chrono::NaiveDateTime,
    file_size: usize,
    folder_name: Option<String>,
}

#[post("/{user_id_path}/upload")]
pub async fn upload_photo(
    state: Data<AppState>,
    user_id_path: Path<i64>,
    query: Query<UploadData>,
    payload: Multipart,
) -> impl Responder {
    base_upload_photo(state.get_ref(), user_id_path.into_inner(), query.0, payload).await
}

#[delete("/{user_id}/delete/{photo_id}")]
pub async fn delete_photo(state: Data<AppState>, path: Path<(i64, i64)>) -> impl Responder {
    let (user_id, photo_id) = path.into_inner();
    base_delete_photo(state.get_ref(), user_id, photo_id).await
}

#[post("/{user_id}/update_caption/{photo_id}")]
pub async fn update_photo_caption(
    state: Data<AppState>,
    path: Path<(i64, i64)>,
    query: Query<Option<String>>,
) -> impl Responder {
    let (user_id, photo_id) = path.into_inner();
    base_update_photo_caption(state.get_ref(), user_id, photo_id, query.into_inner()).await
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeLocationQuery {
    target_user_id: Option<i64>,
    target_folder_name: Option<String>,
}

#[post("/{user_id_path}/change_location/{photo_id_path}")]
pub async fn change_photo_location(
    state: Data<AppState>,
    path: Path<(i64, i64)>,
    query: Query<ChangeLocationQuery>,
) -> impl Responder {
    let db = state.get_ref().db.clone();
    let storage = state.get_ref().storage.borrow();
    let (user_id, photo_id) = path.into_inner();
    let (user, photo) = get_user_and_photo(&db, user_id, photo_id).await?;

    let changed_photo = {
        let mut new = photo.clone();
        new.owner = query.target_user_id.unwrap_or(PUBLIC_USER_ID);
        new.folder = query.target_folder_name.clone();
        new
    };

    storage.move_file(
        photo.partial_path(&user).map_err(StatusError::create)?,
        changed_photo
            .partial_path(&user)
            .map_err(StatusError::create)?,
    );

    match db.send(UpdatePhoto(changed_photo.clone())).await {
        Ok(Ok(_)) => {}
        _ => {
            return Err(StatusError::create(
                "Something went wrong updating the photo",
            ))
        }
    };

    if storage.delete_file(photo.partial_path(&user).map_err(StatusError::create)?) {
        Ok(HttpResponse::Ok().json(changed_photo))
    } else {
        Err(StatusError::create("File could not be deleted"))
    }
}

// endregion Specific User

// region Public

#[get("")]
pub async fn public_photos_list(state: Data<AppState>) -> impl Responder {
    let db = state.get_ref().db.clone();

    match db.send(GetPhotos::Owner(PUBLIC_USER_ID)).await {
        Ok(Ok(photos)) => HttpResponse::Ok().json(photos),
        _ => HttpResponse::InternalServerError().json("Something went wrong"),
    }
}

#[get("/thumbnail/{photo_id}")]
pub async fn public_thumbnail_photo(state: Data<AppState>, photo_id: Path<i64>) -> impl Responder {
    base_thumbnail_photo(state.get_ref(), PUBLIC_USER_ID, photo_id.into_inner()).await
}

#[get("/download/{photo_id}")]
pub async fn public_download_photo(state: Data<AppState>, photo_id: Path<i64>) -> impl Responder {
    base_download_photo(state.get_ref(), PUBLIC_USER_ID, photo_id.into_inner()).await
}

#[post("/upload")]
pub async fn public_upload_photo(
    state: Data<AppState>,
    query: Query<UploadData>,
    payload: Multipart,
) -> impl Responder {
    base_upload_photo(state.get_ref(), PUBLIC_USER_ID, query.into_inner(), payload).await
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCaptionQuery {
    new_caption: Option<String>,
}

#[post("/update_caption/{photo_id}")]
pub async fn public_update_photo_caption(
    state: Data<AppState>,
    path: Path<i64>,
    query: Query<UpdateCaptionQuery>,
) -> impl Responder {
    base_update_photo_caption(
        state.get_ref(),
        PUBLIC_USER_ID,
        path.into_inner(),
        query.into_inner().new_caption,
    )
    .await
}

#[delete("/delete/{photo_id}")]
pub async fn public_delete_photo(state: Data<AppState>, photo_id: Path<i64>) -> impl Responder {
    base_delete_photo(state.get_ref(), PUBLIC_USER_ID, photo_id.into_inner()).await
}

// endregion Public
