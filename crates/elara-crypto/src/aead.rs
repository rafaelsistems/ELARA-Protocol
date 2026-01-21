//! AEAD encryption using ChaCha20-Poly1305

use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Nonce,
};

use elara_core::{ElaraError, ElaraResult, NodeId, PacketClass};

/// Key size for ChaCha20-Poly1305
pub const KEY_SIZE: usize = 32;

/// Nonce size for ChaCha20-Poly1305
pub const NONCE_SIZE: usize = 12;

/// Tag size for ChaCha20-Poly1305
pub const TAG_SIZE: usize = 16;

/// AEAD cipher wrapper
pub struct AeadCipher {
    cipher: ChaCha20Poly1305,
}

impl AeadCipher {
    /// Create a new cipher from key bytes
    pub fn new(key: &[u8; KEY_SIZE]) -> Self {
        let cipher = ChaCha20Poly1305::new_from_slice(key).expect("Invalid key size");
        AeadCipher { cipher }
    }

    /// Encrypt plaintext with associated data
    /// Returns ciphertext with appended auth tag
    pub fn encrypt(&self, nonce: &[u8; NONCE_SIZE], aad: &[u8], plaintext: &[u8]) -> ElaraResult<Vec<u8>> {
        let nonce = Nonce::from_slice(nonce);
        
        self.cipher
            .encrypt(nonce, chacha20poly1305::aead::Payload { msg: plaintext, aad })
            .map_err(|_| ElaraError::DecryptionFailed)
    }

    /// Decrypt ciphertext with associated data
    /// Ciphertext should include the auth tag at the end
    pub fn decrypt(&self, nonce: &[u8; NONCE_SIZE], aad: &[u8], ciphertext: &[u8]) -> ElaraResult<Vec<u8>> {
        let nonce = Nonce::from_slice(nonce);
        
        self.cipher
            .decrypt(nonce, chacha20poly1305::aead::Payload { msg: ciphertext, aad })
            .map_err(|_| ElaraError::DecryptionFailed)
    }
}

/// Derive nonce from packet parameters (unique per packet)
pub fn derive_nonce(node_id: NodeId, seq: u16, class: PacketClass) -> [u8; NONCE_SIZE] {
    let mut nonce = [0u8; NONCE_SIZE];
    nonce[0..8].copy_from_slice(&node_id.to_bytes());
    nonce[8..10].copy_from_slice(&seq.to_le_bytes());
    nonce[10] = class.to_byte();
    nonce[11] = 0; // Reserved
    nonce
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let key = [0x42u8; KEY_SIZE];
        let cipher = AeadCipher::new(&key);
        
        let nonce = derive_nonce(NodeId::new(12345), 1, PacketClass::Core);
        let aad = b"header data";
        let plaintext = b"Hello, ELARA!";
        
        let ciphertext = cipher.encrypt(&nonce, aad, plaintext).unwrap();
        let decrypted = cipher.decrypt(&nonce, aad, &ciphertext).unwrap();
        
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_wrong_key_fails() {
        let key1 = [0x42u8; KEY_SIZE];
        let key2 = [0x43u8; KEY_SIZE];
        let cipher1 = AeadCipher::new(&key1);
        let cipher2 = AeadCipher::new(&key2);
        
        let nonce = derive_nonce(NodeId::new(1), 1, PacketClass::Core);
        let aad = b"header";
        let plaintext = b"secret";
        
        let ciphertext = cipher1.encrypt(&nonce, aad, plaintext).unwrap();
        let result = cipher2.decrypt(&nonce, aad, &ciphertext);
        
        assert!(result.is_err());
    }

    #[test]
    fn test_wrong_aad_fails() {
        let key = [0x42u8; KEY_SIZE];
        let cipher = AeadCipher::new(&key);
        
        let nonce = derive_nonce(NodeId::new(1), 1, PacketClass::Core);
        let plaintext = b"secret";
        
        let ciphertext = cipher.encrypt(&nonce, b"correct aad", plaintext).unwrap();
        let result = cipher.decrypt(&nonce, b"wrong aad", &ciphertext);
        
        assert!(result.is_err());
    }

    #[test]
    fn test_nonce_uniqueness() {
        let n1 = derive_nonce(NodeId::new(1), 1, PacketClass::Core);
        let n2 = derive_nonce(NodeId::new(1), 2, PacketClass::Core);
        let n3 = derive_nonce(NodeId::new(2), 1, PacketClass::Core);
        let n4 = derive_nonce(NodeId::new(1), 1, PacketClass::Perceptual);
        
        assert_ne!(n1, n2);
        assert_ne!(n1, n3);
        assert_ne!(n1, n4);
    }
}
