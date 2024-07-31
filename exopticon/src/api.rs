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
    InternalError,
}

impl Error for UserError {}

impl Display for UserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error!")
    }
}

impl IntoResponse for UserError {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::NotFound => StatusCode::NOT_FOUND.into_response(),
            Self::Validation(_) => StatusCode::UNPROCESSABLE_ENTITY.into_response(),
            Self::InternalError => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
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
impl From<JoinError> for UserError {
    fn from(_: JoinError) -> Self {
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

impl From<tokio::sync::mpsc::error::SendError<Command>> for UserError {
    fn from(_: tokio::sync::mpsc::error::SendError<Command>) -> Self {
        Self::InternalError
    }
}
