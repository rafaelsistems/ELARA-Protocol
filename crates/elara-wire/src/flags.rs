//! Frame flags for ELARA wire protocol

/// Frame flags (1 byte)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameFlags(pub u8);

impl FrameFlags {
    pub const NONE: FrameFlags = FrameFlags(0);

    // Flag bits
    pub const MULTIPATH: u8 = 0b0000_0001;
    pub const RELAY: u8 = 0b0000_0010;
    pub const FRAGMENT: u8 = 0b0000_0100;
    pub const REPAIR: u8 = 0b0000_1000;
    pub const PRIORITY: u8 = 0b0001_0000;
    pub const EXTENSION: u8 = 0b0010_0000;
    pub const COMPRESSED: u8 = 0b0100_0000;
    pub const RESERVED: u8 = 0b1000_0000;

    #[inline]
    pub fn new(bits: u8) -> Self {
        FrameFlags(bits)
    }

    #[inline]
    pub fn is_multipath(self) -> bool {
        self.0 & Self::MULTIPATH != 0
    }

    #[inline]
    pub fn is_relay(self) -> bool {
        self.0 & Self::RELAY != 0
    }

    #[inline]
    pub fn is_fragment(self) -> bool {
        self.0 & Self::FRAGMENT != 0
    }

    #[inline]
    pub fn is_repair(self) -> bool {
        self.0 & Self::REPAIR != 0
    }

    #[inline]
    pub fn is_priority(self) -> bool {
        self.0 & Self::PRIORITY != 0
    }

    #[inline]
    pub fn has_extension(self) -> bool {
        self.0 & Self::EXTENSION != 0
    }

    #[inline]
    pub fn is_compressed(self) -> bool {
        self.0 & Self::COMPRESSED != 0
    }

    #[inline]
    pub fn set_multipath(&mut self, value: bool) {
        if value {
            self.0 |= Self::MULTIPATH;
        } else {
            self.0 &= !Self::MULTIPATH;
        }
    }

    #[inline]
    pub fn set_relay(&mut self, value: bool) {
        if value {
            self.0 |= Self::RELAY;
        } else {
            self.0 &= !Self::RELAY;
        }
    }

    #[inline]
    pub fn set_fragment(&mut self, value: bool) {
        if value {
            self.0 |= Self::FRAGMENT;
        } else {
            self.0 &= !Self::FRAGMENT;
        }
    }

    #[inline]
    pub fn set_repair(&mut self, value: bool) {
        if value {
            self.0 |= Self::REPAIR;
        } else {
            self.0 &= !Self::REPAIR;
        }
    }

    #[inline]
    pub fn set_priority(&mut self, value: bool) {
        if value {
            self.0 |= Self::PRIORITY;
        } else {
            self.0 &= !Self::PRIORITY;
        }
    }

    #[inline]
    pub fn set_extension(&mut self, value: bool) {
        if value {
            self.0 |= Self::EXTENSION;
        } else {
            self.0 &= !Self::EXTENSION;
        }
    }

    #[inline]
    pub fn set_compressed(&mut self, value: bool) {
        if value {
            self.0 |= Self::COMPRESSED;
        } else {
            self.0 &= !Self::COMPRESSED;
        }
    }
}

impl From<u8> for FrameFlags {
    fn from(bits: u8) -> Self {
        FrameFlags(bits)
    }
}

impl From<FrameFlags> for u8 {
    fn from(flags: FrameFlags) -> Self {
        flags.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flag_operations() {
        let mut flags = FrameFlags::NONE;

        assert!(!flags.is_multipath());
        flags.set_multipath(true);
        assert!(flags.is_multipath());

        flags.set_priority(true);
        assert!(flags.is_priority());
        assert!(flags.is_multipath());

        flags.set_multipath(false);
        assert!(!flags.is_multipath());
        assert!(flags.is_priority());
    }

    #[test]
    fn test_flag_bits() {
        let flags = FrameFlags(FrameFlags::MULTIPATH | FrameFlags::RELAY | FrameFlags::EXTENSION);

        assert!(flags.is_multipath());
        assert!(flags.is_relay());
        assert!(flags.has_extension());
        assert!(!flags.is_fragment());
        assert!(!flags.is_compressed());
    }
}
