//! Cryptographic utilities for secure key management and data protection
//!
//! This module provides utilities for encryption, decryption, key derivation,
//! and secure random number generation used throughout the application.

use anyhow::{anyhow, Result};
use ring::{
    aead::{Aad, BoundKey, Nonce, NonceSequence, OpeningKey, SealingKey, UnboundKey, AES_256_GCM},
    rand::{SecureRandom, SystemRandom},
    pbkdf2,
};
use zeroize::{Zeroize, ZeroizeOnDrop};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use std::num::NonZeroU32;

/// AES-256-GCM key size in bytes
pub const AES_KEY_SIZE: usize = 32;

/// Salt size for key derivation
pub const SALT_SIZE: usize = 16;

/// Nonce size for AES-256-GCM
pub const NONCE_SIZE: usize = 12;

/// PBKDF2 iteration count for key derivation
pub const PBKDF2_ITERATIONS: u32 = 100_000;

/// Encrypted data container
#[derive(Debug, Clone)]
pub struct EncryptedData {
    /// The encrypted ciphertext
    pub ciphertext: Vec<u8>,
    /// The nonce used for encryption
    pub nonce: [u8; NONCE_SIZE],
    /// The salt used for key derivation (if applicable)
    pub salt: Option<[u8; SALT_SIZE]>,
}

/// Secure key material that zeros itself on drop
#[derive(Clone, ZeroizeOnDrop)]
pub struct SecureKey {
    key: [u8; AES_KEY_SIZE],
}

impl SecureKey {
    /// Create a new secure key from bytes
    pub fn from_bytes(bytes: [u8; AES_KEY_SIZE]) -> Self {
        Self { key: bytes }
    }

    /// Create a new secure key from a password using PBKDF2
    pub fn from_password(password: &str, salt: &[u8; SALT_SIZE]) -> Self {
        let mut key = [0u8; AES_KEY_SIZE];
        pbkdf2::derive(
            pbkdf2::PBKDF2_HMAC_SHA256,
            NonZeroU32::new(PBKDF2_ITERATIONS).unwrap(),
            salt,
            password.as_bytes(),
            &mut key,
        );
        Self::from_bytes(key)
    }

    /// Get the key bytes (use with caution)
    pub fn as_bytes(&self) -> &[u8; AES_KEY_SIZE] {
        &self.key
    }
}

impl std::fmt::Debug for SecureKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecureKey")
            .field("key", &"[REDACTED]")
            .finish()
    }
}

/// Nonce sequence for AES-GCM encryption
struct CounterNonceSequence {
    counter: u64,
}

impl CounterNonceSequence {
    fn new() -> Self {
        Self { counter: 0 }
    }
}

impl NonceSequence for CounterNonceSequence {
    fn advance(&mut self) -> Result<Nonce, ring::error::Unspecified> {
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        nonce_bytes[4..].copy_from_slice(&self.counter.to_be_bytes());
        self.counter += 1;
        Nonce::try_assume_unique_for_key(&nonce_bytes)
    }
}

/// Generate a cryptographically secure random salt
pub fn generate_salt() -> Result<[u8; SALT_SIZE]> {
    let mut salt = [0u8; SALT_SIZE];
    let rng = SystemRandom::new();
    rng.fill(&mut salt)
        .map_err(|_| anyhow!("Failed to generate random salt"))?;
    Ok(salt)
}

/// Generate a cryptographically secure random key
pub fn generate_key() -> Result<SecureKey> {
    let mut key_bytes = [0u8; AES_KEY_SIZE];
    let rng = SystemRandom::new();
    rng.fill(&mut key_bytes)
        .map_err(|_| anyhow!("Failed to generate random key"))?;
    Ok(SecureKey::from_bytes(key_bytes))
}

/// Generate a cryptographically secure random nonce
pub fn generate_nonce() -> Result<[u8; NONCE_SIZE]> {
    let mut nonce = [0u8; NONCE_SIZE];
    let rng = SystemRandom::new();
    rng.fill(&mut nonce)
        .map_err(|_| anyhow!("Failed to generate random nonce"))?;
    Ok(nonce)
}

/// Encrypt data using AES-256-GCM
pub fn encrypt_data(key: &SecureKey, data: &[u8]) -> Result<EncryptedData> {
    let nonce = generate_nonce()?;
    let nonce_obj = Nonce::try_assume_unique_for_key(&nonce)
        .map_err(|_| anyhow!("Failed to create nonce"))?;

    let unbound_key = UnboundKey::new(&AES_256_GCM, key.as_bytes())
        .map_err(|_| anyhow!("Failed to create encryption key"))?;

    let mut sealing_key = SealingKey::new(unbound_key, CounterNonceSequence::new());

    let mut ciphertext = data.to_vec();
    sealing_key
        .seal_in_place_append_tag(Aad::empty(), &mut ciphertext)
        .map_err(|_| anyhow!("Failed to encrypt data"))?;

    Ok(EncryptedData {
        ciphertext,
        nonce,
        salt: None,
    })
}

/// Decrypt data using AES-256-GCM
pub fn decrypt_data(key: &SecureKey, encrypted: &EncryptedData) -> Result<Vec<u8>> {
    let nonce = Nonce::try_assume_unique_for_key(&encrypted.nonce)
        .map_err(|_| anyhow!("Failed to create nonce for decryption"))?;

    let unbound_key = UnboundKey::new(&AES_256_GCM, key.as_bytes())
        .map_err(|_| anyhow!("Failed to create decryption key"))?;

    let mut opening_key = OpeningKey::new(unbound_key, CounterNonceSequence::new());

    let mut ciphertext = encrypted.ciphertext.clone();
    let plaintext = opening_key
        .open_in_place(Aad::empty(), &mut ciphertext)
        .map_err(|_| anyhow!("Failed to decrypt data - invalid key or corrupted data"))?;

    Ok(plaintext.to_vec())
}

/// Encrypt data with a password (includes key derivation)
pub fn encrypt_with_password(password: &str, data: &[u8]) -> Result<EncryptedData> {
    let salt = generate_salt()?;
    let key = SecureKey::from_password(password, &salt);

    let mut encrypted = encrypt_data(&key, data)?;
    encrypted.salt = Some(salt);

    Ok(encrypted)
}

/// Decrypt data with a password (includes key derivation)
pub fn decrypt_with_password(password: &str, encrypted: &EncryptedData) -> Result<Vec<u8>> {
    let salt = encrypted.salt
        .ok_or_else(|| anyhow!("Salt not found in encrypted data"))?;

    let key = SecureKey::from_password(password, &salt);
    decrypt_data(&key, encrypted)
}

/// Encode encrypted data to base64 string for storage
pub fn encode_encrypted_data(encrypted: &EncryptedData) -> String {
    let mut combined = Vec::new();

    // Add salt if present (16 bytes)
    if let Some(salt) = encrypted.salt {
        combined.extend_from_slice(&salt);
        combined.push(1); // Salt present flag
    } else {
        combined.push(0); // No salt flag
    }

    // Add nonce (12 bytes)
    combined.extend_from_slice(&encrypted.nonce);

    // Add ciphertext
    combined.extend_from_slice(&encrypted.ciphertext);

    BASE64.encode(combined)
}

/// Decode encrypted data from base64 string
pub fn decode_encrypted_data(encoded: &str) -> Result<EncryptedData> {
    let combined = BASE64.decode(encoded)
        .map_err(|e| anyhow!("Failed to decode base64: {}", e))?;

    if combined.is_empty() {
        return Err(anyhow!("Empty encrypted data"));
    }

    let mut offset = 0;

    // Check salt flag
    let has_salt = combined[offset] == 1;
    offset += 1;

    let salt = if has_salt {
        if combined.len() < offset + SALT_SIZE {
            return Err(anyhow!("Invalid encrypted data: insufficient salt bytes"));
        }
        let mut salt_bytes = [0u8; SALT_SIZE];
        salt_bytes.copy_from_slice(&combined[offset..offset + SALT_SIZE]);
        offset += SALT_SIZE;
        Some(salt_bytes)
    } else {
        None
    };

    // Extract nonce
    if combined.len() < offset + NONCE_SIZE {
        return Err(anyhow!("Invalid encrypted data: insufficient nonce bytes"));
    }
    let mut nonce = [0u8; NONCE_SIZE];
    nonce.copy_from_slice(&combined[offset..offset + NONCE_SIZE]);
    offset += NONCE_SIZE;

    // Extract ciphertext
    let ciphertext = combined[offset..].to_vec();

    Ok(EncryptedData {
        ciphertext,
        nonce,
        salt,
    })
}

/// Hash a password using PBKDF2 for secure storage
pub fn hash_password(password: &str, salt: &[u8; SALT_SIZE]) -> [u8; AES_KEY_SIZE] {
    let mut hash = [0u8; AES_KEY_SIZE];
    pbkdf2::derive(
        pbkdf2::PBKDF2_HMAC_SHA256,
        NonZeroU32::new(PBKDF2_ITERATIONS).unwrap(),
        salt,
        password.as_bytes(),
        &mut hash,
    );
    hash
}

/// Verify a password against a hash
pub fn verify_password(password: &str, salt: &[u8; SALT_SIZE], expected_hash: &[u8; AES_KEY_SIZE]) -> bool {
    let computed_hash = hash_password(password, salt);

    // Use constant-time comparison to prevent timing attacks
    use ring::constant_time;
    constant_time::verify_slices_are_equal(&computed_hash, expected_hash).is_ok()
}

/// Generate a secure random string of specified length
pub fn generate_random_string(length: usize) -> Result<String> {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

    let mut bytes = vec![0u8; length];
    let rng = SystemRandom::new();
    rng.fill(&mut bytes)
        .map_err(|_| anyhow!("Failed to generate random bytes"))?;

    let random_string: String = bytes
        .iter()
        .map(|&byte| CHARSET[byte as usize % CHARSET.len()] as char)
        .collect();

    Ok(random_string)
}

/// Generate a secure API key
pub fn generate_api_key() -> Result<String> {
    generate_random_string(32)
}

/// Generate a secure session token
pub fn generate_session_token() -> Result<String> {
    generate_random_string(64)
}

/// Secure memory clearing utility
pub fn secure_zero(data: &mut [u8]) {
    data.zeroize();
}

/// Timing-safe string comparison
pub fn constant_time_compare(a: &str, b: &str) -> bool {
    use ring::constant_time;

    if a.len() != b.len() {
        return false;
    }

    constant_time::verify_slices_are_equal(a.as_bytes(), b.as_bytes()).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_generation() {
        let key1 = generate_key().unwrap();
        let key2 = generate_key().unwrap();

        // Keys should be different
        assert_ne!(key1.as_bytes(), key2.as_bytes());
    }

    #[test]
    fn test_encryption_decryption() {
        let key = generate_key().unwrap();
        let data = b"Hello, World! This is a test message.";

        let encrypted = encrypt_data(&key, data).unwrap();
        let decrypted = decrypt_data(&key, &encrypted).unwrap();

        assert_eq!(data.as_slice(), decrypted.as_slice());
    }

    #[test]
    fn test_password_encryption() {
        let password = "super_secure_password_123";
        let data = b"Sensitive information that needs protection";

        let encrypted = encrypt_with_password(password, data).unwrap();
        let decrypted = decrypt_with_password(password, &encrypted).unwrap();

        assert_eq!(data.as_slice(), decrypted.as_slice());

        // Wrong password should fail
        let wrong_result = decrypt_with_password("wrong_password", &encrypted);
        assert!(wrong_result.is_err());
    }

    #[test]
    fn test_encoding_decoding() {
        let key = generate_key().unwrap();
        let data = b"Test data for encoding";

        let encrypted = encrypt_data(&key, data).unwrap();
        let encoded = encode_encrypted_data(&encrypted);
        let decoded = decode_encrypted_data(&encoded).unwrap();

        assert_eq!(encrypted.ciphertext, decoded.ciphertext);
        assert_eq!(encrypted.nonce, decoded.nonce);
    }

    #[test]
    fn test_password_hashing() {
        let password = "test_password";
        let salt = generate_salt().unwrap();

        let hash1 = hash_password(password, &salt);
        let hash2 = hash_password(password, &salt);

        // Same password and salt should produce same hash
        assert_eq!(hash1, hash2);

        // Verification should work
        assert!(verify_password(password, &salt, &hash1));
        assert!(!verify_password("wrong_password", &salt, &hash1));
    }

    #[test]
    fn test_random_string_generation() {
        let str1 = generate_random_string(32).unwrap();
        let str2 = generate_random_string(32).unwrap();

        assert_eq!(str1.len(), 32);
        assert_eq!(str2.len(), 32);
        assert_ne!(str1, str2);
    }

    #[test]
    fn test_constant_time_compare() {
        assert!(constant_time_compare("hello", "hello"));
        assert!(!constant_time_compare("hello", "world"));
        assert!(!constant_time_compare("hello", "hello2"));
    }

    #[test]
    fn test_secure_key_zeroize() {
        let mut key_bytes = [1u8; AES_KEY_SIZE];
        {
            let _key = SecureKey::from_bytes(key_bytes);
        } // key should be zeroized here

        // This test mainly checks that the code compiles and runs
        // The actual zeroization is handled by the ZeroizeOnDrop trait
    }
}