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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PacketEncoding {
    /// H264 Video
    H264,
    /// μ-law PCM Audio
    PCMU,
}

#[derive(Debug, Deserialize, Serialize)]
/// Message from captureworker
pub enum CaptureMessage {
    /// Log
    Log { level: log::Level, message: String },
    /// Packet of compress audio/video
    Packet {
        /// compression codec used
        encoding: PacketEncoding,
        #[serde(with = "serde_bytes")]
        /// compressed packet data
        data: Vec<u8>,
        /// 90kHz timestamp
        timestamp: i64,
        /// duration in microseconds
        duration: i64,
    },
    /// New file indication
    NewFile {
        filename: String,
        begin_time: String,
    },
    /// File closed indication
    EndFile { filename: String, end_time: String },
    /// metric report
    Metric { label: String, values: Vec<f64> },
}
