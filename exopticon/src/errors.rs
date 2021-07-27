/*
 * Exopticon - A free video surveillance system.
 * Copyright (C) 2020 David Matthew Mattli <dmm@mattli.us>
 *
 * This file is part of Exopticon.
 *
 * Exopticon is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * Exopticon is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Exopticon.  If not, see <http://www.gnu.org/licenses/>.
 */

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

impl From<actix::MailboxError> for ServiceError {
    fn from(_: actix::MailboxError) -> Self {
        Self::InternalServerError
    }
}

impl From<std::io::Error> for ServiceError {
    fn from(_: std::io::Error) -> Self {
        Self::InternalServerError
    }
}

impl From<()> for ServiceError {
    fn from(_: ()) -> Self {
        Self::InternalServerError
    }
}
