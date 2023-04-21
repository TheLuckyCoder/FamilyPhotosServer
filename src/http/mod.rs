use crate::db::Pool;
use crate::utils::file_storage::FileStorage;
use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use tower_http::trace;
use tower_http::trace::TraceLayer;
use tracing::Level;

mod mw;
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
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(trace::DefaultMakeSpan::new().level(Level::INFO))
                .on_response(trace::DefaultOnResponse::new().level(Level::INFO)),
        )
}

#[derive(Clone)]
pub struct AppState {
    pub pool: Pool,
    pub storage: FileStorage,
}
