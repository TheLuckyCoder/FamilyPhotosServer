use axum::extract::DefaultBodyLimit;
use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use axum_login::tower_sessions::{Expiry, SessionManagerLayer};
use axum_login::AuthManagerLayerBuilder;
use sqlx::PgPool;
use time::Duration;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tower_http::{cors, trace};
use tower_sessions_sqlx_store::PostgresStore;
use tracing::Level;

use crate::repo::photos_repo::PhotosRepository;
use crate::repo::users_repo::UsersRepository;
use crate::utils::file_storage::FileStorage;

mod photos_api;
mod users_api;
mod utils;

pub fn router(app_state: AppState, session_store: PostgresStore) -> Router {
    let session_layer = SessionManagerLayer::new(session_store);

    let auth_layer =
        AuthManagerLayerBuilder::new(app_state.users_repo.clone(), session_layer).build();

    Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .merge(users_api::router())
        .nest("/photos", photos_api::router(app_state))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(trace::DefaultMakeSpan::new().level(Level::INFO))
                .on_response(trace::DefaultOnResponse::new().level(Level::INFO)),
        )
        .layer(CorsLayer::new().allow_origin(cors::Any))
        .layer(auth_layer)
        .layer(DefaultBodyLimit::max(2 * 1024 * 1024 * 1024)) // 2GB
}

#[derive(Clone)]
pub struct AppState {
    pub storage: FileStorage,
    pub users_repo: UsersRepository,
    pub photos_repo: PhotosRepository,
}

impl AppState {
    pub fn new(pool: PgPool, storage: FileStorage) -> Self {
        Self {
            storage,
            users_repo: UsersRepository::new(pool.clone()),
            photos_repo: PhotosRepository::new(pool),
        }
    }
}
