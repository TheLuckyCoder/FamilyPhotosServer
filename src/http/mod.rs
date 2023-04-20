use crate::db::Pool;
use crate::utils::file_storage::FileStorage;
use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;

mod photos_api;
mod status_error;
mod users_api;
mod utils;

pub fn router(app_state: AppState) -> Router {
    Router::new()
        .route("/", get(|| async { StatusCode::OK }))
        .route("/ping", get(|| async { StatusCode::OK }))
        .nest("/users", users_api::router(app_state.pool.clone()))
        .nest("/photos", photos_api::router(app_state))
}

#[derive(Clone)]
pub struct AppState {
    pub pool: Pool,
    pub storage: FileStorage,
}
