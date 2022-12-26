use std::fmt;
use std::fmt::Formatter;
use actix_web::{Error, error};
use actix_web::http::StatusCode;

#[derive(Debug)]
pub struct StatusError {
    message: String,
    status_code: StatusCode,
}

impl StatusError {
    pub fn create<S: Into<String>>(message: S) -> Error {
        Error::from(Self {
            message: message.into(),
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        })
    }

    pub fn create_status<S: Into<String>>(message: S, status_code: StatusCode) -> Error {
        Error::from(Self {
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

impl error::ResponseError for StatusError {
    fn status_code(&self) -> StatusCode {
        self.status_code
    }
}