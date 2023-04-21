extern crate diesel;

use axum_server::tls_rustls::RustlsConfig;
use std::net::SocketAddr;
use std::sync::Arc;

use crate::db::*;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use env_logger::Env;
use rand::SeedableRng;
use rand_hc::Hc128Rng;
#[cfg(not(target_env = "msvc"))]
use tikv_jemallocator::Jemalloc;
use tokio::sync::Mutex;

use crate::db::users_db::{GetUsers, InsertUser};
use crate::http::AppState;
use crate::model::user::User;
use crate::utils::env_reader::EnvVariables;
use crate::utils::file_storage::FileStorage;
use crate::utils::password_hash::{generate_password, get_hash_from_password};

mod cli;
mod db;
mod file_scan;
mod http;
mod model;
mod schema;
mod thumbnail;
mod utils;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

/*static mut USERS: Vec<User> = Vec::new();

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
}*/

#[tokio::main]
async fn main() -> Result<(), String> {
    EnvVariables::init();
    let vars = EnvVariables::get_all();

    env_logger::Builder::from_env(Env::default())
        .format_timestamp(None)
        .init();
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    let config =
        AsyncDieselConnectionManager::<diesel_async::AsyncPgConnection>::new(vars.database_url);
    let pool = Pool(
        bb8::Pool::builder()
            .build(config)
            .await
            .expect("Error building the connection pool"),
        Arc::new(Mutex::new(Hc128Rng::from_entropy())),
    );

    let app_state = AppState {
        pool,
        storage: FileStorage::new(vars.storage_path, vars.thumbnail_path),
    };

    if cli::run_cli(&app_state.pool).await {
        return Ok(());
    }

    // Scan the storage directory for new photos in the background
    if vars.scan_new_files {
        file_scan::scan_new_files(app_state.clone()).await;
    }

    if vars.generate_thumbnails_background {
        match thumbnail::generate_background(&app_state.clone()).await {
            Ok(_) => log::info!("Background thumbnail generation finished"),
            Err(e) => log::error!("Could not start background thumbnail generation: {e}"),
        }
    }

    {
        let mut users: Vec<User> = match app_state.pool.send(GetUsers).await {
            Ok(users) => users,
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

            match app_state.pool.send(InsertUser::WithId(public_user)).await {
                Ok(user) => users = vec![user],
                _ => panic!("Failed to create public user"),
            };
        }

        /*unsafe {
            USERS = users;
        }*/
    }

    log::info!("Server listening on port {}", vars.server_port);

    let http_service = http::router(app_state).into_make_service();
    let addr = SocketAddr::from(([127, 0, 0, 1], vars.server_port));

    if vars.use_https {
        let config = RustlsConfig::from_pem_file(
            vars.ssl_certs_path
                .expect("SSL_CERTS_PATH variable is missing"),
            vars.ssl_private_key_path
                .expect("SSL_PRIVATE_KEY_PATH variable is missing"),
        )
        .await
        .map_err(|e| format!("Could not load TLS config: {}", e))?;

        log::info!("Server configured in HTTPS mode");
        axum_server::bind_rustls(addr, config)
            .serve(http_service)
            .await
    } else {
        log::info!("Server configured successfully in HTTP mode");
        axum_server::bind(addr).serve(http_service).await
    }
    .expect("Failed to start axum server");

    Ok(())
}
