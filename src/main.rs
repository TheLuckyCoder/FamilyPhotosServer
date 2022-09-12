extern crate diesel;

use std::sync::Mutex;
use std::time::Instant;

use actix::SyncArbiter;
use actix_web::middleware::{Logger, TrailingSlash};
use actix_web::web::Data;
use actix_web::{dev::ServiceRequest, middleware, web, App, Error, HttpResponse, HttpServer};
use actix_web_httpauth::extractors::{basic, AuthenticationError};
use actix_web_httpauth::headers::www_authenticate::basic::Basic;
use actix_web_httpauth::middleware::HttpAuthentication;
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use rand::prelude::*;
use rand_hc::Hc128Rng;

use crate::api::photos::{
    change_photo_location, delete_photo, download_photo, photos_list, public_delete_photo,
    public_download_photo, public_photos_list, public_upload_photo, upload_photo,
};
use crate::api::users::{get_user, get_users};
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

fn get_env_var(var_name: &str) -> String {
    std::env::var(var_name).unwrap_or_else(|_| panic!("{var_name} must be set!"))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    if std::env::var("RUST_LOG").is_err() {
        eprintln!("Logging is disabled please set RUST_LOG to enable logging")
    }

    let skip_scanning: bool = get_env_var("SKIP_SCANNING").parse().unwrap();
    let server_port: u16 = get_env_var("SERVER_PORT")
        .parse()
        .expect("SERVER_PORT must be a valid port number!");
    let use_https: bool = get_env_var("USE_HTTPS").parse().unwrap();
    let database_url = get_env_var("DATABASE_URL");
    let storage_path = get_env_var("STORAGE_PATH");
    let ssl_private_key_path = get_env_var("SSL_PRIVATE_KEY_PATH");
    let ssl_certs_path = get_env_var("SSL_CERTS_PATH");

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

    // Scan the storage directory for new photos in the background
    if !skip_scanning {
        let app_state_copy = app_state.clone();
        actix_web::rt::spawn(async move {
            let instant = Instant::now();
            let data_scan = DataScan::scan(&app_state_copy).await;
            data_scan.update_database(&app_state_copy).await;

            println!(
                "Photos scanning completed in {} seconds",
                instant.elapsed().as_secs()
            );
        });
    }

    {
        let mut users: Vec<User> = match manager.send(GetUsers).await {
            Ok(Ok(users)) => users,
            _ => panic!("Could not load users"),
        };
        unsafe {
            USERS.append(&mut users);
        }
    }

    log::info!("Starting server on port {server_port}");

    let server = HttpServer::new(move || {
        let logger = Logger::new(r#"%r %s %b "%{Referer}i" "%{User-Agent}i" %T"#);
        let auth = HttpAuthentication::basic(any_user_auth_validator);

        App::new()
            .wrap(logger)
            .wrap(middleware::NormalizePath::new(TrailingSlash::Trim))
            .service(web::resource("").to(HttpResponse::Ok))
            .service(web::resource("/ping").to(HttpResponse::Ok))
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
    });

    if use_https {
        let mut ssl_builder = SslAcceptor::mozilla_intermediate_v5(SslMethod::tls())?;
        ssl_builder.set_private_key_file(ssl_private_key_path, SslFiletype::PEM)?;
        ssl_builder.set_certificate_chain_file(ssl_certs_path)?;

        server
            .bind_openssl(("127.0.0.1", server_port), ssl_builder)?
            .run()
            .await
    } else {
        server.bind(("127.0.0.1", server_port))?.run().await
    }
}
