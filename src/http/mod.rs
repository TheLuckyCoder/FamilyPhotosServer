use crate::repo::photos_repo::PhotosRepository;
use crate::repo::users_repo::UsersRepository;
use crate::utils::file_storage::FileStorage;
use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tower_http::{cors, trace};
use tracing::Level;

mod photos_api;
mod status_error;
mod users_api;
mod utils;

pub fn router(app_state: AppState) -> Router {
    Router::new()
        .route("/", get(|| async { StatusCode::OK }))
        .route("/ping", get(|| async { StatusCode::OK }))
        .nest("/users", users_api::router(app_state.users_repo.clone()))
        .merge(photos_api::router(app_state))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(trace::DefaultMakeSpan::new().level(Level::INFO))
                .on_response(trace::DefaultOnResponse::new().level(Level::INFO)),
        )
        .layer(CorsLayer::new().allow_origin(cors::Any))
}

#[derive(Clone)]
pub struct AppState {
    pub storage: FileStorage,
    pub users_repo: UsersRepository,
    pub photos_repo: PhotosRepository,
}
