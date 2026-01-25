//! Secure frame encryption/decryption - Wire + Crypto binding
//!
//! This module binds the wire protocol with cryptographic operations:
//! - Frame encryption with AEAD
//! - Frame decryption with validation
//! - Multi-ratchet key selection per packet class

use elara_core::{ElaraError, ElaraResult, NodeId, PacketClass, RepresentationProfile, SessionId};
use elara_wire::{Extensions, FixedHeader, Frame, FrameBuilder, FIXED_HEADER_SIZE};

use crate::{AeadCipher, MultiRatchet, ReplayManager, KEY_SIZE};

/// Secure frame processor - handles encryption/decryption of frames
pub struct SecureFrameProcessor {
    /// Session ID
    session_id: SessionId,
    /// Local node ID
    local_node_id: NodeId,
    /// Multi-ratchet for key derivation
    ratchet: MultiRatchet,
    /// Replay protection
    replay_manager: ReplayManager,
    /// Sequence counters per class
    seq_counters: [u16; 5],
}

impl SecureFrameProcessor {
    /// Create a new secure frame processor
    pub fn new(session_id: SessionId, local_node_id: NodeId, session_key: [u8; KEY_SIZE]) -> Self {
        SecureFrameProcessor {
            session_id,
            local_node_id,
            ratchet: MultiRatchet::new(&session_key),
            replay_manager: ReplayManager::new(),
            seq_counters: [0; 5],
        }
    }

    /// Get next sequence number for a class
    fn next_seq(&mut self, class: PacketClass) -> u16 {
        let idx = class.to_byte() as usize;
        let seq = self.seq_counters[idx];
        self.seq_counters[idx] = seq.wrapping_add(1);
        seq
    }

    /// Encrypt a frame for transmission
    pub fn encrypt_frame(
        &mut self,
        class: PacketClass,
        profile: RepresentationProfile,
        time_hint: i32,
        extensions: Extensions,
        payload: &[u8],
    ) -> ElaraResult<Vec<u8>> {
        // Get message key from ratchet
        let key = self.ratchet.next_message_key(class);
        let cipher = AeadCipher::new(&key);

        // Build header
        let seq = self.next_seq(class);
        let mut header = FixedHeader::new(self.session_id, self.local_node_id);
        header.class = class;
        header.profile = profile;
        header.time_hint = time_hint;
        header.set_seq(seq);
        header.set_window(class.replay_window_size());

        // Set extension flag if needed
        if !extensions.is_empty() {
            header.flags.set_extension(true);
        }

        // Serialize header for AAD
        let mut header_bytes = [0u8; FIXED_HEADER_SIZE];
        let _ = header.serialize(&mut header_bytes);

        // Derive nonce from header parameters
        let nonce = crate::derive_nonce(self.local_node_id, seq, class);

        // Encrypt payload (returns ciphertext with tag appended)
        let ciphertext = cipher.encrypt(&nonce, &header_bytes, payload)?;

        // Build complete frame
        let frame = FrameBuilder::new(header)
            .extensions(extensions)
            .payload(ciphertext)
            .build();

        frame.serialize()
    }

    /// Decrypt a received frame
    pub fn decrypt_frame(&mut self, data: &[u8]) -> ElaraResult<DecryptedFrame> {
        // Parse frame structure
        let frame = Frame::parse(data)?;

        // Check session ID
        if frame.header.session_id != self.session_id {
            return Err(ElaraError::SessionMismatch);
        }

        // Check replay
        let seq = frame.header.seq();
        let class = frame.header.class;
        let node_id = frame.header.node_id;

        self.replay_manager.accept(node_id, class, seq)?;

        // Get decryption key (need to sync ratchet if needed)
        let ratchet = self.ratchet.get(class);
        let key = ratchet.message_key();
        let cipher = AeadCipher::new(&key);

        // Derive nonce
        let nonce = crate::derive_nonce(node_id, seq, class);

        // Get AAD (header bytes)
        let aad = &data[..FIXED_HEADER_SIZE];

        // Decrypt payload (ciphertext includes auth tag)
        let plaintext = cipher.decrypt(&nonce, aad, &frame.payload)?;

        // Advance ratchet after successful decryption
        self.ratchet.get_mut(class).advance_message();

        Ok(DecryptedFrame {
            header: frame.header,
            extensions: frame.extensions,
            payload: plaintext,
            source: node_id,
        })
    }

    /// Get session ID
    pub fn session_id(&self) -> SessionId {
        self.session_id
    }

    /// Get local node ID
    pub fn local_node_id(&self) -> NodeId {
        self.local_node_id
    }

    /// Remove replay state for a node (on disconnect)
    pub fn remove_peer(&mut self, node_id: NodeId) {
        self.replay_manager.remove_node(node_id);
    }
}

/// Decrypted frame with validated contents
#[derive(Debug)]
pub struct DecryptedFrame {
    /// Frame header
    pub header: FixedHeader,
    /// Extensions
    pub extensions: Extensions,
    /// Decrypted payload
    pub payload: Vec<u8>,
    /// Source node
    pub source: NodeId,
}

impl DecryptedFrame {
    /// Get packet class
    pub fn class(&self) -> PacketClass {
        self.header.class
    }

    /// Get time hint
    pub fn time_hint(&self) -> i32 {
        self.header.time_hint
    }

    /// Get sequence number
    pub fn seq(&self) -> u16 {
        self.header.seq()
    }
}

/// Batch frame processor for high-throughput scenarios
pub struct BatchFrameProcessor {
    processor: SecureFrameProcessor,
    /// Pending outgoing frames
    outgoing: Vec<Vec<u8>>,
    /// Maximum batch size
    max_batch: usize,
}

impl BatchFrameProcessor {
    pub fn new(processor: SecureFrameProcessor, max_batch: usize) -> Self {
        BatchFrameProcessor {
            processor,
            outgoing: Vec::new(),
            max_batch,
        }
    }

    /// Queue a frame for encryption
    pub fn queue_encrypt(
        &mut self,
        class: PacketClass,
        profile: RepresentationProfile,
        time_hint: i32,
        payload: &[u8],
    ) -> ElaraResult<()> {
        let encrypted =
            self.processor
                .encrypt_frame(class, profile, time_hint, Extensions::new(), payload)?;
        self.outgoing.push(encrypted);

        // Flush if batch is full
        if self.outgoing.len() >= self.max_batch {
            // Would trigger send here
        }

        Ok(())
    }

    /// Get pending outgoing frames
    pub fn drain_outgoing(&mut self) -> Vec<Vec<u8>> {
        std::mem::take(&mut self.outgoing)
    }

    /// Decrypt a batch of frames
    pub fn decrypt_batch(&mut self, frames: Vec<&[u8]>) -> Vec<ElaraResult<DecryptedFrame>> {
        frames
            .into_iter()
            .map(|data| self.processor.decrypt_frame(data))
            .collect()
    }

    /// Get inner processor
    pub fn processor(&self) -> &SecureFrameProcessor {
        &self.processor
    }

    /// Get mutable inner processor
    pub fn processor_mut(&mut self) -> &mut SecureFrameProcessor {
        &mut self.processor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_processors() -> (SecureFrameProcessor, SecureFrameProcessor) {
        let session_id = SessionId::new(12345);
        let session_key = [0x42u8; KEY_SIZE];
        let node1 = NodeId::new(1);
        let node2 = NodeId::new(2);

        let proc1 = SecureFrameProcessor::new(session_id, node1, session_key);
        let proc2 = SecureFrameProcessor::new(session_id, node2, session_key);

        (proc1, proc2)
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let (mut sender, mut receiver) = create_test_processors();

        let payload = b"Hello, ELARA!";
        let encrypted = sender
            .encrypt_frame(
                PacketClass::Core,
                RepresentationProfile::Textual,
                1000,
                Extensions::new(),
                payload,
            )
            .unwrap();

        let decrypted = receiver.decrypt_frame(&encrypted).unwrap();

        assert_eq!(decrypted.payload, payload);
        assert_eq!(decrypted.class(), PacketClass::Core);
        assert_eq!(decrypted.time_hint(), 1000);
    }

    #[test]
    fn test_different_classes() {
        let (mut sender, mut receiver) = create_test_processors();

        for class in [
            PacketClass::Core,
            PacketClass::Perceptual,
            PacketClass::Enhancement,
            PacketClass::Cosmetic,
            PacketClass::Repair,
        ] {
            let payload = format!("Payload for {:?}", class);
            let encrypted = sender
                .encrypt_frame(
                    class,
                    RepresentationProfile::Textual,
                    0,
                    Extensions::new(),
                    payload.as_bytes(),
                )
                .unwrap();

            let decrypted = receiver.decrypt_frame(&encrypted).unwrap();
            assert_eq!(decrypted.payload, payload.as_bytes());
            assert_eq!(decrypted.class(), class);
        }
    }

    #[test]
    fn test_replay_rejection() {
        let (mut sender, mut receiver) = create_test_processors();

        let payload = b"Test payload";
        let encrypted = sender
            .encrypt_frame(
                PacketClass::Core,
                RepresentationProfile::Textual,
                0,
                Extensions::new(),
                payload,
            )
            .unwrap();

        // First decryption should succeed
        let result1 = receiver.decrypt_frame(&encrypted);
        assert!(result1.is_ok());

        // Replay should be rejected
        let result2 = receiver.decrypt_frame(&encrypted);
        assert!(result2.is_err());
    }

    #[test]
    fn test_session_mismatch() {
        let session_key = [0x42u8; KEY_SIZE];
        let mut sender = SecureFrameProcessor::new(SessionId::new(1), NodeId::new(1), session_key);
        let mut receiver =
            SecureFrameProcessor::new(SessionId::new(2), NodeId::new(2), session_key);

        let encrypted = sender
            .encrypt_frame(
                PacketClass::Core,
                RepresentationProfile::Textual,
                0,
                Extensions::new(),
                b"test",
            )
            .unwrap();

        let result = receiver.decrypt_frame(&encrypted);
        assert!(matches!(result, Err(ElaraError::SessionMismatch)));
    }

    #[test]
    fn test_batch_processor() {
        let (proc1, _proc2) = create_test_processors();
        let mut batch = BatchFrameProcessor::new(proc1, 10);

        // Queue multiple frames
        for i in 0..5 {
            batch
                .queue_encrypt(
                    PacketClass::Core,
                    RepresentationProfile::Textual,
                    i * 100,
                    &[i as u8; 10],
                )
                .unwrap();
        }

        let outgoing = batch.drain_outgoing();
        assert_eq!(outgoing.len(), 5);
    }
}
