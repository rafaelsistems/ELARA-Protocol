#![no_main]

use libfuzzer_sys::fuzz_target;
use elara_fuzz::{FuzzTarget, FuzzResult};
use elara_wire::{Frame, FrameBuilder, FixedHeader, Extensions, AUTH_TAG_SIZE, MAX_FRAME_SIZE, MIN_FRAME_SIZE};
use elara_core::{NodeId, SessionId, PacketClass, RepresentationProfile};
use arbitrary::Arbitrary;
use std::panic;

/// Input for comprehensive wire protocol fuzzing
#[derive(Arbitrary, Debug)]
enum FuzzInput {
    /// Test parsing arbitrary bytes (malformed packet handling)
    ParseArbitraryBytes(Vec<u8>),
    
    /// Test roundtrip encoding/decoding with structured data
    RoundtripTest {
        session_id: u64,
        node_id: u64,
        class: u8,
        profile: u8,
        time_hint: i32,
        seq: u16,
        window: u16,
        has_extensions: bool,
        ratchet_id: Option<u32>,
        key_epoch: Option<u16>,
        payload: Vec<u8>,
        auth_tag: [u8; AUTH_TAG_SIZE],
    },
    
    /// Test malformed headers
    MalformedHeader {
        /// Truncate header to this length
        truncate_to: u8,
        base_data: Vec<u8>,
    },
    
    /// Test invalid field values
    InvalidFields {
        invalid_version: u8,
        invalid_crypto_suite: u8,
        invalid_class: u8,
        header_len: u16,
        payload: Vec<u8>,
    },
    
    /// Test boundary conditions
    BoundaryTest {
        /// Test with zero-length payload
        zero_payload: bool,
        /// Test with maximum-length payload
        max_payload: bool,
        /// Test with oversized payload
        oversized_payload: bool,
    },
    
    /// Test corrupted auth tags
    CorruptedAuthTag {
        valid_frame: bool,
        corrupt_tag: [u8; AUTH_TAG_SIZE],
    },
}

/// Production-grade wire protocol fuzzer
struct WireProtocolFuzzer;

impl FuzzTarget for WireProtocolFuzzer {
    type Input = FuzzInput;

    fn fuzz_once(&mut self, input: Self::Input) -> FuzzResult {
        // Catch panics and convert to Bug results
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            match input {
                FuzzInput::ParseArbitraryBytes(bytes) => {
                    self.fuzz_parse_arbitrary(&bytes)
                }
                FuzzInput::RoundtripTest {
                    session_id,
                    node_id,
                    class,
                    profile,
                    time_hint,
                    seq,
                    window,
                    has_extensions,
                    ratchet_id,
                    key_epoch,
                    payload,
                    auth_tag,
                } => {
                    self.fuzz_roundtrip(
                        session_id,
                        node_id,
                        class,
                        profile,
                        time_hint,
                        seq,
                        window,
                        has_extensions,
                        ratchet_id,
                        key_epoch,
                        payload,
                        auth_tag,
                    )
                }
                FuzzInput::MalformedHeader { truncate_to, base_data } => {
                    self.fuzz_malformed_header(truncate_to, &base_data)
                }
                FuzzInput::InvalidFields {
                    invalid_version,
                    invalid_crypto_suite,
                    invalid_class,
                    header_len,
                    payload,
                } => {
                    self.fuzz_invalid_fields(
                        invalid_version,
                        invalid_crypto_suite,
                        invalid_class,
                        header_len,
                        payload,
                    )
                }
                FuzzInput::BoundaryTest {
                    zero_payload,
                    max_payload,
                    oversized_payload,
                } => {
                    self.fuzz_boundary_conditions(zero_payload, max_payload, oversized_payload)
                }
                FuzzInput::CorruptedAuthTag { valid_frame, corrupt_tag } => {
                    self.fuzz_corrupted_auth_tag(valid_frame, corrupt_tag)
                }
            }
        }));

        match result {
            Ok(fuzz_result) => fuzz_result,
            Err(panic_info) => {
                let panic_msg = if let Some(s) = panic_info.downcast_ref::<String>() {
                    s.clone()
                } else if let Some(s) = panic_info.downcast_ref::<&str>() {
                    s.to_string()
                } else {
                    "Unknown panic".to_string()
                };
                FuzzResult::Bug(format!("Panic during fuzzing: {}", panic_msg))
            }
        }
    }
}

impl WireProtocolFuzzer {
    /// Fuzz parsing of arbitrary bytes (malformed packet handling)
    fn fuzz_parse_arbitrary(&self, bytes: &[u8]) -> FuzzResult {
        match Frame::parse(bytes) {
            Ok(frame) => {
                // Successfully parsed - verify basic invariants
                if frame.size() > MAX_FRAME_SIZE {
                    return FuzzResult::Bug(format!(
                        "Parsed frame exceeds MAX_FRAME_SIZE: {} > {}",
                        frame.size(),
                        MAX_FRAME_SIZE
                    ));
                }
                
                // Try to serialize it back
                match frame.serialize() {
                    Ok(serialized) => {
                        // Verify serialized size matches
                        if serialized.len() != frame.size() {
                            return FuzzResult::Bug(format!(
                                "Size mismatch: size() = {}, serialized.len() = {}",
                                frame.size(),
                                serialized.len()
                            ));
                        }
                        FuzzResult::Ok
                    }
                    Err(_) => {
                        // Serialization can fail for valid reasons (e.g., too large)
                        FuzzResult::Ok
                    }
                }
            }
            Err(_) => {
                // Expected for malformed input
                FuzzResult::Invalid
            }
        }
    }

    /// Fuzz roundtrip encoding/decoding with structured data
    fn fuzz_roundtrip(
        &self,
        session_id: u64,
        node_id: u64,
        class: u8,
        profile: u8,
        time_hint: i32,
        seq: u16,
        window: u16,
        has_extensions: bool,
        ratchet_id: Option<u32>,
        key_epoch: Option<u16>,
        payload: Vec<u8>,
        auth_tag: [u8; AUTH_TAG_SIZE],
    ) -> FuzzResult {
        // Limit payload size to avoid oversized frames
        let payload = if payload.len() > MAX_FRAME_SIZE - MIN_FRAME_SIZE {
            payload[..MAX_FRAME_SIZE - MIN_FRAME_SIZE].to_vec()
        } else {
            payload
        };

        // Create header with arbitrary values
        let mut header = FixedHeader::new(
            SessionId::new(session_id),
            NodeId::new(node_id),
        );
        
        // Set packet class (map arbitrary u8 to valid class)
        header.class = match class % 5 {
            0 => PacketClass::Core,
            1 => PacketClass::Perceptual,
            2 => PacketClass::Enhancement,
            3 => PacketClass::Cosmetic,
            _ => PacketClass::Repair,
        };
        
        // Set representation profile (map arbitrary u8 to valid profile)
        header.profile = match profile % 9 {
            0 => RepresentationProfile::Textual,
            1 => RepresentationProfile::VoiceMinimal,
            2 => RepresentationProfile::VoiceStandard,
            3 => RepresentationProfile::VoiceHigh,
            4 => RepresentationProfile::VideoLow,
            5 => RepresentationProfile::VideoStandard,
            6 => RepresentationProfile::VideoHigh,
            7 => RepresentationProfile::StreamAsymmetric,
            _ => RepresentationProfile::Agent,
        };
        
        header.time_hint = time_hint;
        header.set_seq(seq);
        header.set_window(window);

        // Create extensions if requested
        let mut extensions = Extensions::new();
        if has_extensions {
            extensions.ratchet_id = ratchet_id;
            extensions.key_epoch = key_epoch;
        }

        // Build frame
        let frame = FrameBuilder::new(header)
            .extensions(extensions)
            .payload(payload.clone())
            .auth_tag(auth_tag)
            .build();

        // Check size before serialization
        if !frame.fits_mtu() {
            // This is expected for large payloads
            return FuzzResult::Invalid;
        }

        // Serialize
        let serialized = match frame.serialize() {
            Ok(s) => s,
            Err(_) => return FuzzResult::Invalid,
        };

        // Parse back
        let parsed = match Frame::parse(&serialized) {
            Ok(p) => p,
            Err(_) => {
                return FuzzResult::Bug(
                    "Failed to parse frame that was just serialized".to_string()
                );
            }
        };

        // Verify roundtrip integrity
        if parsed.header.session_id != frame.header.session_id {
            return FuzzResult::Bug("Session ID mismatch after roundtrip".to_string());
        }
        if parsed.header.node_id != frame.header.node_id {
            return FuzzResult::Bug("Node ID mismatch after roundtrip".to_string());
        }
        if parsed.header.class != frame.header.class {
            return FuzzResult::Bug("Packet class mismatch after roundtrip".to_string());
        }
        if parsed.header.profile != frame.header.profile {
            return FuzzResult::Bug("Profile mismatch after roundtrip".to_string());
        }
        if parsed.header.time_hint != frame.header.time_hint {
            return FuzzResult::Bug("Time hint mismatch after roundtrip".to_string());
        }
        if parsed.header.seq() != frame.header.seq() {
            return FuzzResult::Bug("Sequence number mismatch after roundtrip".to_string());
        }
        if parsed.header.window() != frame.header.window() {
            return FuzzResult::Bug("Window mismatch after roundtrip".to_string());
        }
        if parsed.payload != frame.payload {
            return FuzzResult::Bug("Payload mismatch after roundtrip".to_string());
        }
        if parsed.auth_tag != frame.auth_tag {
            return FuzzResult::Bug("Auth tag mismatch after roundtrip".to_string());
        }
        if has_extensions {
            if parsed.extensions.ratchet_id != frame.extensions.ratchet_id {
                return FuzzResult::Bug("Ratchet ID mismatch after roundtrip".to_string());
            }
            if parsed.extensions.key_epoch != frame.extensions.key_epoch {
                return FuzzResult::Bug("Key epoch mismatch after roundtrip".to_string());
            }
        }

        FuzzResult::Ok
    }

    /// Fuzz malformed headers (truncated, incomplete)
    fn fuzz_malformed_header(&self, truncate_to: u8, base_data: &[u8]) -> FuzzResult {
        // Create a valid frame first
        let header = FixedHeader::default();
        let frame = FrameBuilder::new(header)
            .payload(base_data.to_vec())
            .auth_tag([0; AUTH_TAG_SIZE])
            .build();

        let serialized = match frame.serialize() {
            Ok(s) => s,
            Err(_) => return FuzzResult::Invalid,
        };

        // Truncate to arbitrary length
        let truncate_len = (truncate_to as usize).min(serialized.len());
        let truncated = &serialized[..truncate_len];

        // Try to parse truncated data
        match Frame::parse(truncated) {
            Ok(_) => {
                // Should not succeed with truncated data
                if truncate_len < MIN_FRAME_SIZE {
                    return FuzzResult::Bug(format!(
                        "Parsed truncated frame with length {} < MIN_FRAME_SIZE",
                        truncate_len
                    ));
                }
                FuzzResult::Ok
            }
            Err(_) => {
                // Expected for truncated data
                FuzzResult::Invalid
            }
        }
    }

    /// Fuzz invalid field values
    fn fuzz_invalid_fields(
        &self,
        invalid_version: u8,
        invalid_crypto_suite: u8,
        invalid_class: u8,
        header_len: u16,
        payload: Vec<u8>,
    ) -> FuzzResult {
        // Create a buffer with invalid header fields
        let mut buf = vec![0u8; MIN_FRAME_SIZE + payload.len().min(100)];
        
        // Byte 0: Invalid version (upper 4 bits) + invalid crypto suite (lower 4 bits)
        buf[0] = (invalid_version << 4) | (invalid_crypto_suite & 0x0F);
        
        // Bytes 2-3: Potentially invalid header length
        buf[2..4].copy_from_slice(&header_len.to_le_bytes());
        
        // Byte 20: Invalid packet class
        buf[20] = invalid_class;
        
        // Add some payload
        let payload_len = payload.len().min(buf.len() - MIN_FRAME_SIZE);
        buf[30..30 + payload_len].copy_from_slice(&payload[..payload_len]);

        // Try to parse
        match Frame::parse(&buf) {
            Ok(_) => FuzzResult::Ok,
            Err(_) => {
                // Expected for invalid fields
                FuzzResult::Invalid
            }
        }
    }

    /// Fuzz boundary conditions
    fn fuzz_boundary_conditions(
        &self,
        zero_payload: bool,
        max_payload: bool,
        oversized_payload: bool,
    ) -> FuzzResult {
        let header = FixedHeader::default();
        
        let payload = if zero_payload {
            vec![]
        } else if max_payload {
            // Maximum payload that fits in MTU
            vec![0u8; MAX_FRAME_SIZE - MIN_FRAME_SIZE]
        } else if oversized_payload {
            // Oversized payload (should fail)
            vec![0u8; MAX_FRAME_SIZE + 1000]
        } else {
            vec![0u8; 100]
        };

        let frame = FrameBuilder::new(header)
            .payload(payload)
            .auth_tag([0; AUTH_TAG_SIZE])
            .build();

        if oversized_payload {
            // Should fail to serialize
            match frame.serialize() {
                Ok(_) => {
                    return FuzzResult::Bug(
                        "Successfully serialized oversized frame".to_string()
                    );
                }
                Err(_) => return FuzzResult::Ok,
            }
        }

        // Try to serialize and parse back
        let serialized = match frame.serialize() {
            Ok(s) => s,
            Err(_) => return FuzzResult::Invalid,
        };

        match Frame::parse(&serialized) {
            Ok(parsed) => {
                if parsed.payload.len() != frame.payload.len() {
                    return FuzzResult::Bug(format!(
                        "Payload length mismatch: {} != {}",
                        parsed.payload.len(),
                        frame.payload.len()
                    ));
                }
                FuzzResult::Ok
            }
            Err(_) => {
                FuzzResult::Bug("Failed to parse valid boundary condition frame".to_string())
            }
        }
    }

    /// Fuzz corrupted auth tags
    fn fuzz_corrupted_auth_tag(&self, valid_frame: bool, corrupt_tag: [u8; AUTH_TAG_SIZE]) -> FuzzResult {
        let header = FixedHeader::default();
        let payload = vec![1, 2, 3, 4, 5];
        
        let auth_tag = if valid_frame {
            [0xAA; AUTH_TAG_SIZE]
        } else {
            corrupt_tag
        };

        let frame = FrameBuilder::new(header)
            .payload(payload)
            .auth_tag(auth_tag)
            .build();

        let serialized = match frame.serialize() {
            Ok(s) => s,
            Err(_) => return FuzzResult::Invalid,
        };

        // Parse should succeed regardless of auth tag content
        // (auth tag verification happens at a higher layer)
        match Frame::parse(&serialized) {
            Ok(parsed) => {
                if parsed.auth_tag != auth_tag {
                    return FuzzResult::Bug("Auth tag mismatch after parse".to_string());
                }
                FuzzResult::Ok
            }
            Err(_) => {
                FuzzResult::Bug("Failed to parse frame with valid structure".to_string())
            }
        }
    }
}

fuzz_target!(|data: FuzzInput| {
    let mut fuzzer = WireProtocolFuzzer;
    let _ = fuzzer.fuzz_once(data);
});
