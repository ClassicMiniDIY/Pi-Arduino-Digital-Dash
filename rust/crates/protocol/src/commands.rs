//! TunerStudio Command Protocol
//!
//! Implements the Speeduino-compatible command set for communication with TunerStudio.

use crate::och_block::OchBlock;

/// Signature string returned by QuerySignature command
/// Must match INI file: signature = "speeduino-travis"
pub const SIGNATURE: &[u8; 32] = b"speeduino-travis\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";

/// Version string returned by QueryVersion command
pub const VERSION: &[u8; 32] = b"Travis Digital Dash v1.0\0\0\0\0\0\0\0\0";

/// TunerStudio command types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// 'Q' - Query signature (returns 32-byte signature)
    QuerySignature,

    /// 'S' - Query version (returns 32-byte version string)
    QueryVersion,

    /// 'r' - Read OCH block (returns 87-byte sensor data)
    /// Requires 4-byte parameters (offset, count) - both ignored
    ReadOch,

    /// 'p' - Read parameter page (returns configuration data)
    /// Requires 6-byte parameters (page, offset, length)
    ReadPage,

    /// 'b' - Burn/write parameter page
    /// Requires 2-byte parameter (page number)
    BurnPage,

    /// 'd' - Get CRC/data checksum
    /// Requires 2-byte parameter
    /// Returns 4-byte CRC (currently always 0)
    GetCrc,

    /// 'F' - Feature flags
    /// Returns 3 zero bytes (legacy compatibility)
    Flags,

    /// Unknown command byte
    Unknown(u8),
}

impl Command {
    /// Parse a command byte into a Command enum
    pub fn from_byte(b: u8) -> Self {
        match b {
            b'Q' => Command::QuerySignature,
            b'S' => Command::QueryVersion,
            b'r' => Command::ReadOch,
            b'p' => Command::ReadPage,
            b'b' => Command::BurnPage,
            b'd' => Command::GetCrc,
            b'F' => Command::Flags,
            _ => Command::Unknown(b),
        }
    }

    /// Get the command byte
    pub fn to_byte(self) -> u8 {
        match self {
            Command::QuerySignature => b'Q',
            Command::QueryVersion => b'S',
            Command::ReadOch => b'r',
            Command::ReadPage => b'p',
            Command::BurnPage => b'b',
            Command::GetCrc => b'd',
            Command::Flags => b'F',
            Command::Unknown(b) => b,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_parsing() {
        assert_eq!(Command::from_byte(b'Q'), Command::QuerySignature);
        assert_eq!(Command::from_byte(b'S'), Command::QueryVersion);
        assert_eq!(Command::from_byte(b'r'), Command::ReadOch);
        assert_eq!(Command::from_byte(b'p'), Command::ReadPage);
        assert_eq!(Command::from_byte(b'b'), Command::BurnPage);
        assert_eq!(Command::from_byte(b'd'), Command::GetCrc);
        assert_eq!(Command::from_byte(b'F'), Command::Flags);

        if let Command::Unknown(b) = Command::from_byte(b'X') {
            assert_eq!(b, b'X');
        } else {
            panic!("Expected Unknown command");
        }
    }

    #[test]
    fn test_command_to_byte() {
        assert_eq!(Command::QuerySignature.to_byte(), b'Q');
        assert_eq!(Command::ReadOch.to_byte(), b'r');
        assert_eq!(Command::Unknown(b'Z').to_byte(), b'Z');
    }

    #[test]
    fn test_signature_format() {
        // Signature must be exactly 32 bytes
        assert_eq!(SIGNATURE.len(), 32);

        // First 16 bytes should be "speeduino-travis"
        assert_eq!(&SIGNATURE[..17], b"speeduino-travis\0");

        // Remaining bytes should be zero-padded
        for i in 17..32 {
            assert_eq!(SIGNATURE[i], 0);
        }
    }

    #[test]
    fn test_version_format() {
        // Version must be exactly 32 bytes
        assert_eq!(VERSION.len(), 32);

        // Should start with version string
        assert!(VERSION.starts_with(b"Travis Digital Dash v1.0"));

        // Should be null-terminated and zero-padded
        let null_pos = VERSION.iter().position(|&b| b == 0).unwrap();
        for i in null_pos..32 {
            assert_eq!(VERSION[i], 0);
        }
    }
}
