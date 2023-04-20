use async_trait::async_trait;
use axum::http::StatusCode;
use axum::response::{ErrorResponse, IntoResponse};
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::AsyncPgConnection;
use rand_hc::Hc128Rng;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct Pool(
    pub bb8::Pool<AsyncDieselConnectionManager<AsyncPgConnection>>,
    pub Arc<Mutex<Hc128Rng>>,
);

#[async_trait]
pub trait Handler<M> {
    type Result;

    async fn send(&self, msg: M) -> Self::Result;
}

/// Utility function for mapping any error into a `500 Internal Server Error`
/// response.
pub fn internal_error<E>(err: E) -> ErrorResponse
where
    E: std::error::Error,
{
    ErrorResponse::from((StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response())
}
