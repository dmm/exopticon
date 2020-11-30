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

use serde::{Deserialize, Serialize};

#[repr(C)]
pub struct FrameMessage {
    pub jpeg: *const u8,
    pub jpeg_size: i32,
    pub offset: i64,
    pub unscaled_height: i32,
    pub unscaled_width: i32,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum CaptureMessage {
    Log {
        message: String,
    },
    Frame {
        #[serde(with = "serde_bytes")]
        jpeg: Vec<u8>,
        offset: i64,
        unscaled_width: i32,
        unscaled_height: i32,
    },
    ScaledFrame {
        #[serde(with = "serde_bytes")]
        jpeg: Vec<u8>,
        offset: i64,
        unscaled_width: i32,
        unscaled_height: i32,
    },
    NewFile {
        filename: String,
        begin_time: String,
    },
    EndFile {
        filename: String,
        end_time: String,
    },
}
