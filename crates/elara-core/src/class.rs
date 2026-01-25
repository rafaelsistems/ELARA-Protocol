//! Packet and event class definitions
//!
//! ELARA uses a class-based system for packet prioritization and handling:
//! - Core: Identity, presence, session - never dropped
//! - Perceptual: Voice, typing - loss tolerant, predictive
//! - Enhancement: HD layers - opportunistic
//! - Cosmetic: Reactions - discardable
//! - Repair: State sync - delayed OK

/// Packet class determines network and crypto behavior
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum PacketClass {
    /// Core state: identity, presence, session, crypto
    /// Network: redundant, never dropped
    /// Crypto: strongest ratchet
    #[default]
    Core = 0x00,

    /// Perceptual state: voice, typing, live updates
    /// Network: predictive, loss tolerant
    /// Crypto: fast ratchet
    Perceptual = 0x01,

    /// Enhancement state: HD layers, quality upgrades
    /// Network: opportunistic, drop first
    /// Crypto: standard ratchet
    Enhancement = 0x02,

    /// Cosmetic state: reactions, filters, UI hints
    /// Network: discardable
    /// Crypto: light ratchet
    Cosmetic = 0x03,

    /// Repair state: state summaries, resync
    /// Network: bursty, delayed OK
    /// Crypto: strong ratchet
    Repair = 0x04,
}

impl PacketClass {
    /// Parse from wire byte
    pub fn from_byte(b: u8) -> Option<Self> {
        match b {
            0x00 => Some(PacketClass::Core),
            0x01 => Some(PacketClass::Perceptual),
            0x02 => Some(PacketClass::Enhancement),
            0x03 => Some(PacketClass::Cosmetic),
            0x04 => Some(PacketClass::Repair),
            _ => None,
        }
    }

    /// Convert to wire byte
    #[inline]
    pub fn to_byte(self) -> u8 {
        self as u8
    }

    /// Get redundancy level (how many times to send)
    pub fn redundancy(self) -> u8 {
        match self {
            PacketClass::Core => 3,
            PacketClass::Perceptual => 1,
            PacketClass::Enhancement => 1,
            PacketClass::Cosmetic => 1,
            PacketClass::Repair => 2,
        }
    }

    /// Can this class be dropped under congestion?
    pub fn is_droppable(self) -> bool {
        match self {
            PacketClass::Core => false,
            PacketClass::Perceptual => false, // Predict instead
            PacketClass::Enhancement => true,
            PacketClass::Cosmetic => true,
            PacketClass::Repair => false,
        }
    }

    /// Priority for scheduling (lower = higher priority)
    pub fn priority(self) -> u8 {
        match self {
            PacketClass::Core => 0,
            PacketClass::Perceptual => 1,
            PacketClass::Repair => 2,
            PacketClass::Enhancement => 3,
            PacketClass::Cosmetic => 4,
        }
    }

    /// Ratchet frequency (messages between ratchet advances)
    pub fn ratchet_frequency(self) -> u32 {
        match self {
            PacketClass::Core => 100,
            PacketClass::Perceptual => 1000,
            PacketClass::Enhancement => 500,
            PacketClass::Cosmetic => 2000,
            PacketClass::Repair => 50,
        }
    }

    /// Replay window size
    pub fn replay_window_size(self) -> u16 {
        match self {
            PacketClass::Core => 64,
            PacketClass::Perceptual => 256, // Larger for reorder tolerance
            PacketClass::Enhancement => 128,
            PacketClass::Cosmetic => 32,
            PacketClass::Repair => 32,
        }
    }
}

/// Representation profile hint
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum RepresentationProfile {
    /// Text-based communication
    #[default]
    Textual = 0x00,

    /// Minimal voice (MSP baseline)
    VoiceMinimal = 0x10,
    /// Standard voice quality
    VoiceStandard = 0x11,
    /// High quality voice
    VoiceHigh = 0x12,

    /// Low quality video
    VideoLow = 0x20,
    /// Standard video
    VideoStandard = 0x21,
    /// High quality video
    VideoHigh = 0x22,

    /// Asymmetric streaming (livestream)
    StreamAsymmetric = 0x30,

    /// AI agent presence
    Agent = 0x40,

    /// Unknown/custom profile
    Custom = 0xFF,
}

impl RepresentationProfile {
    pub fn from_byte(b: u8) -> Self {
        match b {
            0x00 => RepresentationProfile::Textual,
            0x10 => RepresentationProfile::VoiceMinimal,
            0x11 => RepresentationProfile::VoiceStandard,
            0x12 => RepresentationProfile::VoiceHigh,
            0x20 => RepresentationProfile::VideoLow,
            0x21 => RepresentationProfile::VideoStandard,
            0x22 => RepresentationProfile::VideoHigh,
            0x30 => RepresentationProfile::StreamAsymmetric,
            0x40 => RepresentationProfile::Agent,
            _ => RepresentationProfile::Custom,
        }
    }

    #[inline]
    pub fn to_byte(self) -> u8 {
        self as u8
    }

    /// Is this profile supported in MSP (Minimal Survival Profile)?
    pub fn is_msp_supported(self) -> bool {
        matches!(
            self,
            RepresentationProfile::Textual | RepresentationProfile::VoiceMinimal
        )
    }
}

/// State type classification
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StateType {
    /// Core state: never lost, strongest guarantees
    Core,
    /// Perceptual state: loss tolerant, predictable
    Perceptual,
    /// Enhancement state: opportunistic delivery
    Enhancement,
    /// Cosmetic state: freely discardable
    Cosmetic,
}

impl StateType {
    /// Convert to packet class for transmission
    pub fn to_packet_class(self) -> PacketClass {
        match self {
            StateType::Core => PacketClass::Core,
            StateType::Perceptual => PacketClass::Perceptual,
            StateType::Enhancement => PacketClass::Enhancement,
            StateType::Cosmetic => PacketClass::Cosmetic,
        }
    }

    /// Importance weight for divergence control
    pub fn importance(self) -> f64 {
        match self {
            StateType::Core => 1.0,
            StateType::Perceptual => 0.7,
            StateType::Enhancement => 0.3,
            StateType::Cosmetic => 0.1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_class_roundtrip() {
        for class in [
            PacketClass::Core,
            PacketClass::Perceptual,
            PacketClass::Enhancement,
            PacketClass::Cosmetic,
            PacketClass::Repair,
        ] {
            let byte = class.to_byte();
            let recovered = PacketClass::from_byte(byte).unwrap();
            assert_eq!(class, recovered);
        }
    }

    #[test]
    fn test_profile_msp_support() {
        assert!(RepresentationProfile::Textual.is_msp_supported());
        assert!(RepresentationProfile::VoiceMinimal.is_msp_supported());
        assert!(!RepresentationProfile::VideoStandard.is_msp_supported());
    }

    #[test]
    fn test_class_priority_ordering() {
        assert!(PacketClass::Core.priority() < PacketClass::Perceptual.priority());
        assert!(PacketClass::Perceptual.priority() < PacketClass::Enhancement.priority());
        assert!(PacketClass::Enhancement.priority() < PacketClass::Cosmetic.priority());
    }
}
