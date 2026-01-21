//! Fixed header for ELARA wire protocol
//!
//! Fixed header is 30 bytes:
//! - Byte 0: Version (4 bits) + Crypto Suite (4 bits)
//! - Byte 1: Flags
//! - Bytes 2-3: Header length (LE)
//! - Bytes 4-11: Session ID (LE)
//! - Bytes 12-19: Node ID (LE)
//! - Byte 20: Packet class
//! - Byte 21: Representation profile
//! - Bytes 22-25: Time hint (LE, signed)
//! - Bytes 26-29: Seq/Window (LE)

use elara_core::{ElaraError, ElaraResult, NodeId, PacketClass, RepresentationProfile, SessionId};

use crate::FrameFlags;

/// Fixed header size in bytes
pub const FIXED_HEADER_SIZE: usize = 30;

/// Current wire protocol version
pub const WIRE_VERSION: u8 = 0;

/// Crypto suite identifiers
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum CryptoSuite {
    /// X25519 + ChaCha20-Poly1305 + Ed25519
    Suite0 = 0,
    /// X25519 + AES-256-GCM + Ed25519
    Suite1 = 1,
    /// Reserved for post-quantum
    Suite2 = 2,
}

impl CryptoSuite {
    pub fn from_nibble(n: u8) -> Option<Self> {
        match n {
            0 => Some(CryptoSuite::Suite0),
            1 => Some(CryptoSuite::Suite1),
            2 => Some(CryptoSuite::Suite2),
            _ => None,
        }
    }

    #[inline]
    pub fn to_nibble(self) -> u8 {
        self as u8
    }
}

impl Default for CryptoSuite {
    fn default() -> Self {
        CryptoSuite::Suite0
    }
}

/// Fixed header structure
#[derive(Clone, Debug)]
pub struct FixedHeader {
    /// Wire protocol version (4 bits, 0-15)
    pub version: u8,
    /// Crypto suite identifier (4 bits, 0-15)
    pub crypto_suite: CryptoSuite,
    /// Frame flags
    pub flags: FrameFlags,
    /// Total header length (fixed + extensions)
    pub header_len: u16,
    /// Session ID
    pub session_id: SessionId,
    /// Sender node ID
    pub node_id: NodeId,
    /// Packet class
    pub class: PacketClass,
    /// Representation profile hint
    pub profile: RepresentationProfile,
    /// Time hint (offset relative to τs in 100μs units)
    pub time_hint: i32,
    /// Sequence number (upper 16 bits) + window bitmap (lower 16 bits)
    pub seq_window: u32,
}

impl FixedHeader {
    /// Create a new header with default values
    pub fn new(session_id: SessionId, node_id: NodeId) -> Self {
        FixedHeader {
            version: WIRE_VERSION,
            crypto_suite: CryptoSuite::default(),
            flags: FrameFlags::NONE,
            header_len: FIXED_HEADER_SIZE as u16,
            session_id,
            node_id,
            class: PacketClass::Core,
            profile: RepresentationProfile::Textual,
            time_hint: 0,
            seq_window: 0,
        }
    }

    /// Get sequence number (upper 16 bits)
    #[inline]
    pub fn seq(&self) -> u16 {
        (self.seq_window >> 16) as u16
    }

    /// Get window bitmap (lower 16 bits)
    #[inline]
    pub fn window(&self) -> u16 {
        (self.seq_window & 0xFFFF) as u16
    }

    /// Set sequence number
    #[inline]
    pub fn set_seq(&mut self, seq: u16) {
        self.seq_window = ((seq as u32) << 16) | (self.seq_window & 0xFFFF);
    }

    /// Set window bitmap
    #[inline]
    pub fn set_window(&mut self, window: u16) {
        self.seq_window = (self.seq_window & 0xFFFF0000) | (window as u32);
    }

    /// Parse header from bytes
    pub fn parse(buf: &[u8]) -> ElaraResult<Self> {
        if buf.len() < FIXED_HEADER_SIZE {
            return Err(ElaraError::BufferTooShort {
                expected: FIXED_HEADER_SIZE,
                actual: buf.len(),
            });
        }

        // Byte 0: Version + Crypto Suite
        let version = buf[0] >> 4;
        let crypto_suite = CryptoSuite::from_nibble(buf[0] & 0x0F)
            .ok_or_else(|| ElaraError::InvalidWireFormat("Unknown crypto suite".into()))?;

        // Byte 1: Flags
        let flags = FrameFlags::new(buf[1]);

        // Bytes 2-3: Header length
        let header_len = u16::from_le_bytes([buf[2], buf[3]]);

        // Bytes 4-11: Session ID
        let session_id = SessionId::from_bytes(buf[4..12].try_into().unwrap());

        // Bytes 12-19: Node ID
        let node_id = NodeId::from_bytes(buf[12..20].try_into().unwrap());

        // Byte 20: Class
        let class = PacketClass::from_byte(buf[20])
            .ok_or_else(|| ElaraError::UnknownPacketClass(buf[20]))?;

        // Byte 21: Profile
        let profile = RepresentationProfile::from_byte(buf[21]);

        // Bytes 22-25: Time hint
        let time_hint = i32::from_le_bytes(buf[22..26].try_into().unwrap());

        // Bytes 26-29: Seq/Window
        let seq_window = u32::from_le_bytes(buf[26..30].try_into().unwrap());

        Ok(FixedHeader {
            version,
            crypto_suite,
            flags,
            header_len,
            session_id,
            node_id,
            class,
            profile,
            time_hint,
            seq_window,
        })
    }

    /// Serialize header to bytes
    pub fn serialize(&self, buf: &mut [u8]) -> ElaraResult<()> {
        if buf.len() < FIXED_HEADER_SIZE {
            return Err(ElaraError::BufferTooShort {
                expected: FIXED_HEADER_SIZE,
                actual: buf.len(),
            });
        }

        // Byte 0: Version + Crypto Suite
        buf[0] = (self.version << 4) | self.crypto_suite.to_nibble();

        // Byte 1: Flags
        buf[1] = self.flags.0;

        // Bytes 2-3: Header length
        buf[2..4].copy_from_slice(&self.header_len.to_le_bytes());

        // Bytes 4-11: Session ID
        buf[4..12].copy_from_slice(&self.session_id.to_bytes());

        // Bytes 12-19: Node ID
        buf[12..20].copy_from_slice(&self.node_id.to_bytes());

        // Byte 20: Class
        buf[20] = self.class.to_byte();

        // Byte 21: Profile
        buf[21] = self.profile.to_byte();

        // Bytes 22-25: Time hint
        buf[22..26].copy_from_slice(&self.time_hint.to_le_bytes());

        // Bytes 26-29: Seq/Window
        buf[26..30].copy_from_slice(&self.seq_window.to_le_bytes());

        Ok(())
    }

    /// Serialize header to a new Vec
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = vec![0u8; FIXED_HEADER_SIZE];
        self.serialize(&mut buf).unwrap();
        buf
    }
}

impl Default for FixedHeader {
    fn default() -> Self {
        FixedHeader::new(SessionId::ZERO, NodeId::ZERO)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_roundtrip() {
        let header = FixedHeader {
            version: WIRE_VERSION,
            crypto_suite: CryptoSuite::Suite0,
            flags: FrameFlags(FrameFlags::MULTIPATH | FrameFlags::PRIORITY),
            header_len: 30,
            session_id: SessionId::new(0xDEADBEEF_CAFEBABE),
            node_id: NodeId::new(0x12345678_9ABCDEF0),
            class: PacketClass::Perceptual,
            profile: RepresentationProfile::VoiceMinimal,
            time_hint: -12345,
            seq_window: 0x00010002,
        };

        let bytes = header.to_bytes();
        assert_eq!(bytes.len(), FIXED_HEADER_SIZE);

        let parsed = FixedHeader::parse(&bytes).unwrap();

        assert_eq!(parsed.version, header.version);
        assert_eq!(parsed.crypto_suite, header.crypto_suite);
        assert_eq!(parsed.flags, header.flags);
        assert_eq!(parsed.header_len, header.header_len);
        assert_eq!(parsed.session_id, header.session_id);
        assert_eq!(parsed.node_id, header.node_id);
        assert_eq!(parsed.class, header.class);
        assert_eq!(parsed.profile, header.profile);
        assert_eq!(parsed.time_hint, header.time_hint);
        assert_eq!(parsed.seq_window, header.seq_window);
    }

    #[test]
    fn test_seq_window_accessors() {
        let mut header = FixedHeader::default();
        
        header.set_seq(0x1234);
        header.set_window(0x5678);
        
        assert_eq!(header.seq(), 0x1234);
        assert_eq!(header.window(), 0x5678);
        assert_eq!(header.seq_window, 0x12345678);
    }

    #[test]
    fn test_header_too_short() {
        let buf = [0u8; 20]; // Too short
        let result = FixedHeader::parse(&buf);
        assert!(matches!(result, Err(ElaraError::BufferTooShort { .. })));
    }

    #[test]
    fn test_header_size() {
        assert_eq!(FIXED_HEADER_SIZE, 30);
    }
}
