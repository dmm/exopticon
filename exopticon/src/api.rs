/*
 * Exopticon - A free video surveillance system.
 * Copyright (C) 2022 David Matthew Mattli <dmm@mattli.us>
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

use std::{
    error::Error,
    fmt::{self, Display},
};

use actix_http::StatusCode;
use actix_web::{error, http::header::ContentType, HttpResponse};

pub mod camera_groups;

#[derive(Debug)]
pub enum UserError {
    NotFound,
    Validation(String),
    InternalError,
}

impl Error for UserError {}

impl Display for UserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error!")
    }
}

impl error::ResponseError for UserError {
    fn error_response(&self) -> HttpResponse {
        let mut res = HttpResponse::build(self.status_code());
        res.insert_header(ContentType::json());

        if let UserError::Validation(msg) = &*self {
            res.body(String::from(msg))
        } else {
            res.finish()
        }
    }

    fn status_code(&self) -> StatusCode {
        match &*self {
            UserError::NotFound => StatusCode::NOT_FOUND,
            UserError::Validation(_msg) => StatusCode::UNPROCESSABLE_ENTITY,
            UserError::InternalError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<crate::db::Error> for UserError {
    fn from(err: crate::db::Error) -> Self {
        match err {
            crate::db::Error::NotFound => Self::NotFound,
            crate::db::Error::Other(_) => Self::InternalError,
        }
    }
}

impl From<actix_web::error::BlockingError> for UserError {
    fn from(_err: actix_web::error::BlockingError) -> Self {
        Self::InternalError
    }
}

impl From<crate::business::Error> for UserError {
    fn from(err: crate::business::Error) -> Self {
        match err {
            crate::business::Error::Validation(message) => Self::Validation(message),
        }
    }
}
