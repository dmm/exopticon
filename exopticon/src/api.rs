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

use axum::{http::StatusCode, response::IntoResponse};
use tokio::task::JoinError;

use crate::capture_supervisor::Command;

pub mod auth;
pub mod basic_auth_middleware;
pub mod camera_groups;
pub mod cameras;
pub mod static_files;
pub mod storage_groups;
pub mod video_units;
pub mod webrtc;

/// Error to be presented to api user
#[derive(Debug)]
pub enum UserError {
    /// resource wasn't found
    NotFound,
    /// validation error
    Validation(String),
    /// internal server error
    InternalError(String),
}

impl Error for UserError {}

impl Display for UserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error!")
    }
}

impl IntoResponse for UserError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            Self::NotFound => (StatusCode::NOT_FOUND, "not found".to_owned()),
            Self::Validation(err_string) => (StatusCode::UNPROCESSABLE_ENTITY, err_string),
            Self::InternalError(msg) => {
                error!("Internal Server Error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal server error".to_string(),
                )
            }
        };

        (status, message).into_response()
    }
}

impl From<crate::db::Error> for UserError {
    fn from(err: crate::db::Error) -> Self {
        match err {
            crate::db::Error::NotFound => Self::NotFound,
            crate::db::Error::Other(other) => {
                Self::InternalError(format!("other db error: {other}"))
            }
        }
    }
}
impl From<JoinError> for UserError {
    fn from(err: JoinError) -> Self {
        Self::InternalError(format!("JoinError: {err}"))
    }
}

impl From<crate::business::Error> for UserError {
    fn from(err: crate::business::Error) -> Self {
        match err {
            crate::business::Error::Validation(message) => Self::Validation(message),
        }
    }
}

impl From<tokio::sync::mpsc::error::SendError<Command>> for UserError {
    fn from(err: tokio::sync::mpsc::error::SendError<Command>) -> Self {
        Self::InternalError(format!("tokio SendError: {err}"))
    }
}
