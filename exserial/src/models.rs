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
