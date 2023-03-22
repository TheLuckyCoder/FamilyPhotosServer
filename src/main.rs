extern crate diesel;

use actix::SyncArbiter;
use actix_web::middleware::{Logger, TrailingSlash};
use actix_web::web::Data;
use actix_web::{dev::ServiceRequest, middleware, web, App, Error, HttpResponse, HttpServer};
use actix_web_httpauth::extractors::{basic, AuthenticationError};
use actix_web_httpauth::headers::www_authenticate::basic::Basic;
use actix_web_httpauth::middleware::HttpAuthentication;
use env_logger::Env;
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};

use crate::api::photos_api::*;
use crate::api::users_api::*;
use crate::db::users::{GetUsers, InsertUser};
use crate::db::DbActor;
use crate::model::user::User;
use crate::utils::data_scan::DataScan;
use crate::utils::env_reader::EnvVariables;
use crate::utils::file_storage::FileStorage;
use crate::utils::password_hash::{generate_password, get_hash_from_password};
use crate::utils::AppState;

mod api;
mod cli;
mod db;
mod model;
mod schema;
mod thumbnail;
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
    EnvVariables::init();
    let vars = EnvVariables::get_all();
    env_logger::Builder::from_env(Env::default())
        .format_timestamp(None)
        .init();

    let manager = SyncArbiter::start(2, move || DbActor::new(vars.database_url.as_str()));
    log::info!("Database connection pool started");

    let app_state = AppState {
        db: manager.clone(),
        storage: FileStorage::new(vars.storage_path),
    };

    cli::run_cli(&app_state).await;

    // Scan the storage directory for new photos in the background
    if !vars.skip_scanning {
        let app_state_copy = app_state.clone();
        DataScan::run(app_state_copy);
    }

    if vars.generate_thumbnails_background {
        match thumbnail::generate_background(&app_state.clone()).await {
            Ok(_) => log::info!("Background thumbnail generation finished"),
            Err(e) => log::error!("Could not start background thumbnail generation: {e}"),
        }
    }

    {
        let mut users: Vec<User> = match manager.send(GetUsers).await {
            Ok(Ok(users)) => users,
            _ => panic!("Could not load users"),
        };

        if users.is_empty() {
            let public_user = User {
                id: 1,
                display_name: "Public".to_string(),
                user_name: "public".to_string(),
                password: generate_password(),
            };

            println!(
                "No users found, creating public user with password: {}",
                public_user.password
            );

            match manager.send(InsertUser::WithId(public_user)).await {
                Ok(Ok(user)) => users = vec![user],
                _ => panic!("Failed to create public user"),
            };
        }

        unsafe {
            USERS = users;
        }
    }

    log::info!("Starting server on port {}", vars.server_port);

    let server = HttpServer::new(move || {
        let logger = Logger::new(r#"%r %s %b "%{User-Agent}i" %T"#);
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
                    .service(thumbnail_photo)
                    .service(download_photo)
                    .service(get_photo_exif)
                    .service(upload_photo)
                    .service(update_photo_caption)
                    .service(delete_photo)
                    .service(change_photo_location),
            )
            .service(
                web::scope("/public_photos")
                    .wrap(auth)
                    .service(public_photos_list)
                    .service(public_thumbnail_photo)
                    .service(public_download_photo)
                    .service(public_upload_photo)
                    .service(public_update_photo_caption)
                    .service(public_delete_photo),
            )
            .app_data(Data::new(app_state.clone()))
    });

    if vars.use_https {
        let mut ssl_builder = SslAcceptor::mozilla_intermediate_v5(SslMethod::tls())?;
        ssl_builder.set_private_key_file(
            vars.ssl_private_key_path
                .expect("SSL_PRIVATE_KEY_PATH is missing"),
            SslFiletype::PEM,
        )?;
        ssl_builder
            .set_certificate_chain_file(vars.ssl_certs_path.expect("SSL_CERTS_PATH is missing"))?;

        log::info!("Server configured successfully in HTTPS mode");
        server
            .bind_openssl(("127.0.0.1", vars.server_port), ssl_builder)?
            .run()
            .await
    } else {
        log::info!("Server configured successfully in HTTP mode");
        server.bind(("127.0.0.1", vars.server_port))?.run().await
    }
}
