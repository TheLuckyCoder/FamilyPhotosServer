use std::net::SocketAddr;
use std::str::FromStr;

use anyhow::Context;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::ConnectOptions;
#[cfg(not(target_env = "msvc"))]
use tikv_jemallocator::Jemalloc;
use tokio::net::TcpListener;
use tower_sessions_sqlx_store::PostgresStore;
use tracing::info;
use tracing::log::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

use crate::http::AppState;
use crate::model::user::{User, PUBLIC_USER_ID};
use crate::repo::users_repo::UsersRepository;
use crate::utils::env_reader::EnvVariables;
use crate::utils::password_hash::{generate_hash_from_password, generate_random_password};
use crate::utils::storage_resolver::StorageResolver;

mod cli;
mod file_scan;
mod http;
mod model;
mod previews;
mod repo;
mod utils;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Environment Variables
    EnvVariables::init();
    let vars = EnvVariables::get_all();

    // Logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().compact())
        .with(EnvFilter::from_default_env())
        .init();

    let connect_options = PgConnectOptions::from_str(&vars.database_url)
        .expect("Failed to deserialize connection string")
        .log_statements(LevelFilter::Trace);

    // Database pool and app state
    let pool = PgPoolOptions::new()
        .max_connections(128)
        .connect_with(connect_options)
        .await
        .expect("Error building the connection pool");

    if vars.auto_migrate_database {
        sqlx::migrate!().run(&pool).await?;
    }

    let app_state = AppState::new(
        pool.clone(),
        StorageResolver::new(vars.storage_path, vars.previews_path),
    );

    let session_store = PostgresStore::new(pool);
    session_store
        .migrate()
        .await
        .expect("Failed to run schema migration for authentication");

    // Create default public user
    create_public_user(&app_state.users_repo).await?;

    // Run the CLI
    if cli::run_cli(&app_state).await {
        return Ok(());
    }

    // Scan the storage directory for new photos in the background
    if vars.scan_new_files {
        file_scan::scan_new_files(app_state.clone());
    }

    info!("Server listening on port {}", vars.server_port);

    let http_service = http::router(app_state, session_store).into_make_service();
    let addr = SocketAddr::from(([127, 0, 0, 1], vars.server_port));
    let listener = TcpListener::bind(addr).await?;

    Ok(axum::serve(listener, http_service).await?)
}

async fn create_public_user(repo: &UsersRepository) -> anyhow::Result<()> {
    if repo.get_user(PUBLIC_USER_ID).await.is_some() {
        return Ok(());
    }

    let user = User {
        id: PUBLIC_USER_ID.to_string(),
        name: PUBLIC_USER_ID.to_string(),
        password_hash: generate_hash_from_password(generate_random_password()),
    };

    info!("No users found, creating public user");

    repo.insert_user(&user)
        .await
        .context("Failed to create public user")
}
