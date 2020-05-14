// errors.rs
use actix_web::error::ResponseError;
use actix_web::HttpResponse;

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
            Self::InternalServerError => {
                HttpResponse::InternalServerError().json("Internal Server Error")
            }
            Self::NotFound => HttpResponse::NotFound().json("Not Found"),
            Self::BadRequest(ref message) => HttpResponse::BadRequest().json(message),
        }
    }
}

impl From<diesel::result::Error> for ServiceError {
    fn from(_err: diesel::result::Error) -> Self {
        Self::InternalServerError
    }
}
