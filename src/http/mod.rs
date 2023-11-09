use axum::error_handling::HandleErrorLayer;
use axum::extract::DefaultBodyLimit;
use axum::http::StatusCode;
use axum::routing::get;
use axum::{BoxError, Router};
use axum_login::AuthManagerLayer;
use sqlx::PgPool;
use time::Duration;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tower_http::{cors, trace};
use tower_sessions::{Expiry, PostgresStore, SessionManagerLayer};
use tracing::{error, Level};

use crate::repo::photos_repo::PhotosRepository;
use crate::repo::users_repo::UsersRepository;
use crate::thumbnail::ThumbnailManager;
use crate::utils::file_storage::FileStorage;

mod photos_api;
mod users_api;
mod utils;

pub fn router(app_state: AppState, session_store: PostgresStore) -> Router {
    let session_layer = SessionManagerLayer::new(session_store)
        .with_expiry(Expiry::OnInactivity(Duration::days(30)));

    let auth_service = ServiceBuilder::new()
        .layer(HandleErrorLayer::new(|e: BoxError| async move {
            error!("Auth error: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        }))
        .layer(AuthManagerLayer::new(
            app_state.users_repo.clone(),
            session_layer,
        ));

    Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/ping", get(|| async { StatusCode::OK }))
        .merge(users_api::router())
        .merge(photos_api::router(app_state))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(trace::DefaultMakeSpan::new().level(Level::INFO))
                .on_response(trace::DefaultOnResponse::new().level(Level::INFO)),
        )
        .layer(CorsLayer::new().allow_origin(cors::Any))
        .layer(auth_service)
        .layer(DefaultBodyLimit::max(2 * 1024 * 1024 * 1024)) // 2GB
}

#[derive(Clone)]
pub struct AppState {
    pub storage: FileStorage,
    pub users_repo: UsersRepository,
    pub photos_repo: PhotosRepository,
    pub thumbnail_manager: ThumbnailManager,
}

impl AppState {
    pub fn new(pool: PgPool, storage: FileStorage) -> Self {
        Self {
            storage,
            users_repo: UsersRepository::new(pool.clone()),
            photos_repo: PhotosRepository::new(pool),
            thumbnail_manager: ThumbnailManager::default(),
        }
    }
}
