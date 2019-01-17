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
            Error::ConnectionFailed => write!(f, "Connection failed!"),
            Error::Unauthorized => write!(f, "Unauthorized"),
            Error::InvalidResponse => write!(f, "Invalid Response from device"),
            Error::InvalidArgument => write!(f, "Invalid argument provided"),
        }
    }
}

impl From<hyper::error::Error> for Error {
    fn from(_err: hyper::error::Error) -> Self {
        Error::ConnectionFailed
    }
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(_err: std::string::FromUtf8Error) -> Self {
        Error::InvalidResponse
    }
}

impl From<sxd_document::parser::Error> for Error {
    fn from(_err: sxd_document::parser::Error) -> Self {
        Error::InvalidResponse
    }
}

impl From<sxd_xpath::Error> for Error {
    fn from(_err: sxd_xpath::Error) -> Self {
        Error::InvalidResponse
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(_err: std::num::ParseIntError) -> Self {
        Error::InvalidResponse
    }
}
