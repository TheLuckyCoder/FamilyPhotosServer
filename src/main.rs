extern crate diesel;

use axum_server::tls_rustls::RustlsConfig;
use std::net::SocketAddr;
use std::sync::Arc;

use crate::db::*;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use rand::SeedableRng;
use rand_hc::Hc128Rng;
#[cfg(not(target_env = "msvc"))]
use tikv_jemallocator::Jemalloc;
use tokio::sync::Mutex;
use tracing::{error, info};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

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

#[tokio::main]
async fn main() -> Result<(), String> {
    EnvVariables::init();
    let vars = EnvVariables::get_all();

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().compact())
        .with(EnvFilter::from_default_env())
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
            Ok(_) => info!("Background thumbnail generation finished"),
            Err(e) => error!("Could not start background thumbnail generation: {e}"),
        }
    }

    {
        let users: Vec<User> = app_state
            .pool
            .send(GetUsers)
            .await
            .expect("Could not load users");

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

            app_state
                .pool
                .send(InsertUser::WithId(public_user))
                .await
                .expect("Failed inserting the default public user");
        }
    }

    info!("Server listening on port {}", vars.server_port);

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

        info!("Server configured in HTTPS mode");
        axum_server::bind_rustls(addr, config)
            .serve(http_service)
            .await
    } else {
        info!("Server configured in HTTP mode");
        axum_server::bind(addr).serve(http_service).await
    }
    .expect("Failed to start axum server");

    Ok(())
}
