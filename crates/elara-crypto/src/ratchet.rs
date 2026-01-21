//! Multi-ratchet key management per packet class

use hkdf::Hkdf;
use sha2::Sha256;

use elara_core::PacketClass;

use crate::KEY_SIZE;

/// Ratchet state for a single packet class
#[derive(Clone)]
pub struct ClassRatchet {
    /// Current chain key
    chain_key: [u8; KEY_SIZE],
    /// Current epoch (increments on ratchet advance)
    epoch: u32,
    /// Message counter within epoch
    message_count: u32,
    /// Packet class this ratchet is for
    class: PacketClass,
}

impl ClassRatchet {
    /// Create a new ratchet from root key
    pub fn new(root_key: &[u8; KEY_SIZE], class: PacketClass) -> Self {
        let chain_key = Self::derive_class_key(root_key, class);
        ClassRatchet {
            chain_key,
            epoch: 0,
            message_count: 0,
            class,
        }
    }

    /// Derive class-specific key from session root
    fn derive_class_key(root_key: &[u8; KEY_SIZE], class: PacketClass) -> [u8; KEY_SIZE] {
        let hkdf = Hkdf::<Sha256>::new(Some(&[class.to_byte()]), root_key);
        let mut key = [0u8; KEY_SIZE];
        hkdf.expand(b"ELARA_CLASS_KEY_v0", &mut key)
            .expect("HKDF expand failed");
        key
    }

    /// Get current message key (for encryption/decryption)
    pub fn message_key(&self) -> [u8; KEY_SIZE] {
        let hkdf = Hkdf::<Sha256>::new(None, &self.chain_key);
        let mut key = [0u8; KEY_SIZE];
        let info = format!("ELARA_MSG_KEY_{}", self.message_count);
        hkdf.expand(info.as_bytes(), &mut key)
            .expect("HKDF expand failed");
        key
    }

    /// Advance to next message (within same epoch)
    pub fn advance_message(&mut self) {
        self.message_count += 1;

        // Check if we need to advance epoch based on class frequency
        if self.message_count >= self.class.ratchet_frequency() {
            self.advance_epoch();
        }
    }

    /// Advance to next epoch (ratchet forward)
    pub fn advance_epoch(&mut self) {
        let hkdf = Hkdf::<Sha256>::new(Some(&self.epoch.to_le_bytes()), &self.chain_key);
        hkdf.expand(b"ELARA_RATCHET_CHAIN", &mut self.chain_key)
            .expect("HKDF expand failed");

        self.epoch += 1;
        self.message_count = 0;
    }

    /// Get current epoch
    pub fn epoch(&self) -> u32 {
        self.epoch
    }

    /// Get message count within epoch
    pub fn message_count(&self) -> u32 {
        self.message_count
    }

    /// Get packet class
    pub fn class(&self) -> PacketClass {
        self.class
    }

    /// Sync to a specific epoch (for recovery)
    pub fn sync_to_epoch(&mut self, target_epoch: u32) {
        while self.epoch < target_epoch {
            self.advance_epoch();
        }
    }
}

impl std::fmt::Debug for ClassRatchet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClassRatchet")
            .field("class", &self.class)
            .field("epoch", &self.epoch)
            .field("message_count", &self.message_count)
            .finish_non_exhaustive()
    }
}

/// Multi-ratchet manager for all packet classes
#[derive(Debug)]
pub struct MultiRatchet {
    core: ClassRatchet,
    perceptual: ClassRatchet,
    enhancement: ClassRatchet,
    cosmetic: ClassRatchet,
    repair: ClassRatchet,
}

impl MultiRatchet {
    /// Create from session root key
    pub fn new(session_root: &[u8; KEY_SIZE]) -> Self {
        MultiRatchet {
            core: ClassRatchet::new(session_root, PacketClass::Core),
            perceptual: ClassRatchet::new(session_root, PacketClass::Perceptual),
            enhancement: ClassRatchet::new(session_root, PacketClass::Enhancement),
            cosmetic: ClassRatchet::new(session_root, PacketClass::Cosmetic),
            repair: ClassRatchet::new(session_root, PacketClass::Repair),
        }
    }

    /// Get ratchet for a specific class
    pub fn get(&self, class: PacketClass) -> &ClassRatchet {
        match class {
            PacketClass::Core => &self.core,
            PacketClass::Perceptual => &self.perceptual,
            PacketClass::Enhancement => &self.enhancement,
            PacketClass::Cosmetic => &self.cosmetic,
            PacketClass::Repair => &self.repair,
        }
    }

    /// Get mutable ratchet for a specific class
    pub fn get_mut(&mut self, class: PacketClass) -> &mut ClassRatchet {
        match class {
            PacketClass::Core => &mut self.core,
            PacketClass::Perceptual => &mut self.perceptual,
            PacketClass::Enhancement => &mut self.enhancement,
            PacketClass::Cosmetic => &mut self.cosmetic,
            PacketClass::Repair => &mut self.repair,
        }
    }

    /// Get message key for a class and advance
    pub fn next_message_key(&mut self, class: PacketClass) -> [u8; KEY_SIZE] {
        let ratchet = self.get_mut(class);
        let key = ratchet.message_key();
        ratchet.advance_message();
        key
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ratchet_advance() {
        let root = [0x42u8; KEY_SIZE];
        let mut ratchet = ClassRatchet::new(&root, PacketClass::Core);

        let key1 = ratchet.message_key();
        ratchet.advance_message();
        let key2 = ratchet.message_key();

        // Different messages should have different keys
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_epoch_advance() {
        let root = [0x42u8; KEY_SIZE];
        let mut ratchet = ClassRatchet::new(&root, PacketClass::Core);

        let epoch0_key = ratchet.message_key();
        ratchet.advance_epoch();
        let epoch1_key = ratchet.message_key();

        // Different epochs should have different keys
        assert_ne!(epoch0_key, epoch1_key);
        assert_eq!(ratchet.epoch(), 1);
        assert_eq!(ratchet.message_count(), 0);
    }

    #[test]
    fn test_class_isolation() {
        let root = [0x42u8; KEY_SIZE];
        let multi = MultiRatchet::new(&root);

        let core_key = multi.get(PacketClass::Core).message_key();
        let perceptual_key = multi.get(PacketClass::Perceptual).message_key();

        // Different classes should have different keys
        assert_ne!(core_key, perceptual_key);
    }

    #[test]
    fn test_sync_to_epoch() {
        let root = [0x42u8; KEY_SIZE];
        let mut ratchet1 = ClassRatchet::new(&root, PacketClass::Core);
        let mut ratchet2 = ClassRatchet::new(&root, PacketClass::Core);

        // Advance ratchet1 to epoch 5
        for _ in 0..5 {
            ratchet1.advance_epoch();
        }

        // Sync ratchet2 to epoch 5
        ratchet2.sync_to_epoch(5);

        // Should have same key
        assert_eq!(ratchet1.message_key(), ratchet2.message_key());
    }

    #[test]
    fn test_auto_epoch_advance() {
        let root = [0x42u8; KEY_SIZE];
        let mut ratchet = ClassRatchet::new(&root, PacketClass::Core);

        // Core ratchet frequency is 100
        for _ in 0..100 {
            ratchet.advance_message();
        }

        // Should have auto-advanced to epoch 1
        assert_eq!(ratchet.epoch(), 1);
    }
}
