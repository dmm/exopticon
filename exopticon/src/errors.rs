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

use thiserror::Error;

/// Enum of service errors
#[derive(Error, Debug)]
pub enum ServiceError {
    /// Internal server error
    #[error("Internal Server Error")]
    InternalServerError,

    /// Bad Request
    #[error("BadRequest: {0}")]
    BadRequest(String),

    /// Resource Not Found
    #[error("Not Found")]
    NotFound,
}

impl From<diesel::result::Error> for ServiceError {
    fn from(_err: diesel::result::Error) -> Self {
        Self::InternalServerError
    }
}

impl From<std::io::Error> for ServiceError {
    fn from(_: std::io::Error) -> Self {
        Self::InternalServerError
    }
}

impl From<()> for ServiceError {
    fn from((): ()) -> Self {
        Self::InternalServerError
    }
}

impl From<ServiceError> for axum::http::StatusCode {
    fn from(_: ServiceError) -> Self {
        Self::INTERNAL_SERVER_ERROR
    }
}
