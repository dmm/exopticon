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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Hash, Serialize)]
pub enum FrameResolution {
    /// Standard definition frame, 480p
    SD,
    /// High definition frame, camera native resolution
    HD,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "kind")]
pub enum FrameSource {
    /// Camera with camera id
    Camera {
        /// id of camera
        #[serde(rename = "cameraId")]
        camera_id: i32,
    },
    /// Analysis Engine, with engine id
    AnalysisEngine {
        /// id of source analysis engine
        #[serde(rename = "analysisEngineId")]
        analysis_engine_id: i32,
        /// identifying tag for analysis frame
        tag: String,
    },
    /// Video Playback
    Playback {
        /// Playback id, must be unique per socket
        id: u64,
    },
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

#[derive(Serialize, Deserialize)]
pub struct CameraFrame {
    /// id of camera that produced frame
    pub camera_id: i32,
    /// jpeg image data
    pub jpeg: Vec<u8>,
    /// resolution of frame
    pub resolution: FrameResolution,
    /// source of frame
    pub source: FrameSource,
    /// offset from beginning of video unit
    pub offset: i64,
    /// original width of image
    pub unscaled_width: i32,
    /// original height of image,
    pub unscaled_height: i32,
}
