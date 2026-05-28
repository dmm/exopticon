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
use std::io::{self, Read, Write};

use bincode::{deserialize, serialize};
use serde::{Serialize, de::DeserializeOwned};

use crate::models::CaptureMessage;

pub mod exlog;
pub mod models;

pub fn write_framed<W, T>(writer: &mut W, message: &T) -> io::Result<()>
where
    W: Write,
    T: Serialize,
{
    let serialized = serialize(message).expect("Unable to serialize message!");

    // write length of message as big-endian u32 for framing
    let message_length = u32::try_from(serialized.len()).expect("Framed message too large!");
    let message_length_be = message_length.to_be_bytes();
    writer.write_all(&message_length_be)?;

    // Write message
    writer.write_all(serialized.as_slice())?;
    writer.flush()
}

pub fn read_framed<R, T>(reader: &mut R) -> io::Result<T>
where
    R: Read,
    T: DeserializeOwned,
{
    let mut message_length = [0u8; 4];
    reader.read_exact(&mut message_length)?;
    let message_length = u32::from_be_bytes(message_length);
    let message_length =
        usize::try_from(message_length).expect("usize overflow reading frame length");
    let mut serialized = vec![0; message_length];
    reader.read_exact(&mut serialized)?;

    deserialize(&serialized).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

pub fn serialize_frame<T>(message: &T) -> io::Result<Vec<u8>>
where
    T: Serialize,
{
    let mut frame = Vec::new();
    write_framed(&mut frame, message)?;
    Ok(frame)
}

pub fn print_message(message: CaptureMessage) {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    write_framed(&mut handle, &message).expect("unable to write frame");
}

#[cfg(test)]
mod tests {
    use crate::models::{CaptureCommand, CaptureMessage};

    #[test]
    fn framed_capture_message_round_trips() {
        let msg = CaptureMessage::ReserveFile;
        let mut frame = Vec::new();
        crate::write_framed(&mut frame, &msg).expect("message serialized");
        let round_trip: CaptureMessage =
            crate::read_framed(&mut frame.as_slice()).expect("message deserialized");

        match round_trip {
            CaptureMessage::ReserveFile => {}
            _ => panic!("unexpected capture message"),
        }
    }

    #[test]
    fn framed_capture_command_round_trips() {
        let msg = CaptureCommand::FileReserved {
            video_unit_id: 11,
            video_file_id: 13,
            filename: "/tmp/video.mkv".to_string(),
        };
        let mut frame = Vec::new();
        crate::write_framed(&mut frame, &msg).expect("command serialized");
        let round_trip: CaptureCommand =
            crate::read_framed(&mut frame.as_slice()).expect("command deserialized");

        match round_trip {
            CaptureCommand::FileReserved {
                video_unit_id,
                video_file_id,
                filename,
            } => {
                assert_eq!(video_unit_id, 11);
                assert_eq!(video_file_id, 13);
                assert_eq!(filename, "/tmp/video.mkv");
            }
            _ => panic!("unexpected capture command"),
        }
    }
}
