// errors.rs
use actix_web::{error::ResponseError, HttpResponse};

/// Enum of service errors
#[derive(Fail, Debug)]
pub enum ServiceError {
    /// Internal server error
    #[fail(display = "Internal Server Error")]
    InternalServerError,

    /// Bad Request
    #[fail(display = "BadRequest: {}", _0)]
    BadRequest(String),

    /// Resource Not Found
    #[fail(display = "Not Found")]
    NotFound,
}

// impl ResponseError trait allows to convert our errors into http responses with appropriate data
impl ResponseError for ServiceError {
    fn error_response(&self) -> HttpResponse {
        match *self {
            ServiceError::InternalServerError => {
                HttpResponse::InternalServerError().json("Internal Server Error")
            }
            ServiceError::NotFound => HttpResponse::NotFound().json("Not Found"),
            ServiceError::BadRequest(ref message) => HttpResponse::BadRequest().json(message),
        }
    }
}
