use actix_web::http::StatusCode;
use actix_web::{error, Error};
use std::fmt;
use std::fmt::Formatter;

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

    pub fn new_status<S: Into<String>>(message: S, status_code: StatusCode) -> Error {
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
