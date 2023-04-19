use axum::http::StatusCode;
use axum::response::{ErrorResponse, IntoResponse, Response};
use std::fmt;
use std::fmt::Formatter;

#[derive(Debug)]
pub struct StatusError {
    message: String,
    status_code: StatusCode,
}

impl StatusError {
    pub fn create<S: Into<String>>(message: S) -> ErrorResponse {
        ErrorResponse::from(Self {
            message: message.into(),
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        })
    }

    pub fn new_status<S: Into<String>>(message: S, status_code: StatusCode) -> ErrorResponse {
        ErrorResponse::from(Self {
            message: message.into(),
            status_code,
        })
    }
}

impl fmt::Display for StatusError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.status_code, self.message)
    }
}

impl IntoResponse for StatusError {
    fn into_response(self) -> Response {
        (self.status_code, self.message).into_response()
    }
}
