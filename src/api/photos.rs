use actix_files::NamedFile;
use std::borrow::Borrow;
use std::fmt;
use std::fmt::Formatter;
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path::Display;

use actix_multipart::Multipart;
use actix_web::http::header::{CacheControl, CacheDirective};
use actix_web::http::StatusCode;
use actix_web::web::{Bytes, Data, Path, Query};
use actix_web::{
    delete, error, get, post, web, Error, HttpRequest, HttpResponse, Responder, Result,
};
use chrono::naive::serde::ts_milliseconds;
use futures_util::TryStreamExt as _;
use serde::Deserialize;

use crate::db::photos::{CreatePhoto, DeletePhoto, GetPhoto, GetPhotos, UpdatePhoto};
use crate::db::users::GetUser;
use crate::model::photo::{Photo, PhotoBody};
use crate::model::user::User;
use crate::AppState;

const PUBLIC_USER_ID: i64 = 1;

#[derive(Debug)]
struct SimpleError(&'static str);

impl fmt::Display for SimpleError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "my error: {}", self.0)
    }
}

// Use default implementation for `error_response()` method
impl error::ResponseError for SimpleError {}

async fn base_download_photo(state: &AppState, user_id: i64, photo_id: i64) -> Result<NamedFile> {
    let db = state.db.clone();
    let storage = state.storage.borrow();

    let user: User = match db.send(GetUser::Id(user_id)).await {
        Ok(Ok(user)) => user,
        _ => return Err(Error::from(SimpleError("Invalid user id"))),
    };

    let photo: Photo = match db.send(GetPhoto { id: photo_id }).await {
        Ok(Ok(photo)) => photo,
        _ => return Err(Error::from(SimpleError("Could not find photo"))),
    };

    let photo_path = photo
        .partial_path(&user)
        .expect("Photo does not belong to this user");
    // let file = File::open(storage.resolve(photo_path)).expect("Could not open photo");
    let file = NamedFile::open_async(storage.resolve(photo_path))
        .await
        .expect("Could not open photo");
    // file.use_etag(false);

    /*let (tx, rx) = local_channel::mpsc::channel::<Result<Bytes, Error>>();
    actix_web::rt::spawn(async move {
        const CAPACITY: usize = 8192 * 4;
        let mut reader = BufReader::with_capacity(CAPACITY, file);
        let mut buf = [0u8; CAPACITY];

        while reader.read_exact(&mut buf).is_ok() {
            if tx.send(Ok(Bytes::copy_from_slice(&buf))).is_err() {
                return;
            }
        }
    });*/
    /*.content_type(
        mime_guess::from_path(photo.name)
            .first_or(mime_guess::mime::IMAGE_JPEG)
            .essence_str(),
    )*/
    /*let response = HttpResponse::Ok()
    .insert_header(CacheControl(vec![CacheDirective::MaxAge(31536000)]))
    .finish(); // 1 Year*/

    // .streaming(file)

    Ok(file)
}

async fn base_upload_photo(
    state: &AppState,
    user_id: i64,
    query: UploadData,
    mut payload: Multipart,
) -> Result<HttpResponse, Error> {
    let db = state.db.clone();
    let storage = state.storage.borrow();

    let user: User = match db.send(GetUser::Id(user_id)).await {
        Ok(Ok(user)) => user,
        _ => return Ok(HttpResponse::BadRequest().json("Invalid user id")),
    };

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
            file_size: 0,
            folder: query.folder_name.clone(),
        });

        let folder = match query.folder_name.clone() {
            None => String::new(),
            Some(folder) => folder + "/",
        };

        let filepath = storage.resolve(format!("{}/{}{}", user.user_name, folder, file_name));
        println!("Uploading file to {}", filepath.to_string_lossy());

        // File::create is blocking operation, use thread pool
        let mut f = web::block(|| File::create(filepath)).await??;

        // Field in turn is stream of *Bytes* object
        while let Some(chunk) = field.try_next().await? {
            // filesystem operations are blocking, we have to use thread pool
            f = web::block(move || f.write_all(&chunk).map(|_| f)).await??;
        }
    }

    match db.send(CreatePhoto(new_photo.unwrap())).await {
        // TODO Handle Unwrap
        Ok(Ok(photo)) => {
            if photo.owner != user_id {
                Ok(HttpResponse::BadRequest().json(
                    "Photo does not belong to user ".to_string() + user_id.to_string().as_str(),
                ))
            } else {
                Ok(HttpResponse::Ok().json(photo))
            }
        }
        _ => Ok(HttpResponse::InternalServerError().json("Something went wrong")),
    }
}

async fn base_delete_photo(state: &AppState, user_id: i64, photo_id: i64) -> impl Responder {
    let db = state.db.clone();
    let storage = state.storage.borrow();

    let user: User = match db.send(GetUser::Id(user_id)).await {
        Ok(Ok(user)) => user,
        _ => return HttpResponse::BadRequest().json("Invalid user id"),
    };

    let photo: Photo = match db.send(GetPhoto { id: photo_id }).await {
        Ok(Ok(photo)) => photo,
        _ => return HttpResponse::InternalServerError().json("Something went wrong"),
    };

    if storage.delete_file(
        photo
            .partial_path(&user)
            .expect("Photo does not belong to this user"),
    ) {
        match db.send(DeletePhoto { id: photo_id }).await {
            Ok(Ok(_count)) => HttpResponse::Ok().json("{\"deleted\": true}"),
            _ => HttpResponse::InternalServerError().json("Failed to remove photo from database"),
        }
    } else {
        HttpResponse::InternalServerError().json("File could not be deleted")
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

#[get("/{user_id}/download/{photo_id}")]
pub async fn download_photo(state: Data<AppState>, path: Path<(i64, i64)>) -> Result<NamedFile> {
    let (user_id, photo_id) = path.into_inner();
    base_download_photo(state.get_ref(), user_id, photo_id).await
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadData {
    #[serde(with = "ts_milliseconds")]
    time_created: chrono::NaiveDateTime,
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeLocationQuery {
    target_user_id: i64,
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

    let user = match db.send(GetUser::Id(user_id)).await {
        Ok(Ok(user)) => user,
        _ => return HttpResponse::BadRequest().json("Invalid user id"),
    };

    let photo: Photo = match db.send(GetPhoto { id: photo_id }).await {
        Ok(Ok(photo)) => photo,
        _ => return HttpResponse::InternalServerError().json("Something went wrong"),
    };

    let changed_photo = {
        let mut new = photo.clone();
        new.owner = query.target_user_id;
        new.folder = query.target_folder_name.clone();
        new
    };

    storage.move_file(
        photo.partial_path(&user).unwrap(),
        changed_photo.partial_path(&user).unwrap(),
    );

    match db.send(UpdatePhoto(changed_photo)).await {
        Ok(Ok(_)) => {}
        _ => return HttpResponse::InternalServerError().json("Something went wrong"),
    };

    if storage.delete_file(
        photo
            .partial_path(&user)
            .expect("Photo does not belong to user"),
    ) {
        HttpResponse::Ok().json("{\"deleted\": true}")
    } else {
        HttpResponse::InternalServerError().json("File could not be deleted")
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

#[delete("/delete/{photo_id}")]
pub async fn public_delete_photo(state: Data<AppState>, photo_id: Path<i64>) -> impl Responder {
    base_delete_photo(state.get_ref(), PUBLIC_USER_ID, photo_id.into_inner()).await
}

// endregion Public
