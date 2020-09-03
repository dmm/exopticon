use thiserror::Error;

#[derive(Error, Debug)]
pub enum SnapshotError {
    /// Represents badness
    #[error("invalid")]
    Invalid,

    #[error("state")]
    StateError(#[from] gst::StateChangeError),

    #[error("bad")]
    VideoError(#[from] gst::glib::BoolError),

    #[error("snake")]
    InitError(#[from] gst::glib::Error),

    /// Represents a failure to read from input.
    #[error("Read error")]
    ReadError { source: std::io::Error },

    /// Represents all other cases of `std::io::Error`.
    #[error(transparent)]
    IOError(#[from] std::io::Error),
}
