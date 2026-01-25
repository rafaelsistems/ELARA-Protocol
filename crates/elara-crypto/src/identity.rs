//! Identity management using Ed25519

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};

use elara_core::NodeId;

/// Identity keypair for a node
#[derive(Clone)]
pub struct Identity {
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
    node_id: NodeId,
}

impl Identity {
    /// Generate a new random identity
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        let node_id = Self::derive_node_id(&verifying_key);

        Identity {
            signing_key,
            verifying_key,
            node_id,
        }
    }

    /// Create identity from existing signing key bytes
    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(bytes);
        let verifying_key = signing_key.verifying_key();
        let node_id = Self::derive_node_id(&verifying_key);

        Identity {
            signing_key,
            verifying_key,
            node_id,
        }
    }

    /// Get the signing key bytes (secret)
    pub fn signing_key_bytes(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }

    /// Get the verifying key bytes (public)
    pub fn verifying_key_bytes(&self) -> [u8; 32] {
        self.verifying_key.to_bytes()
    }

    /// Get the node ID (truncated hash of public key)
    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    /// Sign a message
    pub fn sign(&self, message: &[u8]) -> [u8; 64] {
        let signature = self.signing_key.sign(message);
        signature.to_bytes()
    }

    /// Verify a signature
    pub fn verify(&self, message: &[u8], signature: &[u8; 64]) -> bool {
        let sig = Signature::from_bytes(signature);
        self.verifying_key.verify(message, &sig).is_ok()
    }

    /// Derive NodeId from public key (first 8 bytes of SHA-256)
    fn derive_node_id(verifying_key: &VerifyingKey) -> NodeId {
        let mut hasher = Sha256::new();
        hasher.update(verifying_key.as_bytes());
        let hash = hasher.finalize();
        let id_bytes: [u8; 8] = hash[0..8].try_into().unwrap();
        NodeId::from_bytes(id_bytes)
    }
}

impl std::fmt::Debug for Identity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Identity")
            .field("node_id", &self.node_id)
            .finish_non_exhaustive()
    }
}

/// Public identity for verification only
#[derive(Clone)]
pub struct PublicIdentity {
    verifying_key: VerifyingKey,
    node_id: NodeId,
}

impl PublicIdentity {
    /// Create from verifying key bytes
    pub fn from_bytes(bytes: &[u8; 32]) -> Option<Self> {
        let verifying_key = VerifyingKey::from_bytes(bytes).ok()?;
        let node_id = Identity::derive_node_id(&verifying_key);
        Some(PublicIdentity {
            verifying_key,
            node_id,
        })
    }

    /// Get the node ID
    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    /// Verify a signature
    pub fn verify(&self, message: &[u8], signature: &[u8; 64]) -> bool {
        let sig = Signature::from_bytes(signature);
        self.verifying_key.verify(message, &sig).is_ok()
    }

    /// Get the verifying key bytes
    pub fn to_bytes(&self) -> [u8; 32] {
        self.verifying_key.to_bytes()
    }
}

impl std::fmt::Debug for PublicIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PublicIdentity")
            .field("node_id", &self.node_id)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_generation() {
        let id1 = Identity::generate();
        let id2 = Identity::generate();

        // Different identities should have different node IDs
        assert_ne!(id1.node_id(), id2.node_id());
    }

    #[test]
    fn test_sign_verify() {
        let identity = Identity::generate();
        let message = b"Hello, ELARA!";

        let signature = identity.sign(message);
        assert!(identity.verify(message, &signature));

        // Wrong message should fail
        assert!(!identity.verify(b"Wrong message", &signature));
    }

    #[test]
    fn test_public_identity() {
        let identity = Identity::generate();
        let public = PublicIdentity::from_bytes(&identity.verifying_key_bytes()).unwrap();

        assert_eq!(identity.node_id(), public.node_id());

        let message = b"Test message";
        let signature = identity.sign(message);
        assert!(public.verify(message, &signature));
    }

    #[test]
    fn test_identity_roundtrip() {
        let identity = Identity::generate();
        let bytes = identity.signing_key_bytes();
        let restored = Identity::from_bytes(&bytes);

        assert_eq!(identity.node_id(), restored.node_id());
    }
}
