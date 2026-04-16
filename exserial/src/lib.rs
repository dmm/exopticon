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

use std::convert::TryFrom;
use std::io::{self, Write};

//
use bincode::serialize;

use crate::models::CaptureMessage;

pub mod exlog;
pub mod models;

pub fn print_message(message: CaptureMessage) {
    let serialized = serialize(&message).expect("Unable to serialize message!");
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    // write length of message as big-endian u32 for framing
    let message_length = u32::try_from(serialized.len()).expect("Framed message too large!");
    let message_length_be = message_length.to_be_bytes();
    handle
        .write_all(&message_length_be)
        .expect("unable to write frame length!");

    // Write message
    handle
        .write_all(serialized.as_slice())
        .expect("unable to write frame!");
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
