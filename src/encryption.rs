use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{CashewError, Result};

/// Encryption metadata stored alongside a CID.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptionInfo {
    /// SHA-256 hash of the encryption key, base64-encoded.
    pub key_hash: String,
    /// AES-GCM nonce/IV, base64-encoded.
    pub iv: String,
}

impl EncryptionInfo {
    pub fn new(key: &[u8; 32], iv: &[u8]) -> Self {
        let key_hash = {
            let mut hasher = Sha256::new();
            hasher.update(key);
            BASE64.encode(hasher.finalize())
        };
        Self {
            key_hash,
            iv: BASE64.encode(iv),
        }
    }

    pub fn iv_bytes(&self) -> Result<Vec<u8>> {
        BASE64.decode(&self.iv).map_err(|_| CashewError::InvalidIV)
    }
}

/// Encrypts data with AES-256-GCM, returning (ciphertext, nonce).
pub fn encrypt(data: &[u8], key: &[u8; 32]) -> Result<(Vec<u8>, Vec<u8>)> {
    let cipher = Aes256Gcm::new(key.into());
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, data)
        .map_err(|e| CashewError::EncryptionFailed(e.to_string()))?;

    Ok((ciphertext, nonce_bytes.to_vec()))
}

/// Encrypts data with a specific nonce (for deterministic re-encryption).
pub fn encrypt_with_nonce(data: &[u8], key: &[u8; 32], nonce_bytes: &[u8]) -> Result<Vec<u8>> {
    let cipher = Aes256Gcm::new(key.into());
    let nonce = Nonce::from_slice(nonce_bytes);
    cipher
        .encrypt(nonce, data)
        .map_err(|e| CashewError::EncryptionFailed(e.to_string()))
}

/// Decrypts AES-256-GCM ciphertext.
pub fn decrypt(ciphertext: &[u8], key: &[u8; 32], nonce_bytes: &[u8]) -> Result<Vec<u8>> {
    let cipher = Aes256Gcm::new(key.into());
    let nonce = Nonce::from_slice(nonce_bytes);
    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| CashewError::DecryptionFailed(e.to_string()))
}

/// Strategy for encrypting nodes in the Merkle tree.
#[derive(Clone, Debug)]
pub enum EncryptionStrategy {
    /// Encrypt only the targeted node.
    Targeted([u8; 32]),
    /// Encrypt all direct children (one level).
    List([u8; 32]),
    /// Encrypt the entire subtree recursively.
    Recursive([u8; 32]),
}

impl EncryptionStrategy {
    pub fn key(&self) -> &[u8; 32] {
        match self {
            Self::Targeted(k) | Self::List(k) | Self::Recursive(k) => k,
        }
    }
}

/// Provides decryption keys by their hash.
pub trait KeyProvider: Send + Sync {
    fn key_for_hash(&self, key_hash: &str) -> Option<[u8; 32]>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = [42u8; 32];
        let plaintext = b"hello, merkle world!";
        let (ciphertext, nonce) = encrypt(plaintext, &key).unwrap();
        let decrypted = decrypt(&ciphertext, &key, &nonce).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_with_nonce_deterministic() {
        let key = [7u8; 32];
        let nonce = [1u8; 12];
        let plaintext = b"deterministic test";
        let ct1 = encrypt_with_nonce(plaintext, &key, &nonce).unwrap();
        let ct2 = encrypt_with_nonce(plaintext, &key, &nonce).unwrap();
        assert_eq!(ct1, ct2);
    }

    #[test]
    fn test_wrong_key_fails() {
        let key = [42u8; 32];
        let wrong_key = [99u8; 32];
        let plaintext = b"secret data";
        let (ciphertext, nonce) = encrypt(plaintext, &key).unwrap();
        assert!(decrypt(&ciphertext, &wrong_key, &nonce).is_err());
    }

    #[test]
    fn test_encryption_info() {
        let key = [42u8; 32];
        let iv = [1u8; 12];
        let info = EncryptionInfo::new(&key, &iv);
        assert!(!info.key_hash.is_empty());
        assert!(!info.iv.is_empty());
        let decoded_iv = info.iv_bytes().unwrap();
        assert_eq!(decoded_iv, iv);
    }

    #[test]
    fn test_encryption_strategy_key() {
        let key = [5u8; 32];
        let targeted = EncryptionStrategy::Targeted(key);
        let list = EncryptionStrategy::List(key);
        let recursive = EncryptionStrategy::Recursive(key);
        assert_eq!(targeted.key(), &key);
        assert_eq!(list.key(), &key);
        assert_eq!(recursive.key(), &key);
    }
}
