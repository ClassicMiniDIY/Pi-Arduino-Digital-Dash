//! Shared protocol types

/// Result type for protocol operations
pub type ProtocolResult<T> = Result<T, ProtocolError>;

/// Protocol error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolError {
    /// Timeout waiting for data
    Timeout,

    /// Invalid command received
    InvalidCommand,

    /// Buffer overflow
    BufferOverflow,

    /// Checksum mismatch
    ChecksumError,

    /// Generic I/O error
    IoError,
}

impl ProtocolError {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProtocolError::Timeout => "Timeout",
            ProtocolError::InvalidCommand => "Invalid command",
            ProtocolError::BufferOverflow => "Buffer overflow",
            ProtocolError::ChecksumError => "Checksum error",
            ProtocolError::IoError => "I/O error",
        }
    }
}

/// Trait for reading/writing protocol data
/// (Platform-specific implementations will provide this)
pub trait SerialPort {
    /// Check how many bytes are available to read
    fn available(&self) -> usize;

    /// Read a single byte (non-blocking)
    /// Returns None if no data available
    fn read_byte(&mut self) -> Option<u8>;

    /// Write bytes to the port
    fn write(&mut self, data: &[u8]);

    /// Flush any pending writes
    fn flush(&mut self);
}
