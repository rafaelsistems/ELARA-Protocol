//! Test the wire protocol fuzzer to ensure it works correctly

use elara_fuzz::{FuzzTarget, FuzzResult};
use elara_wire::{Frame, FrameBuilder, FixedHeader, Extensions, AUTH_TAG_SIZE, MAX_FRAME_SIZE, MIN_FRAME_SIZE};
use elara_core::{NodeId, SessionId, PacketClass, RepresentationProfile};

/// Simplified version of the wire protocol fuzzer for testing
struct WireProtocolFuzzer;

impl FuzzTarget for WireProtocolFuzzer {
    type Input = Vec<u8>;

    fn fuzz_once(&mut self, input: Self::Input) -> FuzzResult {
        match Frame::parse(&input) {
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
}

#[test]
fn test_fuzzer_with_valid_frame() {
    let mut fuzzer = WireProtocolFuzzer;
    
    // Create a valid frame
    let header = FixedHeader::new(SessionId::new(12345), NodeId::new(67890));
    let frame = FrameBuilder::new(header)
        .payload(vec![1, 2, 3, 4, 5])
        .auth_tag([0xAA; AUTH_TAG_SIZE])
        .build();
    
    let bytes = frame.serialize().unwrap();
    
    // Fuzz with valid frame
    let result = fuzzer.fuzz_once(bytes);
    assert_eq!(result, FuzzResult::Ok);
}

#[test]
fn test_fuzzer_with_truncated_frame() {
    let mut fuzzer = WireProtocolFuzzer;
    
    // Create a valid frame
    let header = FixedHeader::new(SessionId::new(12345), NodeId::new(67890));
    let frame = FrameBuilder::new(header)
        .payload(vec![1, 2, 3, 4, 5])
        .auth_tag([0xAA; AUTH_TAG_SIZE])
        .build();
    
    let bytes = frame.serialize().unwrap();
    
    // Truncate to less than minimum size
    let truncated = &bytes[..MIN_FRAME_SIZE - 5];
    
    // Fuzz with truncated frame
    let result = fuzzer.fuzz_once(truncated.to_vec());
    assert_eq!(result, FuzzResult::Invalid);
}

#[test]
fn test_fuzzer_with_arbitrary_bytes() {
    let mut fuzzer = WireProtocolFuzzer;
    
    // Test with completely arbitrary bytes
    let arbitrary = vec![0xFF; 100];
    let result = fuzzer.fuzz_once(arbitrary);
    
    // Should either be Ok (if it happens to parse) or Invalid (expected)
    assert!(matches!(result, FuzzResult::Ok | FuzzResult::Invalid));
}

#[test]
fn test_fuzzer_roundtrip() {
    let mut fuzzer = WireProtocolFuzzer;
    
    // Create frames with various configurations
    let test_cases = vec![
        // Zero-length payload
        (vec![], [0x00; AUTH_TAG_SIZE]),
        // Small payload
        (vec![1, 2, 3], [0xAA; AUTH_TAG_SIZE]),
        // Medium payload
        (vec![0x42; 100], [0xBB; AUTH_TAG_SIZE]),
        // Large payload (but within MTU)
        (vec![0xFF; 500], [0xCC; AUTH_TAG_SIZE]),
    ];
    
    for (payload, auth_tag) in test_cases {
        let header = FixedHeader::new(SessionId::new(1), NodeId::new(2));
        let frame = FrameBuilder::new(header)
            .payload(payload.clone())
            .auth_tag(auth_tag)
            .build();
        
        if frame.fits_mtu() {
            let bytes = frame.serialize().unwrap();
            let result = fuzzer.fuzz_once(bytes);
            assert_eq!(result, FuzzResult::Ok, "Failed for payload len: {}", payload.len());
        }
    }
}

#[test]
fn test_fuzzer_with_extensions() {
    let mut fuzzer = WireProtocolFuzzer;
    
    let header = FixedHeader::new(SessionId::new(100), NodeId::new(200));
    let mut ext = Extensions::new();
    ext.ratchet_id = Some(42);
    ext.key_epoch = Some(5);
    
    let frame = FrameBuilder::new(header)
        .extensions(ext)
        .payload(vec![1, 2, 3, 4, 5])
        .auth_tag([0xDD; AUTH_TAG_SIZE])
        .build();
    
    let bytes = frame.serialize().unwrap();
    let result = fuzzer.fuzz_once(bytes);
    assert_eq!(result, FuzzResult::Ok);
}

#[test]
fn test_fuzzer_with_max_payload() {
    let mut fuzzer = WireProtocolFuzzer;
    
    let header = FixedHeader::new(SessionId::new(1), NodeId::new(2));
    
    // Maximum payload that fits in MTU
    let max_payload_size = MAX_FRAME_SIZE - MIN_FRAME_SIZE;
    let payload = vec![0x42; max_payload_size];
    
    let frame = FrameBuilder::new(header)
        .payload(payload)
        .auth_tag([0xEE; AUTH_TAG_SIZE])
        .build();
    
    assert!(frame.fits_mtu());
    
    let bytes = frame.serialize().unwrap();
    let result = fuzzer.fuzz_once(bytes);
    assert_eq!(result, FuzzResult::Ok);
}

#[test]
fn test_fuzzer_with_oversized_payload() {
    let header = FixedHeader::new(SessionId::new(1), NodeId::new(2));
    
    // Oversized payload (exceeds MTU)
    let oversized_payload = vec![0x42; MAX_FRAME_SIZE + 100];
    
    let frame = FrameBuilder::new(header)
        .payload(oversized_payload)
        .auth_tag([0xFF; AUTH_TAG_SIZE])
        .build();
    
    assert!(!frame.fits_mtu());
    
    // Should fail to serialize
    assert!(frame.serialize().is_err());
}

#[test]
fn test_fuzzer_with_various_packet_classes() {
    let mut fuzzer = WireProtocolFuzzer;
    
    let classes = vec![
        PacketClass::Core,
        PacketClass::Perceptual,
        PacketClass::Enhancement,
        PacketClass::Cosmetic,
        PacketClass::Repair,
    ];
    
    for class in classes {
        let mut header = FixedHeader::new(SessionId::new(1), NodeId::new(2));
        header.class = class;
        
        let frame = FrameBuilder::new(header)
            .payload(vec![1, 2, 3])
            .auth_tag([0xAA; AUTH_TAG_SIZE])
            .build();
        
        let bytes = frame.serialize().unwrap();
        let result = fuzzer.fuzz_once(bytes);
        assert_eq!(result, FuzzResult::Ok, "Failed for class: {:?}", class);
    }
}

#[test]
fn test_fuzzer_with_various_profiles() {
    let mut fuzzer = WireProtocolFuzzer;
    
    let profiles = vec![
        RepresentationProfile::Textual,
        RepresentationProfile::VoiceMinimal,
        RepresentationProfile::VoiceStandard,
        RepresentationProfile::VoiceHigh,
        RepresentationProfile::VideoLow,
        RepresentationProfile::VideoStandard,
        RepresentationProfile::VideoHigh,
        RepresentationProfile::StreamAsymmetric,
        RepresentationProfile::Agent,
    ];
    
    for profile in profiles {
        let mut header = FixedHeader::new(SessionId::new(1), NodeId::new(2));
        header.profile = profile;
        
        let frame = FrameBuilder::new(header)
            .payload(vec![1, 2, 3])
            .auth_tag([0xBB; AUTH_TAG_SIZE])
            .build();
        
        let bytes = frame.serialize().unwrap();
        let result = fuzzer.fuzz_once(bytes);
        assert_eq!(result, FuzzResult::Ok, "Failed for profile: {:?}", profile);
    }
}
