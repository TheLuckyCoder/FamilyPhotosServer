use crate::db::Pool;
use crate::utils::file_storage::FileStorage;
use axum::Router;

mod photos;
mod status_error;
mod users;

type AxumResult<T> = axum::response::Result<T>;

pub fn router(app_state: AppState) -> Router {
    Router::new()
        .nest("/users", users::router(app_state.pool.clone()))
        .nest("/photos", photos::router(app_state))
}

#[derive(Clone)]
pub struct AppState {
    pub pool: Pool,
    pub storage: FileStorage,
}
