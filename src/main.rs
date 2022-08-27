#[macro_use]
extern crate diesel;

use std::sync::Mutex;
use std::time::Instant;

use actix::SyncArbiter;
use actix_web::middleware::{Logger, TrailingSlash};
use actix_web::web::Data;
use actix_web::{dev::ServiceRequest, middleware, web, App, Error, HttpServer};
use actix_web_httpauth::extractors::{basic, AuthenticationError};
use actix_web_httpauth::headers::www_authenticate::basic::Basic;
use actix_web_httpauth::middleware::HttpAuthentication;
use rand::prelude::*;
use rand_hc::Hc128Rng;

use crate::api::home::ping;
use crate::api::photos::{
    change_photo_location, delete_photo, download_photo, photos_list, public_delete_photo,
    public_download_photo, public_photos_list, public_upload_photo, upload_photo,
};
use crate::api::users::{create_user, get_user, get_users};
use crate::db::users::GetUsers;
use crate::db::DbActor;
use crate::model::user::User;
use crate::utils::data_scan::DataScan;
use crate::utils::db::get_pool;
use crate::utils::file_storage::FileStorage;
use crate::utils::password_hash::get_hash_from_password;
use crate::utils::AppState;

mod api;
mod db;
mod model;
mod schema;
mod utils;

static mut USERS: Vec<User> = Vec::new();

async fn any_user_auth_validator(
    req: ServiceRequest,
    credentials: basic::BasicAuth,
) -> Result<ServiceRequest, (Error, ServiceRequest)> {
    let found = unsafe { USERS.clone() }
        .into_iter()
        .find(|user| user.user_name == credentials.user_id());

    if let Some(user) = found {
        if let Some(password) = credentials.password() {
            if get_hash_from_password(&password.to_string()) == user.password {
                return Ok(req);
            }
        }
    }

    Err((Error::from(AuthenticationError::new(Basic::new())), req))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let storage_path = std::env::var("STORAGE_PATH").expect("STORAGE_PATH must be set");

    std::env::set_var("RUST_LOG", "debug");
    std::env::set_var("RUST_BACKTRACE", "1");
    env_logger::init();

    let manager = SyncArbiter::start(2, move || {
        let pool = get_pool(database_url.as_str());
        let rng = Mutex::new(Hc128Rng::from_entropy());
        DbActor(pool, rng)
    });

    let app_state = AppState {
        db: manager.clone(),
        storage: FileStorage::new(storage_path.clone()),
    };

    let app_state2 = app_state.clone();
    actix_web::rt::spawn(async move {
        let instant = Instant::now();
        let data_scan = DataScan::scan(&app_state2).await;
        data_scan.update_database(&app_state2).await;
        println!(
            "Photos scanning completed in {} seconds",
            instant.elapsed().as_secs()
        );
    });

    {
        let mut users: Vec<User> = match manager.send(GetUsers).await {
            Ok(Ok(users)) => users,
            _ => panic!("Could not load users"),
        };
        unsafe {
            USERS.append(&mut users);
        }
    }

    HttpServer::new(move || {
        let logger = Logger::new(r#"%r %s %b "%{Referer}i" "%{User-Agent}i" %T"#);
        let auth = HttpAuthentication::basic(any_user_auth_validator);

        App::new()
            .wrap(logger)
            .wrap(middleware::NormalizePath::new(TrailingSlash::Trim))
            .service(ping)
            .service(create_user)
            .service(
                web::scope("/user")
                    .wrap(auth.clone())
                    .service(get_user)
                    .service(get_users),
            )
            .service(
                web::scope("/photos")
                    .wrap(auth.clone())
                    .service(photos_list)
                    .service(download_photo)
                    .service(upload_photo)
                    .service(delete_photo)
                    .service(change_photo_location),
            )
            .service(
                web::scope("/public_photos")
                    .wrap(auth)
                    .service(public_photos_list)
                    .service(public_download_photo)
                    .service(public_upload_photo)
                    .service(public_delete_photo),
            )
            .app_data(Data::new(app_state.clone()))
    })
    .bind(("127.0.0.1", 5000))?
    .run()
    .await
}
