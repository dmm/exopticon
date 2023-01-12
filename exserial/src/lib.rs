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
use std::ffi::CStr;
use std::io::{self, Write};
use std::slice;

//
use bincode::serialize;
use libc::c_char;

use crate::models::{CaptureMessage, FrameMessage};

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

/// Take FrameMessage struct and write a framed message to stdout
///
/// # Safety
///
/// frame pointer must be to a valid, aligned FrameMessage.
///
#[no_mangle]
pub unsafe extern "C" fn send_frame_message(frame: *const FrameMessage) {
    let frame = {
        assert!(!frame.is_null());
        &*frame
    };
    let jpeg = {
        assert!(!frame.jpeg.is_null());
        slice::from_raw_parts(frame.jpeg, frame.jpeg_size as usize)
    };

    let frame = CaptureMessage::Frame {
        jpeg: jpeg.to_vec(),
        offset: frame.offset,
        unscaled_height: frame.unscaled_height,
        unscaled_width: frame.unscaled_width,
    };
    print_message(frame);
}

/// Take FrameMessage struct and write a framed scaled, message to stdout
///
/// # Safety
///
/// frame pointer must be to a valid, aligned FrameMessage.
///
#[no_mangle]
pub unsafe extern "C" fn send_scaled_frame_message(frame: *const FrameMessage, _height: i32) {
    let frame = {
        assert!(!frame.is_null());
        &*frame
    };
    let jpeg = {
        assert!(!frame.jpeg.is_null());
        slice::from_raw_parts(frame.jpeg, frame.jpeg_size as usize)
    };

    let frame = CaptureMessage::ScaledFrame {
        jpeg: jpeg.to_vec(),
        offset: frame.offset,
        unscaled_height: frame.unscaled_height,
        unscaled_width: frame.unscaled_width,
    };

    print_message(frame);
}

/// Send a packet of compressed video
///
/// # Safety
///
/// data pointer must be valid and point to an allocation `size` in
/// length
///
#[no_mangle]
pub unsafe extern "C" fn send_packet(data: *const u8, size: u64) {
    assert!(!data.is_null());

    let data_slice = slice::from_raw_parts(data, size as usize);
    let message = CaptureMessage::Packet {
        //        encoding: models::PacketEncoding::H264,
        data: data_slice.to_vec(),
    };

    print_message(message);
}

/// Send a message signaling a new file was created.
///
/// # Safety
///
/// filename and iso_begin_time must be null-terminated character arrays.
///
#[no_mangle]
pub unsafe extern "C" fn send_new_file_message(
    filename: *const c_char,
    iso_begin_time: *const c_char,
) {
    let filename = {
        assert!(!filename.is_null());

        CStr::from_ptr(filename).to_string_lossy().into_owned()
    };

    let iso_begin_time = {
        assert!(!iso_begin_time.is_null());

        CStr::from_ptr(iso_begin_time)
            .to_string_lossy()
            .into_owned()
    };

    let message = CaptureMessage::NewFile {
        filename,
        begin_time: iso_begin_time,
    };

    print_message(message);
}

/// Send a message signaling the closing of a file.
///
/// # Safety
///
/// filename and iso_end_time must be null-terminated character arrays.
///
#[no_mangle]
pub unsafe extern "C" fn send_end_file_message(
    filename: *const c_char,
    iso_end_time: *const c_char,
) {
    let filename = {
        assert!(!filename.is_null());

        CStr::from_ptr(filename).to_string_lossy().into_owned()
    };

    let iso_end_time = {
        assert!(!iso_end_time.is_null());

        CStr::from_ptr(iso_end_time).to_string_lossy().into_owned()
    };

    let message = CaptureMessage::EndFile {
        filename,
        end_time: iso_end_time,
    };

    print_message(message);
}

/// Send a log message
///
/// # Safety
///
/// message must be a null-terminated character arrays.
///
#[no_mangle]
pub unsafe extern "C" fn send_log_message(_level: i32, message: *const c_char) {
    let message = {
        assert!(!message.is_null());

        CStr::from_ptr(message).to_string_lossy().into_owned()
    };

    let capture_message = CaptureMessage::Log { message };

    print_message(capture_message);
}

#[no_mangle]
pub unsafe extern "C" fn send_metric_report(
    label: *const c_char,
    values: *const f64,
    value_count: u64,
) {
    assert!(!values.is_null());
    assert!(value_count > 0);

    let label = {
        assert!(!label.is_null());
        CStr::from_ptr(label).to_string_lossy().into_owned()
    };

    let values_slice = slice::from_raw_parts(values, value_count as usize);

    let message = CaptureMessage::Metric {
        label,
        values: values_slice.to_vec(),
    };

    print_message(message);
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
