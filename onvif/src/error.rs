/*
 * onvif - An onvif client library
 * Copyright (C) 2020 David Matthew Mattli <dmm@mattli.us>
 *
 * This file is part of onvif.
 *
 * onvif is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * onvif is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with onvif.  If not, see <http://www.gnu.org/licenses/>.
 */

//! Onvif api error

use thiserror::Error;

/// Onvif Error
#[derive(Error, Debug)]
pub enum Error {
    #[error("Connection to remote device failed")]
    ConnectionFailed,

    #[error("Operation required authentication and this failed")]
    Unauthorized,

    #[error("The remote device returned an invalid response")]
    InvalidResponse,

    #[error("An invalid argument was provided")]
    InvalidArgument,

    /// Represents all other cases of `std::io::Error`
    #[error(transparent)]
    IO(#[from] std::io::Error),
}

impl From<hyper::Error> for Error {
    #[must_use]
    fn from(_err: hyper::Error) -> Self {
        Self::ConnectionFailed
    }
}

impl From<std::string::FromUtf8Error> for Error {
    #[must_use]
    fn from(_err: std::string::FromUtf8Error) -> Self {
        Self::InvalidResponse
    }
}

impl From<sxd_document::parser::Error> for Error {
    #[must_use]
    fn from(_err: sxd_document::parser::Error) -> Self {
        Self::InvalidResponse
    }
}

impl From<sxd_xpath::Error> for Error {
    #[must_use]
    fn from(_err: sxd_xpath::Error) -> Self {
        Self::InvalidResponse
    }
}

impl From<std::num::ParseIntError> for Error {
    #[must_use]
    fn from(_err: std::num::ParseIntError) -> Self {
        Self::InvalidResponse
    }
}

impl From<std::str::ParseBoolError> for Error {
    #[must_use]
    fn from(_err: std::str::ParseBoolError) -> Self {
        Self::InvalidResponse
    }
}
