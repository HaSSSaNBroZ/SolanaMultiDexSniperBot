//! Encryption service implementation
//!
//! This module provides AES-256-GCM encryption, key management,
//! and secure data handling for the Solana Sniper Bot.

use crate::config::models::SecurityConfig;
use crate::core::result::AppResult;
use crate::core::error::AppError;
use crate::utils::crypto::{
    SecureKey, EncryptedData as CryptoEncryptedData,
    encrypt_data, decrypt_data, encrypt_with_password, decrypt_with_password,
    encode_encrypted_data, decode_encrypted_data, generate_salt, generate_key,
};

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc, Duration};
use tracing::{debug, info, warn, error, instrument};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde::{Serialize, Deserialize};
use zeroize::Zeroize;

/// Result type for encryption operations
pub type EncryptionResult<T> = Result<T, EncryptionError>;

/// Encryption-specific errors
#[derive(Debug, thiserror::Error)]
pub enum EncryptionError {
    /// Invalid key error
    #[error("Invalid encryption key: {0}")]
    InvalidKey(String),

    /// Encryption failed
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),

    /// Decryption failed
    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),

    /// Key not found
    #[error("Key not found: {0}")]
    KeyNotFound(String),

    /// Key expired
    #[error("Key expired: {0}")]
    KeyExpired(String),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),
}

/// Encrypted data wrapper with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedData {
    /// Base64 encoded encrypted data
    pub data: String,
    /// Encryption algorithm used
    pub algorithm: String,
    /// Key ID used for encryption
    pub key_id: Option<String>,
    /// Timestamp when encrypted
    pub encrypted_at: DateTime<Utc>,
    /// Optional metadata
    pub metadata: Option<HashMap<String, String>>,
}

impl EncryptedData {
    /// Create from crypto encrypted data
    fn from_crypto(crypto_data: CryptoEncryptedData, algorithm: &str, key_id: Option<String>) -> Self {
        Self {
            data: encode_encrypted_data(&crypto_data),
            algorithm: algorithm.to_string(),
            key_id,
            encrypted_at: Utc::now(),
            metadata: None,
        }
    }

    /// Convert to crypto encrypted data
    fn to_crypto(&self) -> AppResult<CryptoEncryptedData> {
        decode_encrypted_data(&self.data)
            .map_err(|e| AppError::security(format!("Failed to decode encrypted data: {}", e)))
    }
}

/// Encryption service for secure data handling
#[derive(Debug)]
pub struct EncryptionService {
    /// Key manager
    key_manager: Arc<KeyManager>,
    /// Configuration
    config: SecurityConfig,
    /// Encryption metrics
    metrics: Arc<RwLock<EncryptionMetrics>>,
}

impl EncryptionService {
    /// Create a new encryption service
    #[instrument(skip(config))]
    pub fn new(config: &SecurityConfig) -> AppResult<Self> {
        info!("🔐 Initializing encryption service");

        // Validate configuration
        Self::validate_config(config)?;

        // Initialize key manager
        let key_manager = Arc::new(KeyManager::new(config)?);

        // Initialize metrics
        let metrics = Arc::new(RwLock::new(EncryptionMetrics::new()));

        info!("✅ Encryption service initialized with {} algorithm",
              config.encryption_algorithm);

        Ok(Self {
            key_manager,
            config: config.clone(),
            metrics,
        })
    }

    /// Validate encryption configuration
    fn validate_config(config: &SecurityConfig) -> AppResult<()> {
        if config.encryption_key.is_empty() {
            return Err(AppError::security("Encryption key not configured"));
        }

        // Validate algorithm
        match config.encryption_algorithm.as_str() {
            "AES-256-GCM" | "AES-256-CBC" | "ChaCha20-Poly1305" => {},
            _ => return Err(AppError::security(
                format!("Unsupported encryption algorithm: {}", config.encryption_algorithm)
            )),
        }

        Ok(())
    }

    /// Encrypt data using the default key
    #[instrument(skip(self, data))]
    pub async fn encrypt(&self, data: &[u8]) -> EncryptionResult<EncryptedData> {
        let start_time = std::time::Instant::now();

        // Get default key
        let key = self.key_manager.get_default_key().await?;

        // Encrypt data
        let encrypted = encrypt_data(&key, data)
            .map_err(|e| EncryptionError::EncryptionFailed(e.to_string()))?;

        // Update metrics
        self.update_metrics("encrypt", true, start_time.elapsed()).await;

        Ok(EncryptedData::from_crypto(
            encrypted,
            &self.config.encryption_algorithm,
            Some("default".to_string())
        ))
    }

    /// Decrypt data using the appropriate key
    #[instrument(skip(self, encrypted))]
    pub async fn decrypt(&self, encrypted: &EncryptedData) -> EncryptionResult<Vec<u8>> {
        let start_time = std::time::Instant::now();

        // Validate algorithm
        if encrypted.algorithm != self.config.encryption_algorithm {
            return Err(EncryptionError::InvalidConfiguration(
                format!("Algorithm mismatch: expected {}, got {}",
                        self.config.encryption_algorithm, encrypted.algorithm)
            ));
        }

        // Get key
        let key_id = encrypted.key_id.as_deref().unwrap_or("default");
        let key = self.key_manager.get_key(key_id).await?;

        // Convert to crypto format
        let crypto_data = encrypted.to_crypto()
            .map_err(|e| EncryptionError::DecryptionFailed(e.to_string()))?;

        // Decrypt data
        let decrypted = decrypt_data(&key, &crypto_data)
            .map_err(|e| EncryptionError::DecryptionFailed(e.to_string()))?;

        // Update metrics
        self.update_metrics("decrypt", true, start_time.elapsed()).await;

        Ok(decrypted)
    }

    /// Encrypt string data
    pub async fn encrypt_string(&self, data: &str) -> EncryptionResult<EncryptedData> {
        self.encrypt(data.as_bytes()).await
    }

    /// Decrypt to string
    pub async fn decrypt_string(&self, encrypted: &EncryptedData) -> EncryptionResult<String> {
        let decrypted = self.decrypt(encrypted).await?;
        String::from_utf8(decrypted)
            .map_err(|e| EncryptionError::DecryptionFailed(
                format!("Invalid UTF-8 in decrypted data: {}", e)
            ))
    }

    /// Encrypt JSON serializable data
    pub async fn encrypt_json<T: Serialize>(&self, data: &T) -> EncryptionResult<EncryptedData> {
        let json = serde_json::to_vec(data)
            .map_err(|e| EncryptionError::EncryptionFailed(
                format!("Failed to serialize data: {}", e)
            ))?;

        self.encrypt(&json).await
    }

    /// Decrypt JSON data
    pub async fn decrypt_json<T: for<'de> Deserialize<'de>>(&self, encrypted: &EncryptedData) -> EncryptionResult<T> {
        let decrypted = self.decrypt(encrypted).await?;

        serde_json::from_slice(&decrypted)
            .map_err(|e| EncryptionError::DecryptionFailed(
                format!("Failed to deserialize data: {}", e)
            ))
    }

    /// Encrypt with password (for user data)
    #[instrument(skip(self, password, data))]
    pub async fn encrypt_with_password(&self, password: &str, data: &[u8]) -> EncryptionResult<EncryptedData> {
        let start_time = std::time::Instant::now();

        let encrypted = encrypt_with_password(password, data)
            .map_err(|e| EncryptionError::EncryptionFailed(e.to_string()))?;

        // Update metrics
        self.update_metrics("encrypt_password", true, start_time.elapsed()).await;

        Ok(EncryptedData::from_crypto(
            encrypted,
            &self.config.encryption_algorithm,
            None
        ))
    }

    /// Decrypt with password
    #[instrument(skip(self, password, encrypted))]
    pub async fn decrypt_with_password(&self, password: &str, encrypted: &EncryptedData) -> EncryptionResult<Vec<u8>> {
        let start_time = std::time::Instant::now();

        let crypto_data = encrypted.to_crypto()
            .map_err(|e| EncryptionError::DecryptionFailed(e.to_string()))?;

        let decrypted = decrypt_with_password(password, &crypto_data)
            .map_err(|e| EncryptionError::DecryptionFailed(e.to_string()))?;

        // Update metrics
        self.update_metrics("decrypt_password", true, start_time.elapsed()).await;

        Ok(decrypted)
    }

    /// Rotate encryption keys
    #[instrument(skip(self))]
    pub async fn rotate_keys(&self) -> EncryptionResult<()> {
        info!("🔄 Starting key rotation");

        self.key_manager.rotate_keys().await?;

        info!("✅ Key rotation completed");
        Ok(())
    }

    /// Clear sensitive data from memory
    pub async fn clear_sensitive_data(&self) -> AppResult<()> {
        debug!("🧹 Clearing sensitive data from memory");

        self.key_manager.clear_keys().await?;

        Ok(())
    }

    /// Get encryption metrics
    pub async fn get_metrics(&self) -> EncryptionMetrics {
        self.metrics.read().await.clone()
    }

    /// Health check
    pub async fn health_check(&self) -> AppResult<()> {
        // Verify we can encrypt/decrypt
        let test_data = b"health_check_test";
        let encrypted = self.encrypt(test_data).await
            .map_err(|e| AppError::security(format!("Encryption health check failed: {}", e)))?;

        let decrypted = self.decrypt(&encrypted).await
            .map_err(|e| AppError::security(format!("Decryption health check failed: {}", e)))?;

        if decrypted != test_data {
            return Err(AppError::security("Encryption/decryption health check failed: data mismatch"));
        }

        Ok(())
    }

    /// Update metrics
    async fn update_metrics(&self, operation: &str, success: bool, duration: std::time::Duration) {
        let mut metrics = self.metrics.write().await;

        metrics.total_operations += 1;
        if success {
            metrics.successful_operations += 1;
        } else {
            metrics.failed_operations += 1;
        }

        match operation {
            "encrypt" => metrics.encryptions += 1,
            "decrypt" => metrics.decryptions += 1,
            "encrypt_password" => metrics.password_encryptions += 1,
            "decrypt_password" => metrics.password_decryptions += 1,
            _ => {}
        }

        metrics.total_duration += duration;
        metrics.last_operation_at = Some(Utc::now());
    }
}

/// Key manager for handling encryption keys
#[derive(Debug)]
pub struct KeyManager {
    /// Master key derived from configuration
    master_key: Arc<RwLock<SecureKey>>,
    /// Active keys mapped by ID
    keys: Arc<RwLock<HashMap<String, KeyEntry>>>,
    /// Key rotation configuration
    rotation_config: KeyRotationConfig,
    /// Last rotation timestamp
    last_rotation: Arc<RwLock<Option<DateTime<Utc>>>>,
}

impl KeyManager {
    /// Create a new key manager
    fn new(config: &SecurityConfig) -> AppResult<Self> {
        // Decode master key from configuration
        let key_bytes = BASE64.decode(&config.encryption_key)
            .map_err(|e| AppError::security(format!("Invalid encryption key encoding: {}", e)))?;

        if key_bytes.len() != 32 {
            return Err(AppError::security(
                format!("Invalid encryption key size: expected 32 bytes, got {}", key_bytes.len())
            ));
        }

        let mut key_array = [0u8; 32];
        key_array.copy_from_slice(&key_bytes);
        let master_key = SecureKey::from_bytes(key_array);

        // Create rotation configuration
        let rotation_config = KeyRotationConfig {
            enabled: config.key_rotation_days.is_some(),
            rotation_interval: config.key_rotation_days
                .map(|days| Duration::days(days as i64)),
            retain_old_keys: Duration::days(7), // Keep old keys for 7 days
        };

        let mut keys = HashMap::new();

        // Add default key
        keys.insert("default".to_string(), KeyEntry {
            key: master_key.clone(),
            created_at: Utc::now(),
            expires_at: None,
            is_active: true,
        });

        Ok(Self {
            master_key: Arc::new(RwLock::new(master_key)),
            keys: Arc::new(RwLock::new(keys)),
            rotation_config,
            last_rotation: Arc::new(RwLock::new(None)),
        })
    }

    /// Get default key
    async fn get_default_key(&self) -> EncryptionResult<SecureKey> {
        self.get_key("default").await
    }

    /// Get key by ID
    async fn get_key(&self, key_id: &str) -> EncryptionResult<SecureKey> {
        let keys = self.keys.read().await;

        match keys.get(key_id) {
            Some(entry) => {
                if !entry.is_active {
                    return Err(EncryptionError::KeyExpired(key_id.to_string()));
                }

                if let Some(expires_at) = entry.expires_at {
                    if Utc::now() > expires_at {
                        return Err(EncryptionError::KeyExpired(key_id.to_string()));
                    }
                }

                Ok(entry.key.clone())
            }
            None => Err(EncryptionError::KeyNotFound(key_id.to_string())),
        }
    }

    /// Rotate keys
    async fn rotate_keys(&self) -> EncryptionResult<()> {
        if !self.rotation_config.enabled {
            return Ok(());
        }

        let mut keys = self.keys.write().await;
        let mut last_rotation = self.last_rotation.write().await;

        // Generate new key
        let new_key = generate_key()
            .map_err(|e| EncryptionError::InvalidKey(e.to_string()))?;

        // Create new key ID with timestamp
        let new_key_id = format!("key_{}", Utc::now().timestamp());

        // Mark old default key as inactive
        if let Some(old_default) = keys.get_mut("default") {
            old_default.is_active = false;
            old_default.expires_at = Some(Utc::now() + self.rotation_config.retain_old_keys);
        }

        // Add new key
        keys.insert(new_key_id.clone(), KeyEntry {
            key: new_key.clone(),
            created_at: Utc::now(),
            expires_at: None,
            is_active: true,
        });

        // Update default to point to new key
        keys.insert("default".to_string(), KeyEntry {
            key: new_key,
            created_at: Utc::now(),
            expires_at: None,
            is_active: true,
        });

        // Clean up expired keys
        keys.retain(|id, entry| {
            if id == "default" || id == &new_key_id {
                return true;
            }

            if let Some(expires_at) = entry.expires_at {
                expires_at > Utc::now()
            } else {
                entry.is_active
            }
        });

        *last_rotation = Some(Utc::now());

        Ok(())
    }

    /// Clear all keys from memory
    async fn clear_keys(&self) -> AppResult<()> {
        let mut keys = self.keys.write().await;

        // Clear will trigger zeroization for SecureKey
        keys.clear();

        Ok(())
    }
}

/// Key entry with metadata
#[derive(Debug, Clone)]
struct KeyEntry {
    /// The encryption key
    key: SecureKey,
    /// When the key was created
    created_at: DateTime<Utc>,
    /// When the key expires
    expires_at: Option<DateTime<Utc>>,
    /// Whether the key is active
    is_active: bool,
}

/// Key rotation configuration
#[derive(Debug, Clone)]
struct KeyRotationConfig {
    /// Whether rotation is enabled
    enabled: bool,
    /// Rotation interval
    rotation_interval: Option<Duration>,
    /// How long to retain old keys
    retain_old_keys: Duration,
}

/// Key derivation methods
#[derive(Debug, Clone)]
pub struct KeyDerivation;

impl KeyDerivation {
    /// Derive a key from password
    pub fn from_password(password: &str, salt: &[u8; 16]) -> SecureKey {
        SecureKey::from_password(password, salt)
    }

    /// Generate a new salt
    pub fn generate_salt() -> AppResult<[u8; 16]> {
        generate_salt()
            .map_err(|e| AppError::security(format!("Failed to generate salt: {}", e)))
    }
}

/// Encryption metrics
#[derive(Debug, Clone)]
pub struct EncryptionMetrics {
    /// Total operations
    pub total_operations: u64,
    /// Successful operations
    pub successful_operations: u64,
    /// Failed operations
    pub failed_operations: u64,
    /// Total encryptions
    pub encryptions: u64,
    /// Total decryptions
    pub decryptions: u64,
    /// Password-based encryptions
    pub password_encryptions: u64,
    /// Password-based decryptions
    pub password_decryptions: u64,
    /// Total operation duration
    pub total_duration: std::time::Duration,
    /// Last operation timestamp
    pub last_operation_at: Option<DateTime<Utc>>,
    /// Service started at
    pub started_at: DateTime<Utc>,
}

impl EncryptionMetrics {
    fn new() -> Self {
        Self {
            total_operations: 0,
            successful_operations: 0,
            failed_operations: 0,
            encryptions: 0,
            decryptions: 0,
            password_encryptions: 0,
            password_decryptions: 0,
            total_duration: std::time::Duration::ZERO,
            last_operation_at: None,
            started_at: Utc::now(),
        }
    }

    /// Calculate success rate
    pub fn success_rate(&self) -> f64 {
        if self.total_operations == 0 {
            return 0.0;
        }
        (self.successful_operations as f64 / self.total_operations as f64) * 100.0
    }

    /// Calculate average operation duration
    pub fn average_duration(&self) -> std::time::Duration {
        if self.total_operations == 0 {
            return std::time::Duration::ZERO;
        }
        self.total_duration / self.total_operations as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConfigLoader;

    #[tokio::test]
    async fn test_encryption_service() {
        let mut config = ConfigLoader::new().without_env().create_default_config();

        // Set a valid test key (32 bytes base64 encoded)
        config.security.encryption_key = BASE64.encode([0u8; 32]);

        let service = EncryptionService::new(&config.security).unwrap();

        // Test string encryption/decryption
        let test_data = "Hello, Solana Sniper Bot!";
        let encrypted = service.encrypt_string(test_data).await.unwrap();
        let decrypted = service.decrypt_string(&encrypted).await.unwrap();

        assert_eq!(test_data, decrypted);
    }

    #[tokio::test]
    async fn test_json_encryption() {
        let mut config = ConfigLoader::new().without_env().create_default_config();
        config.security.encryption_key = BASE64.encode([1u8; 32]);

        let service = EncryptionService::new(&config.security).unwrap();

        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct TestData {
            id: u32,
            name: String,
            active: bool,
        }

        let test_data = TestData {
            id: 42,
            name: "Test".to_string(),
            active: true,
        };

        let encrypted = service.encrypt_json(&test_data).await.unwrap();
        let decrypted: TestData = service.decrypt_json(&encrypted).await.unwrap();

        assert_eq!(test_data, decrypted);
    }

    #[tokio::test]
    async fn test_password_encryption() {
        let mut config = ConfigLoader::new().without_env().create_default_config();
        config.security.encryption_key = BASE64.encode([2u8; 32]);

        let service = EncryptionService::new(&config.security).unwrap();

        let password = "my_secure_password";
        let test_data = b"Sensitive wallet data";

        let encrypted = service.encrypt_with_password(password, test_data).await.unwrap();
        let decrypted = service.decrypt_with_password(password, &encrypted).await.unwrap();

        assert_eq!(test_data.to_vec(), decrypted);

        // Wrong password should fail
        let wrong_password = "wrong_password";
        let result = service.decrypt_with_password(wrong_password, &encrypted).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_health_check() {
        let mut config = ConfigLoader::new().without_env().create_default_config();
        config.security.encryption_key = BASE64.encode([3u8; 32]);

        let service = EncryptionService::new(&config.security).unwrap();

        let result = service.health_check().await;
        assert!(result.is_ok());
    }
}