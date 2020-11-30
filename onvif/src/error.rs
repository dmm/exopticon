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

/// Onvif Error
#[derive(Debug)]
pub enum Error {
    /// Connection to remote device failed
    ConnectionFailed,
    /// Operation required authentication and this failed
    Unauthorized,
    /// The remote device returned an invalid response
    InvalidResponse,
    /// An invalid argument was provided
    InvalidArgument,
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::ConnectionFailed => write!(f, "Connection failed!"),
            Self::Unauthorized => write!(f, "Unauthorized"),
            Self::InvalidResponse => write!(f, "Invalid Response from device"),
            Self::InvalidArgument => write!(f, "Invalid argument provided"),
        }
    }
}

impl From<hyper::error::Error> for Error {
    #[must_use]
    fn from(_err: hyper::error::Error) -> Self {
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
