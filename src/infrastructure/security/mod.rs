//! Security infrastructure module
//!
//! This module provides comprehensive security services including encryption,
//! authentication, authorization, and audit logging for the Solana Sniper Bot.

pub mod authentication;
pub mod encryption;

// Re-export commonly used types
pub use authentication::{
    AuthenticationService, AuthenticationResult, AuthenticationError,
    SessionManager, Session, SessionToken, UserRole, Permission,
};
pub use encryption::{
    EncryptionService, EncryptionResult, EncryptionError,
    EncryptedData, KeyManager, KeyDerivation,
};

use crate::config::AppConfig;
use crate::core::result::AppResult;
use crate::core::error::AppError;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error, instrument};

/// Security service coordinator
#[derive(Debug, Clone)]
pub struct SecurityService {
    /// Encryption service
    pub encryption: EncryptionService,
    /// Authentication service
    pub authentication: AuthenticationService,
    /// Security configuration
    config: Arc<AppConfig>,
    /// Security event logger
    event_logger: Arc<SecurityEventLogger>,
}

impl SecurityService {
    /// Create a new security service
    #[instrument(skip(config))]
    pub fn new(config: &AppConfig) -> AppResult<Self> {
        info!("🔐 Initializing security services");

        // Initialize encryption service
        let encryption = EncryptionService::new(&config.security)?;

        // Initialize authentication service
        let authentication = AuthenticationService::new(&config.security, &config.telegram)?;

        // Initialize event logger
        let event_logger = Arc::new(SecurityEventLogger::new());

        info!("✅ Security services initialized");

        Ok(Self {
            encryption,
            authentication,
            config: Arc::new(config.clone()),
            event_logger,
        })
    }

    /// Log a security event
    pub async fn log_security_event(&self, event: SecurityEvent) {
        self.event_logger.log_event(event).await;
    }

    /// Validate security configuration
    pub fn validate_configuration(&self) -> AppResult<()> {
        // Check encryption key strength
        if self.config.security.encryption_key.is_empty() {
            return Err(AppError::security("Encryption key not configured"));
        }

        // Validate session timeout
        if self.config.security.session_timeout_minutes == 0 {
            return Err(AppError::security("Invalid session timeout"));
        }

        // Check if production requires stronger security
        if self.config.is_production() {
            if !self.config.security.enable_2fa {
                warn!("⚠️  Two-factor authentication disabled in production");
            }

            if self.config.security.allow_unsafe_operations {
                return Err(AppError::security("Unsafe operations enabled in production"));
            }

            if self.config.security.log_sensitive_data {
                return Err(AppError::security("Sensitive data logging enabled in production"));
            }
        }

        Ok(())
    }

    /// Perform security health check
    pub async fn health_check(&self) -> SecurityHealthStatus {
        let mut status = SecurityHealthStatus::default();

        // Check encryption service
        match self.encryption.health_check().await {
            Ok(_) => {
                status.encryption_healthy = true;
            }
            Err(e) => {
                error!("Encryption service unhealthy: {}", e);
                status.issues.push(format!("Encryption: {}", e));
            }
        }

        // Check authentication service
        match self.authentication.health_check().await {
            Ok(_) => {
                status.authentication_healthy = true;
            }
            Err(e) => {
                error!("Authentication service unhealthy: {}", e);
                status.issues.push(format!("Authentication: {}", e));
            }
        }

        // Check audit logging
        if self.config.security.enable_audit_logging {
            status.audit_logging_enabled = true;
        }

        // Overall health
        status.overall_healthy = status.encryption_healthy &&
            status.authentication_healthy &&
            status.issues.is_empty();

        status
    }

    /// Shutdown security services
    pub async fn shutdown(&self) -> AppResult<()> {
        info!("🛑 Shutting down security services");

        // Flush security event logs
        self.event_logger.flush().await?;

        // Clear sensitive data from memory
        self.encryption.clear_sensitive_data().await?;
        self.authentication.clear_sessions().await?;

        info!("✅ Security services shut down");
        Ok(())
    }
}

/// Security event types
#[derive(Debug, Clone)]
pub enum SecurityEvent {
    /// Authentication attempt
    AuthenticationAttempt {
        user_id: String,
        success: bool,
        method: String,
        ip_address: Option<String>,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// Session created
    SessionCreated {
        session_id: String,
        user_id: String,
        expires_at: chrono::DateTime<chrono::Utc>,
    },
    /// Session terminated
    SessionTerminated {
        session_id: String,
        reason: String,
    },
    /// Access denied
    AccessDenied {
        user_id: Option<String>,
        resource: String,
        reason: String,
    },
    /// Configuration changed
    ConfigurationChanged {
        user_id: String,
        field: String,
        old_value: String,
        new_value: String,
    },
    /// Encryption operation
    EncryptionOperation {
        operation: String,
        key_id: Option<String>,
        success: bool,
    },
    /// Security violation
    SecurityViolation {
        violation_type: String,
        details: String,
        severity: SecuritySeverity,
    },
}

/// Security severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecuritySeverity {
    /// Low severity
    Low,
    /// Medium severity
    Medium,
    /// High severity
    High,
    /// Critical severity
    Critical,
}

/// Security event logger
#[derive(Debug)]
struct SecurityEventLogger {
    /// Event buffer
    events: Arc<RwLock<Vec<SecurityEvent>>>,
    /// Maximum buffer size
    max_buffer_size: usize,
}

impl SecurityEventLogger {
    fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
            max_buffer_size: 1000,
        }
    }

    async fn log_event(&self, event: SecurityEvent) {
        let mut events = self.events.write().await;

        // Log to tracing
        match &event {
            SecurityEvent::AuthenticationAttempt { user_id, success, .. } => {
                if *success {
                    info!("🔓 Authentication successful for user: {}", user_id);
                } else {
                    warn!("🔒 Authentication failed for user: {}", user_id);
                }
            }
            SecurityEvent::SecurityViolation { violation_type, details, severity } => {
                match severity {
                    SecuritySeverity::Critical => {
                        error!("🚨 CRITICAL security violation: {} - {}", violation_type, details);
                    }
                    SecuritySeverity::High => {
                        error!("⚠️  High security violation: {} - {}", violation_type, details);
                    }
                    SecuritySeverity::Medium => {
                        warn!("⚠️  Medium security violation: {} - {}", violation_type, details);
                    }
                    SecuritySeverity::Low => {
                        info!("ℹ️  Low security violation: {} - {}", violation_type, details);
                    }
                }
            }
            _ => {}
        }

        // Add to buffer
        events.push(event);

        // Prevent unbounded growth
        if events.len() > self.max_buffer_size {
            events.drain(0..100); // Remove oldest 100 events
        }
    }

    async fn flush(&self) -> AppResult<()> {
        let mut events = self.events.write().await;

        // In production, this would persist events to database
        info!("📊 Flushing {} security events", events.len());

        events.clear();
        Ok(())
    }

    async fn get_recent_events(&self, count: usize) -> Vec<SecurityEvent> {
        let events = self.events.read().await;
        events.iter()
            .rev()
            .take(count)
            .cloned()
            .collect()
    }
}

/// Security health status
#[derive(Debug, Clone, Default)]
pub struct SecurityHealthStatus {
    /// Overall health status
    pub overall_healthy: bool,
    /// Encryption service health
    pub encryption_healthy: bool,
    /// Authentication service health
    pub authentication_healthy: bool,
    /// Audit logging enabled
    pub audit_logging_enabled: bool,
    /// Security issues
    pub issues: Vec<String>,
}

/// Security utility functions
pub mod utils {
    use super::*;
    use rand::{Rng, thread_rng};
    use std::net::IpAddr;

    /// Generate a secure random token
    pub fn generate_secure_token(length: usize) -> String {
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
        let mut rng = thread_rng();

        (0..length)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }

    /// Validate IP address for whitelisting
    pub fn validate_ip_whitelist(ip: &str, allowed_ranges: &[String]) -> bool {
        if allowed_ranges.is_empty() {
            return true; // No whitelist configured
        }

        match ip.parse::<IpAddr>() {
            Ok(addr) => {
                // Check if IP is in any allowed range
                allowed_ranges.iter().any(|range| {
                    if range.contains('/') {
                        // CIDR notation support would be implemented here
                        // For now, exact match
                        range == ip
                    } else {
                        // Exact match
                        range == ip
                    }
                })
            }
            Err(_) => false,
        }
    }

    /// Sanitize user input for logging
    pub fn sanitize_for_logging(input: &str) -> String {
        input
            .chars()
            .filter(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.' | '@'))
            .take(100) // Limit length
            .collect()
    }

    /// Mask sensitive data for display
    pub fn mask_sensitive_data(data: &str, visible_chars: usize) -> String {
        if data.len() <= visible_chars {
            return "*".repeat(data.len());
        }

        let visible_part = &data[..visible_chars];
        let masked_length = data.len() - visible_chars;
        format!("{}...{}", visible_part, "*".repeat(masked_length.min(8)))
    }
}

/// Security constants
pub mod constants {
    /// Maximum password length
    pub const MAX_PASSWORD_LENGTH: usize = 128;

    /// Minimum password length
    pub const MIN_PASSWORD_LENGTH: usize = 8;

    /// Session token length
    pub const SESSION_TOKEN_LENGTH: usize = 64;

    /// API key length
    pub const API_KEY_LENGTH: usize = 32;

    /// Maximum login attempts before lockout
    pub const MAX_LOGIN_ATTEMPTS: u32 = 5;

    /// Account lockout duration in minutes
    pub const LOCKOUT_DURATION_MINUTES: u64 = 30;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConfigLoader;

    #[tokio::test]
    async fn test_security_service_creation() {
        let config = ConfigLoader::new().without_env().create_default_config();
        let result = SecurityService::new(&config);

        assert!(result.is_ok());
    }

    #[test]
    fn test_secure_token_generation() {
        let token1 = utils::generate_secure_token(32);
        let token2 = utils::generate_secure_token(32);

        assert_eq!(token1.len(), 32);
        assert_eq!(token2.len(), 32);
        assert_ne!(token1, token2); // Should be different
    }

    #[test]
    fn test_data_masking() {
        let sensitive = "my-secret-key-12345";
        let masked = utils::mask_sensitive_data(sensitive, 3);

        assert!(masked.starts_with("my-"));
        assert!(masked.contains("..."));
        assert!(masked.contains("*"));
    }

    #[test]
    fn test_input_sanitization() {
        let input = "test@example.com<script>alert('xss')</script>";
        let sanitized = utils::sanitize_for_logging(input);

        assert!(!sanitized.contains('<'));
        assert!(!sanitized.contains('>'));
        assert!(sanitized.contains("test@example.com"));
    }
}