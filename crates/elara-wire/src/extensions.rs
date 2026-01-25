//! Variable header extensions (TLV format)
//!
//! Extensions allow protocol evolution without breaking compatibility.
//! Format: [TYPE:1][LEN:1][VALUE:LEN]

use elara_core::{ElaraError, ElaraResult};

/// Extension type identifiers
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ExtensionType {
    /// Current ratchet epoch (4 bytes)
    RatchetId = 0x01,
    /// Key generation number (2 bytes)
    KeyEpoch = 0x02,
    /// Node role in swarm (1 byte)
    SwarmRole = 0x03,
    /// Hop count for relayed packets (1 byte)
    RelayHop = 0x04,
    /// Interest subscription bitmap (8 bytes)
    InterestMask = 0x05,
    /// FEC group identifier (2 bytes)
    RedundancyGroup = 0x06,
    /// Compression algorithm used (1 byte)
    CompressionHint = 0x07,
    /// Fragment index + total (4 bytes)
    FragmentInfo = 0x08,
    /// Multi-path identifier (2 bytes)
    PathId = 0x09,
    /// Scheduling priority (1 byte)
    PriorityHint = 0x0A,
    /// Vector clock reference (variable)
    CausalityRef = 0x0B,
    /// End of extensions marker
    End = 0xFF,
}

impl ExtensionType {
    pub fn from_byte(b: u8) -> Option<Self> {
        match b {
            0x01 => Some(ExtensionType::RatchetId),
            0x02 => Some(ExtensionType::KeyEpoch),
            0x03 => Some(ExtensionType::SwarmRole),
            0x04 => Some(ExtensionType::RelayHop),
            0x05 => Some(ExtensionType::InterestMask),
            0x06 => Some(ExtensionType::RedundancyGroup),
            0x07 => Some(ExtensionType::CompressionHint),
            0x08 => Some(ExtensionType::FragmentInfo),
            0x09 => Some(ExtensionType::PathId),
            0x0A => Some(ExtensionType::PriorityHint),
            0x0B => Some(ExtensionType::CausalityRef),
            0xFF => Some(ExtensionType::End),
            _ => None,
        }
    }
}

/// Swarm role for a node
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum SwarmRole {
    /// Regular participant
    Participant = 0x00,
    /// Relay node (forwards packets)
    Relay = 0x01,
    /// Aggregator (summarizes state)
    Aggregator = 0x02,
    /// Observer (read-only)
    Observer = 0x03,
    /// Source (livestream origin)
    Source = 0x04,
}

impl SwarmRole {
    pub fn from_byte(b: u8) -> Option<Self> {
        match b {
            0x00 => Some(SwarmRole::Participant),
            0x01 => Some(SwarmRole::Relay),
            0x02 => Some(SwarmRole::Aggregator),
            0x03 => Some(SwarmRole::Observer),
            0x04 => Some(SwarmRole::Source),
            _ => None,
        }
    }
}

/// Fragment information
#[derive(Clone, Copy, Debug, Default)]
pub struct FragmentInfo {
    /// Fragment index (0-based)
    pub index: u16,
    /// Total number of fragments
    pub total: u16,
}

impl FragmentInfo {
    pub fn new(index: u16, total: u16) -> Self {
        FragmentInfo { index, total }
    }

    pub fn to_bytes(&self) -> [u8; 4] {
        let mut buf = [0u8; 4];
        buf[0..2].copy_from_slice(&self.index.to_le_bytes());
        buf[2..4].copy_from_slice(&self.total.to_le_bytes());
        buf
    }

    pub fn from_bytes(buf: &[u8; 4]) -> Self {
        FragmentInfo {
            index: u16::from_le_bytes([buf[0], buf[1]]),
            total: u16::from_le_bytes([buf[2], buf[3]]),
        }
    }

    pub fn is_first(&self) -> bool {
        self.index == 0
    }

    pub fn is_last(&self) -> bool {
        self.index + 1 == self.total
    }
}

/// Parsed extensions
#[derive(Clone, Debug, Default)]
pub struct Extensions {
    pub ratchet_id: Option<u32>,
    pub key_epoch: Option<u16>,
    pub swarm_role: Option<SwarmRole>,
    pub relay_hop: Option<u8>,
    pub interest_mask: Option<u64>,
    pub redundancy_group: Option<u16>,
    pub compression_hint: Option<u8>,
    pub fragment_info: Option<FragmentInfo>,
    pub path_id: Option<u16>,
    pub priority_hint: Option<u8>,
    pub causality_ref: Option<Vec<u8>>,
}

impl Extensions {
    pub fn new() -> Self {
        Extensions::default()
    }

    /// Parse extensions from buffer
    /// Returns (Extensions, bytes consumed)
    pub fn parse(buf: &[u8], max_len: usize) -> ElaraResult<(Self, usize)> {
        let mut extensions = Extensions::new();
        let mut offset = 0;

        while offset < max_len && offset < buf.len() {
            let ext_type = buf[offset];

            // End marker
            if ext_type == ExtensionType::End as u8 {
                offset += 1;
                break;
            }

            // Need at least type + length
            if offset + 2 > buf.len() {
                break;
            }

            let ext_len = buf[offset + 1] as usize;

            // Check bounds
            if offset + 2 + ext_len > buf.len() {
                return Err(ElaraError::InvalidWireFormat(
                    "Extension length exceeds buffer".into(),
                ));
            }

            let value = &buf[offset + 2..offset + 2 + ext_len];

            // Parse known extensions
            match ExtensionType::from_byte(ext_type) {
                Some(ExtensionType::RatchetId) if ext_len == 4 => {
                    extensions.ratchet_id = Some(u32::from_le_bytes(value.try_into().unwrap()));
                }
                Some(ExtensionType::KeyEpoch) if ext_len == 2 => {
                    extensions.key_epoch = Some(u16::from_le_bytes(value.try_into().unwrap()));
                }
                Some(ExtensionType::SwarmRole) if ext_len == 1 => {
                    extensions.swarm_role = SwarmRole::from_byte(value[0]);
                }
                Some(ExtensionType::RelayHop) if ext_len == 1 => {
                    extensions.relay_hop = Some(value[0]);
                }
                Some(ExtensionType::InterestMask) if ext_len == 8 => {
                    extensions.interest_mask = Some(u64::from_le_bytes(value.try_into().unwrap()));
                }
                Some(ExtensionType::RedundancyGroup) if ext_len == 2 => {
                    extensions.redundancy_group =
                        Some(u16::from_le_bytes(value.try_into().unwrap()));
                }
                Some(ExtensionType::CompressionHint) if ext_len == 1 => {
                    extensions.compression_hint = Some(value[0]);
                }
                Some(ExtensionType::FragmentInfo) if ext_len == 4 => {
                    extensions.fragment_info =
                        Some(FragmentInfo::from_bytes(value.try_into().unwrap()));
                }
                Some(ExtensionType::PathId) if ext_len == 2 => {
                    extensions.path_id = Some(u16::from_le_bytes(value.try_into().unwrap()));
                }
                Some(ExtensionType::PriorityHint) if ext_len == 1 => {
                    extensions.priority_hint = Some(value[0]);
                }
                Some(ExtensionType::CausalityRef) => {
                    extensions.causality_ref = Some(value.to_vec());
                }
                _ => {
                    // Unknown extension, skip
                }
            }

            offset += 2 + ext_len;
        }

        Ok((extensions, offset))
    }

    /// Serialize extensions to buffer
    /// Returns bytes written
    pub fn serialize(&self, buf: &mut [u8]) -> ElaraResult<usize> {
        let mut offset = 0;

        // Helper macro for writing extensions
        macro_rules! write_ext {
            ($type:expr, $value:expr) => {
                if let Some(ref v) = $value {
                    let bytes = v;
                    if offset + 2 + bytes.len() > buf.len() {
                        return Err(ElaraError::BufferTooShort {
                            expected: offset + 2 + bytes.len(),
                            actual: buf.len(),
                        });
                    }
                    buf[offset] = $type as u8;
                    buf[offset + 1] = bytes.len() as u8;
                    buf[offset + 2..offset + 2 + bytes.len()].copy_from_slice(bytes);
                    offset += 2 + bytes.len();
                }
            };
        }

        if let Some(v) = self.ratchet_id {
            write_ext!(ExtensionType::RatchetId, Some(v.to_le_bytes().to_vec()));
        }
        if let Some(v) = self.key_epoch {
            write_ext!(ExtensionType::KeyEpoch, Some(v.to_le_bytes().to_vec()));
        }
        if let Some(v) = self.swarm_role {
            write_ext!(ExtensionType::SwarmRole, Some(vec![v as u8]));
        }
        if let Some(v) = self.relay_hop {
            write_ext!(ExtensionType::RelayHop, Some(vec![v]));
        }
        if let Some(v) = self.interest_mask {
            write_ext!(ExtensionType::InterestMask, Some(v.to_le_bytes().to_vec()));
        }
        if let Some(v) = self.redundancy_group {
            write_ext!(
                ExtensionType::RedundancyGroup,
                Some(v.to_le_bytes().to_vec())
            );
        }
        if let Some(v) = self.compression_hint {
            write_ext!(ExtensionType::CompressionHint, Some(vec![v]));
        }
        if let Some(ref v) = self.fragment_info {
            write_ext!(ExtensionType::FragmentInfo, Some(v.to_bytes().to_vec()));
        }
        if let Some(v) = self.path_id {
            write_ext!(ExtensionType::PathId, Some(v.to_le_bytes().to_vec()));
        }
        if let Some(v) = self.priority_hint {
            write_ext!(ExtensionType::PriorityHint, Some(vec![v]));
        }
        if let Some(ref v) = self.causality_ref {
            write_ext!(ExtensionType::CausalityRef, Some(v.clone()));
        }

        // Write end marker
        if offset < buf.len() {
            buf[offset] = ExtensionType::End as u8;
            offset += 1;
        }

        Ok(offset)
    }

    /// Check if any extensions are present
    pub fn is_empty(&self) -> bool {
        self.ratchet_id.is_none()
            && self.key_epoch.is_none()
            && self.swarm_role.is_none()
            && self.relay_hop.is_none()
            && self.interest_mask.is_none()
            && self.redundancy_group.is_none()
            && self.compression_hint.is_none()
            && self.fragment_info.is_none()
            && self.path_id.is_none()
            && self.priority_hint.is_none()
            && self.causality_ref.is_none()
    }

    /// Calculate serialized size
    pub fn serialized_size(&self) -> usize {
        let mut size = 1; // End marker

        if self.ratchet_id.is_some() {
            size += 6;
        } // type + len + 4 bytes
        if self.key_epoch.is_some() {
            size += 4;
        } // type + len + 2 bytes
        if self.swarm_role.is_some() {
            size += 3;
        } // type + len + 1 byte
        if self.relay_hop.is_some() {
            size += 3;
        }
        if self.interest_mask.is_some() {
            size += 10;
        } // type + len + 8 bytes
        if self.redundancy_group.is_some() {
            size += 4;
        }
        if self.compression_hint.is_some() {
            size += 3;
        }
        if self.fragment_info.is_some() {
            size += 6;
        }
        if self.path_id.is_some() {
            size += 4;
        }
        if self.priority_hint.is_some() {
            size += 3;
        }
        if let Some(ref v) = self.causality_ref {
            size += 2 + v.len();
        }

        size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extensions_roundtrip() {
        let mut ext = Extensions::new();
        ext.ratchet_id = Some(0x12345678);
        ext.key_epoch = Some(42);
        ext.swarm_role = Some(SwarmRole::Relay);
        ext.fragment_info = Some(FragmentInfo::new(2, 5));

        let mut buf = vec![0u8; 256];
        let written = ext.serialize(&mut buf).unwrap();

        let (parsed, consumed) = Extensions::parse(&buf, written).unwrap();

        assert_eq!(parsed.ratchet_id, ext.ratchet_id);
        assert_eq!(parsed.key_epoch, ext.key_epoch);
        assert_eq!(parsed.swarm_role, ext.swarm_role);
        assert_eq!(parsed.fragment_info.unwrap().index, 2);
        assert_eq!(parsed.fragment_info.unwrap().total, 5);
        assert_eq!(consumed, written);
    }

    #[test]
    fn test_empty_extensions() {
        let ext = Extensions::new();
        assert!(ext.is_empty());

        let mut buf = vec![0u8; 16];
        let written = ext.serialize(&mut buf).unwrap();

        // Should just be end marker
        assert_eq!(written, 1);
        assert_eq!(buf[0], ExtensionType::End as u8);
    }

    #[test]
    fn test_fragment_info() {
        let frag = FragmentInfo::new(3, 10);
        assert!(!frag.is_first());
        assert!(!frag.is_last());

        let first = FragmentInfo::new(0, 10);
        assert!(first.is_first());

        let last = FragmentInfo::new(9, 10);
        assert!(last.is_last());
    }
}
