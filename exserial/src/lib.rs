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

use serde::{Serialize, de::DeserializeOwned};

use crate::models::CaptureMessage;

pub mod exlog;
pub mod models;

pub fn write_framed<W, T>(writer: &mut W, message: &T) -> io::Result<()>
where
    W: Write,
    T: Serialize,
{
    let serialized = postcard::to_allocvec(message).expect("Unable to serialize message!");

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

    deserialize_message(&serialized)
}

pub fn deserialize_message<T>(serialized: &[u8]) -> io::Result<T>
where
    T: DeserializeOwned,
{
    postcard::from_bytes(serialized).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
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
    use crate::models::{CaptureCommand, CaptureMessage, PacketEncoding};

    #[test]
    fn all_capture_message_variants_round_trip() {
        let messages = [
            CaptureMessage::Log {
                level: log::Level::Info,
                message: "worker started".to_string(),
            },
            CaptureMessage::Packet {
                encoding: PacketEncoding::H264,
                data: vec![0, 1, 127, 128, 255],
                timestamp: -90_000,
                duration: 33_333,
            },
            CaptureMessage::Packet {
                encoding: PacketEncoding::PCMU,
                data: vec![255; 160],
                timestamp: 90_000,
                duration: 20_000,
            },
            CaptureMessage::ReserveFile,
            CaptureMessage::FileOpened {
                video_unit_id: 11,
                video_file_id: 13,
                filename: "/tmp/video.mkv".to_string(),
                begin_time: "2026-10-27T00:00:00Z".to_string(),
            },
            CaptureMessage::EndFile {
                video_unit_id: 11,
                video_file_id: 13,
                filename: "/tmp/video.mkv".to_string(),
                end_time: "2026-10-27T00:01:00Z".to_string(),
            },
            CaptureMessage::Metric {
                label: "capture".to_string(),
                values: vec![0.0, -1.5, 123.25],
            },
        ];

        for message in messages {
            let frame = crate::serialize_frame(&message).expect("message serialized");
            let length = u32::from_be_bytes(frame[..4].try_into().unwrap()) as usize;
            assert_eq!(length, frame.len() - 4);

            // The actor decodes the payload supplied by LengthDelimitedCodec.
            let payload: CaptureMessage =
                crate::deserialize_message(&frame[4..]).expect("payload deserialized");
            assert_eq!(payload, message);

            let framed: CaptureMessage =
                crate::read_framed(&mut frame.as_slice()).expect("frame deserialized");
            assert_eq!(framed, message);
        }
    }

    #[test]
    fn failed_file_reservation_round_trips() {
        let command = CaptureCommand::FileReservationFailed {
            message: "storage unavailable".to_string(),
        };
        let frame = crate::serialize_frame(&command).expect("command serialized");
        let decoded: CaptureCommand =
            crate::read_framed(&mut frame.as_slice()).expect("command deserialized");
        assert_eq!(decoded, command);
    }

    #[test]
    fn invalid_payload_is_an_io_error() {
        let error = crate::deserialize_message::<CaptureMessage>(&[]).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    }

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
