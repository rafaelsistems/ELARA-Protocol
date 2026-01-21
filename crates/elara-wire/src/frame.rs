//! Complete frame structure for ELARA wire protocol
//!
//! Frame = Fixed Header + Extensions + Encrypted Payload + Auth Tag

use elara_core::{ElaraError, ElaraResult};

use crate::{Extensions, FixedHeader, FIXED_HEADER_SIZE};

/// Auth tag size (AEAD)
pub const AUTH_TAG_SIZE: usize = 16;

/// Maximum frame size (MTU-friendly)
pub const MAX_FRAME_SIZE: usize = 1400;

/// Minimum frame size (header + tag)
pub const MIN_FRAME_SIZE: usize = FIXED_HEADER_SIZE + AUTH_TAG_SIZE;

/// Complete ELARA frame
#[derive(Clone, Debug)]
pub struct Frame {
    /// Fixed header
    pub header: FixedHeader,
    /// Variable extensions
    pub extensions: Extensions,
    /// Encrypted payload (events)
    pub payload: Vec<u8>,
    /// Authentication tag
    pub auth_tag: [u8; AUTH_TAG_SIZE],
}

impl Frame {
    /// Create a new frame
    pub fn new(header: FixedHeader) -> Self {
        Frame {
            header,
            extensions: Extensions::new(),
            payload: Vec::new(),
            auth_tag: [0u8; AUTH_TAG_SIZE],
        }
    }

    /// Parse frame from bytes (without decryption)
    pub fn parse(buf: &[u8]) -> ElaraResult<Self> {
        if buf.len() < MIN_FRAME_SIZE {
            return Err(ElaraError::BufferTooShort {
                expected: MIN_FRAME_SIZE,
                actual: buf.len(),
            });
        }

        // Parse fixed header
        let header = FixedHeader::parse(buf)?;

        // Validate header length
        if header.header_len as usize > buf.len() - AUTH_TAG_SIZE {
            return Err(ElaraError::InvalidWireFormat(
                "Header length exceeds frame".into(),
            ));
        }

        // Parse extensions if present
        let extensions = if header.flags.has_extension() && header.header_len as usize > FIXED_HEADER_SIZE {
            let ext_buf = &buf[FIXED_HEADER_SIZE..header.header_len as usize];
            let (ext, _) = Extensions::parse(ext_buf, ext_buf.len())?;
            ext
        } else {
            Extensions::new()
        };

        // Extract payload (still encrypted)
        let payload_start = header.header_len as usize;
        let payload_end = buf.len() - AUTH_TAG_SIZE;
        let payload = buf[payload_start..payload_end].to_vec();

        // Extract auth tag
        let mut auth_tag = [0u8; AUTH_TAG_SIZE];
        auth_tag.copy_from_slice(&buf[payload_end..]);

        Ok(Frame {
            header,
            extensions,
            payload,
            auth_tag,
        })
    }

    /// Serialize frame to bytes (payload should already be encrypted)
    pub fn serialize(&self) -> ElaraResult<Vec<u8>> {
        let ext_size = if self.extensions.is_empty() {
            0
        } else {
            self.extensions.serialized_size()
        };

        let total_size = FIXED_HEADER_SIZE + ext_size + self.payload.len() + AUTH_TAG_SIZE;

        if total_size > MAX_FRAME_SIZE {
            return Err(ElaraError::InvalidWireFormat(format!(
                "Frame too large: {} > {}",
                total_size, MAX_FRAME_SIZE
            )));
        }

        let mut buf = vec![0u8; total_size];

        // Write header
        let mut header = self.header.clone();
        header.header_len = (FIXED_HEADER_SIZE + ext_size) as u16;
        if !self.extensions.is_empty() {
            header.flags.set_extension(true);
        }
        header.serialize(&mut buf)?;

        // Write extensions
        if !self.extensions.is_empty() {
            self.extensions
                .serialize(&mut buf[FIXED_HEADER_SIZE..FIXED_HEADER_SIZE + ext_size])?;
        }

        // Write payload
        let payload_start = FIXED_HEADER_SIZE + ext_size;
        buf[payload_start..payload_start + self.payload.len()].copy_from_slice(&self.payload);

        // Write auth tag
        buf[total_size - AUTH_TAG_SIZE..].copy_from_slice(&self.auth_tag);

        Ok(buf)
    }

    /// Get the associated data for AEAD (header + extensions)
    pub fn associated_data(&self) -> Vec<u8> {
        let ext_size = if self.extensions.is_empty() {
            0
        } else {
            self.extensions.serialized_size()
        };

        let mut aad = vec![0u8; FIXED_HEADER_SIZE + ext_size];

        let mut header = self.header.clone();
        header.header_len = (FIXED_HEADER_SIZE + ext_size) as u16;
        header.serialize(&mut aad).unwrap();

        if !self.extensions.is_empty() {
            self.extensions
                .serialize(&mut aad[FIXED_HEADER_SIZE..])
                .unwrap();
        }

        aad
    }

    /// Calculate total frame size
    pub fn size(&self) -> usize {
        let ext_size = if self.extensions.is_empty() {
            0
        } else {
            self.extensions.serialized_size()
        };
        FIXED_HEADER_SIZE + ext_size + self.payload.len() + AUTH_TAG_SIZE
    }

    /// Check if frame fits in MTU
    pub fn fits_mtu(&self) -> bool {
        self.size() <= MAX_FRAME_SIZE
    }
}

/// Frame builder for convenient construction
pub struct FrameBuilder {
    frame: Frame,
}

impl FrameBuilder {
    pub fn new(header: FixedHeader) -> Self {
        FrameBuilder {
            frame: Frame::new(header),
        }
    }

    pub fn extensions(mut self, ext: Extensions) -> Self {
        self.frame.extensions = ext;
        self
    }

    pub fn payload(mut self, payload: Vec<u8>) -> Self {
        self.frame.payload = payload;
        self
    }

    pub fn auth_tag(mut self, tag: [u8; AUTH_TAG_SIZE]) -> Self {
        self.frame.auth_tag = tag;
        self
    }

    pub fn build(self) -> Frame {
        self.frame
    }
}

/// Frame slice for zero-copy parsing
pub struct FrameSlice<'a> {
    pub header: &'a [u8],
    pub extensions: &'a [u8],
    pub payload: &'a [u8],
    pub auth_tag: &'a [u8; AUTH_TAG_SIZE],
}

impl<'a> FrameSlice<'a> {
    /// Create a frame slice from a buffer (zero-copy)
    pub fn from_bytes(buf: &'a [u8]) -> ElaraResult<Self> {
        if buf.len() < MIN_FRAME_SIZE {
            return Err(ElaraError::BufferTooShort {
                expected: MIN_FRAME_SIZE,
                actual: buf.len(),
            });
        }

        // Read header length
        let header_len = u16::from_le_bytes([buf[2], buf[3]]) as usize;

        if header_len > buf.len() - AUTH_TAG_SIZE {
            return Err(ElaraError::InvalidWireFormat(
                "Header length exceeds frame".into(),
            ));
        }

        let header = &buf[0..FIXED_HEADER_SIZE];
        let extensions = &buf[FIXED_HEADER_SIZE..header_len];
        let payload = &buf[header_len..buf.len() - AUTH_TAG_SIZE];
        let auth_tag: &[u8; AUTH_TAG_SIZE] = buf[buf.len() - AUTH_TAG_SIZE..]
            .try_into()
            .map_err(|_| ElaraError::InvalidWireFormat("Invalid auth tag".into()))?;

        Ok(FrameSlice {
            header,
            extensions,
            payload,
            auth_tag,
        })
    }

    /// Parse the fixed header
    pub fn parse_header(&self) -> ElaraResult<FixedHeader> {
        FixedHeader::parse(self.header)
    }

    /// Parse extensions
    pub fn parse_extensions(&self) -> ElaraResult<Extensions> {
        if self.extensions.is_empty() {
            Ok(Extensions::new())
        } else {
            let (ext, _) = Extensions::parse(self.extensions, self.extensions.len())?;
            Ok(ext)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elara_core::{NodeId, PacketClass, SessionId};

    #[test]
    fn test_frame_roundtrip() {
        let header = FixedHeader {
            session_id: SessionId::new(12345),
            node_id: NodeId::new(67890),
            class: PacketClass::Perceptual,
            time_hint: 100,
            ..Default::default()
        };

        let mut ext = Extensions::new();
        ext.ratchet_id = Some(42);

        let frame = FrameBuilder::new(header)
            .extensions(ext)
            .payload(vec![1, 2, 3, 4, 5])
            .auth_tag([0xAA; AUTH_TAG_SIZE])
            .build();

        let bytes = frame.serialize().unwrap();
        let parsed = Frame::parse(&bytes).unwrap();

        assert_eq!(parsed.header.session_id, frame.header.session_id);
        assert_eq!(parsed.header.node_id, frame.header.node_id);
        assert_eq!(parsed.header.class, frame.header.class);
        assert_eq!(parsed.extensions.ratchet_id, Some(42));
        assert_eq!(parsed.payload, vec![1, 2, 3, 4, 5]);
        assert_eq!(parsed.auth_tag, [0xAA; AUTH_TAG_SIZE]);
    }

    #[test]
    fn test_frame_slice_zero_copy() {
        let header = FixedHeader::new(SessionId::new(1), NodeId::new(2));
        let frame = FrameBuilder::new(header)
            .payload(vec![10, 20, 30])
            .auth_tag([0xBB; AUTH_TAG_SIZE])
            .build();

        let bytes = frame.serialize().unwrap();
        let slice = FrameSlice::from_bytes(&bytes).unwrap();

        assert_eq!(slice.payload, &[10, 20, 30]);
        assert_eq!(slice.auth_tag, &[0xBB; AUTH_TAG_SIZE]);

        let parsed_header = slice.parse_header().unwrap();
        assert_eq!(parsed_header.session_id, SessionId::new(1));
    }

    #[test]
    fn test_frame_size_limits() {
        let header = FixedHeader::default();
        let frame = FrameBuilder::new(header)
            .payload(vec![0u8; MAX_FRAME_SIZE]) // Too large
            .build();

        assert!(!frame.fits_mtu());
        assert!(frame.serialize().is_err());
    }

    #[test]
    fn test_associated_data() {
        let header = FixedHeader::new(SessionId::new(100), NodeId::new(200));
        let mut ext = Extensions::new();
        ext.key_epoch = Some(5);

        let frame = FrameBuilder::new(header)
            .extensions(ext)
            .payload(vec![1, 2, 3])
            .build();

        let aad = frame.associated_data();

        // AAD should include header + extensions, but not payload or tag
        assert!(aad.len() > FIXED_HEADER_SIZE);
        assert!(aad.len() < frame.size());
    }
}
