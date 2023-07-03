use std::net::SocketAddr;

use axum_server::tls_rustls::RustlsConfig;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::ConnectOptions;
#[cfg(not(target_env = "msvc"))]
use tikv_jemallocator::Jemalloc;
use tracing::{error, info};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

use crate::http::AppState;
use crate::model::user::User;
use crate::repo::photos_repo::PhotosRepository;
use crate::repo::users_repo::UsersRepository;
use crate::utils::env_reader::EnvVariables;
use crate::utils::file_storage::FileStorage;

mod cli;
mod file_scan;
mod http;
mod model;
mod repo;
mod thumbnail;
mod utils;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[tokio::main]
async fn main() -> Result<(), String> {
    // Environment Variables
    EnvVariables::init();
    let vars = EnvVariables::get_all();

    // Logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().compact())
        .with(EnvFilter::from_default_env())
        .init();

    // Database pool and app state
    let pool = PgPoolOptions::new()
        .max_connections(128)
        .connect(&vars.database_url)
        .await
        .expect("Error building the connection pool");

    sqlx::migrate!()
        .run(&pool)
        .await
        .map_err(|e| e.to_string())?;

    let app_state = AppState {
        storage: FileStorage::new(vars.storage_path, vars.thumbnail_path),
        users_repo: UsersRepository::new(pool.clone()),
        photos_repo: PhotosRepository::new(pool.clone()),
    };

    // Run the CLI
    if cli::run_cli(&pool, &app_state).await {
        return Ok(());
    }

    // Scan the storage directory for new photos in the background
    if vars.scan_new_files {
        file_scan::scan_new_files(app_state.clone());
    }

    if vars.generate_thumbnails_background {
        match thumbnail::generate_background(app_state.clone()).await {
            Ok(_) => info!("Background thumbnail generation finished"),
            Err(e) => error!("Could not start background thumbnail generation: {e}"),
        }
    }

    info!("Server listening on port {}", vars.server_port);

    let http_service = http::router(pool, app_state).into_make_service();
    let addr = SocketAddr::from(([127, 0, 0, 1], vars.server_port));

    if vars.use_https {
        let config = RustlsConfig::from_pem_file(
            vars.ssl_certs_path
                .expect("SSL_CERTS_PATH variable is missing"),
            vars.ssl_private_key_path
                .expect("SSL_PRIVATE_KEY_PATH variable is missing"),
        )
        .await
        .map_err(|e| format!("Failed to load TLS config: {}", e))?;

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
