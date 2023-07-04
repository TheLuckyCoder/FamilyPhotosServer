use crate::model::user::User;
use crate::repo::photos_repo::PhotosRepository;
use crate::repo::session_repo::SessionRepository;
use crate::repo::users_repo::UsersRepository;
use crate::utils::file_storage::FileStorage;
use axum::extract::DefaultBodyLimit;
use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use axum_login::axum_sessions::{PersistencePolicy, SessionLayer};
use axum_login::{AuthLayer, PostgresStore};
use sqlx::PgPool;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tower_http::{cors, trace};
use tracing::Level;

mod photos_api;
mod users_api;
mod utils;

pub fn router(pool: PgPool, app_state: AppState, session_secret: &[u8]) -> Router {
    let session_store = SessionRepository::new(pool.clone());
    let session_layer = SessionLayer::new(session_store, session_secret)
        .with_persistence_policy(PersistencePolicy::ChangedOnly);

    let user_store = PostgresStore::<User>::new(pool);
    let auth_layer = AuthLayer::new(user_store, session_secret);

    Router::new()
        .route("/", get(|| async { StatusCode::OK }))
        .route("/ping", get(|| async { StatusCode::OK }))
        .merge(users_api::router(app_state.users_repo.clone()))
        .merge(photos_api::router(app_state))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(trace::DefaultMakeSpan::new().level(Level::INFO))
                .on_response(trace::DefaultOnResponse::new().level(Level::INFO)),
        )
        .layer(CorsLayer::new().allow_origin(cors::Any))
        .layer(auth_layer)
        .layer(session_layer)
        .layer(DefaultBodyLimit::max(1024 * 1024 * 1024))
}

#[derive(Clone)]
pub struct AppState {
    pub storage: FileStorage,
    pub users_repo: UsersRepository,
    pub photos_repo: PhotosRepository,
}
