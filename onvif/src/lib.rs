//! Onvif client library

#![deny(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
#![allow(clippy::integer_arithmetic)]
#![allow(clippy::missing_inline_in_public_items)]
#![allow(clippy::multiple_crate_versions)]
#![allow(clippy::implicit_return)]
#![allow(clippy::expect_used)] // TODO: Fix this one
#![allow(clippy::missing_errors_doc)] // TODO: Fix this one

#[macro_use]
extern crate log;
extern crate serde;
extern crate serde_json;

/// module implementing the onvif camera api
pub mod camera;

/// module implementing device discovery
pub mod discovery;

/// module describing onvif errors
pub mod error;

/// utility module, mostly soap tools
mod util;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
