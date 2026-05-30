use thiserror::Error;

#[derive(Debug, Error)]
pub enum ObdError {
    #[error("Serial port error: {0}")]
    Serial(#[from] serialport::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("ELM327 did not respond within timeout")]
    Timeout,

    #[error("ELM327 error response: {0}")]
    ElmError(String),

    #[error("No data returned by ECU")]
    NoData,

    #[error("Invalid hex in response: {0}")]
    InvalidHex(String),

    #[error("Unexpected response format: {0}")]
    ParseError(String),

    #[error("Unsupported PID: 0x{0:02X}")]
    UnsupportedPid(u8),
}

pub type Result<T> = std::result::Result<T, ObdError>;
