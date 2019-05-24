//! Onvif client library

#![warn(
    clippy::all,
    clippy::restriction,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo
)]
#![allow(clippy::integer_arithmetic)]
#![allow(clippy::missing_inline_in_public_items)]
#![allow(clippy::multiple_crate_versions)]
#![allow(clippy::implicit_return)]

#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;
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
