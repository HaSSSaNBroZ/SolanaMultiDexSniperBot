//! Application error types and error handling utilities
//!
//! This module defines a comprehensive error system for the Solana Sniper Bot.
//! It provides structured error types, error contexts, and utilities for
//! error propagation and handling throughout the application.

use thiserror::Error;
use std::fmt;
use serde::{Deserialize, Serialize};

/// Main application error type that encompasses all possible errors
#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum AppError {
    /// Configuration-related errors
    #[error("Configuration error: {message}")]
    Config {
        message: String,
        #[source]
        source: Option<Box<AppError>>,
    },

    /// Database operation errors
    #[error("Database error: {message}")]
    Database {
        message: String,
        operation: String,
        #[source]
        source: Option<Box<AppError>>,
    },

    /// Network and RPC communication errors
    #[error("Network error: {message}")]
    Network {
        message: String,
        endpoint: Option<String>,
        retry_count: u32,
        #[source]
        source: Option<Box<AppError>>,
    },

    /// Trading execution errors
    #[error("Trading error: {message}")]
    Trading {
        message: String,
        trade_id: Option<String>,
        token_address: Option<String>,
        #[source]
        source: Option<Box<AppError>>,
    },

    /// Risk management errors
    #[error("Risk management error: {message}")]
    Risk {
        message: String,
        risk_score: Option<u8>,
        rule_name: Option<String>,
        #[source]
        source: Option<Box<AppError>>,
    },

    /// Security and authentication errors
    #[error("Security error: {message}")]
    Security {
        message: String,
        user_id: Option<String>,
        #[source]
        source: Option<Box<AppError>>,
    },

    /// Telegram bot errors
    #[error("Telegram error: {message}")]
    Telegram {
        message: String,
        chat_id: Option<i64>,
        command: Option<String>,
        #[source]
        source: Option<Box<AppError>>,
    },

    /// Solana blockchain errors
    #[error("Solana error: {message}")]
    Solana {
        message: String,
        transaction_signature: Option<String>,
        slot: Option<u64>,
        #[source]
        source: Option<Box<AppError>>,
    },

    /// DEX integration errors
    #[error("DEX error: {message}")]
    Dex {
        message: String,
        dex_name: String,
        pool_address: Option<String>,
        #[source]
        source: Option<Box<AppError>>,
    },

    /// Analytics and metrics errors
    #[error("Analytics error: {message}")]
    Analytics {
        message: String,
        metric_name: Option<String>,
        #[source]
        source: Option<Box<AppError>>,
    },

    /// Validation errors
    #[error("Validation error: {message}")]
    Validation {
        message: String,
        field: Option<String>,
        value: Option<String>,
    },

    /// Internal system errors
    #[error("Internal error: {message}")]
    Internal {
        message: String,
        component: Option<String>,
        #[source]
        source: Option<Box<AppError>>,
    },

    /// Timeout errors
    #[error("Timeout error: {message}")]
    Timeout {
        message: String,
        operation: String,
        duration_ms: u64,
    },

    /// Rate limiting errors
    #[error("Rate limit exceeded: {message}")]
    RateLimit {
        message: String,
        limit: u32,
        window_seconds: u64,
        retry_after_seconds: Option<u64>,
    },

    /// External service errors
    #[error("External service error: {service} - {message}")]
    ExternalService {
        service: String,
        message: String,
        status_code: Option<u16>,
        #[source]
        source: Option<Box<AppError>>,
    },
}

/// Error severity levels for logging and alerting
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorSeverity {
    /// Low impact errors that don't affect core functionality
    Low,
    /// Medium impact errors that may affect some features
    Medium,
    /// High impact errors that affect core functionality
    High,
    /// Critical errors that require immediate attention
    Critical,
}

/// Error context for additional debugging information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    /// Error severity level
    pub severity: ErrorSeverity,
    /// Component where the error occurred
    pub component: String,
    /// User ID associated with the error (if applicable)
    pub user_id: Option<String>,
    /// Session ID associated with the error (if applicable)
    pub session_id: Option<String>,
    /// Additional metadata
    pub metadata: std::collections::HashMap<String, String>,
    /// Timestamp when the error occurred
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Error category for grouping related errors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorKind {
    /// Configuration and setup errors
    Configuration,
    /// Database and persistence errors
    Persistence,
    /// Network and communication errors
    Network,
    /// Business logic errors
    Business,
    /// Security and authentication errors
    Security,
    /// External service integration errors
    Integration,
    /// Performance and timeout errors
    Performance,
    /// Validation and input errors
    Validation,
    /// System and infrastructure errors
    System,
}

impl AppError {
    /// Create a new configuration error
    pub fn config<S: Into<String>>(message: S) -> Self {
        Self::Config {
            message: message.into(),
            source: None,
        }
    }

    /// Create a new database error
    pub fn database<S: Into<String>>(message: S, operation: S) -> Self {
        Self::Database {
            message: message.into(),
            operation: operation.into(),
            source: None,
        }
    }

    /// Create a new network error
    pub fn network<S: Into<String>>(message: S) -> Self {
        Self::Network {
            message: message.into(),
            endpoint: None,
            retry_count: 0,
            source: None,
        }
    }

    /// Create a new trading error
    pub fn trading<S: Into<String>>(message: S) -> Self {
        Self::Trading {
            message: message.into(),
            trade_id: None,
            token_address: None,
            source: None,
        }
    }

    /// Create a new risk management error
    pub fn risk<S: Into<String>>(message: S) -> Self {
        Self::Risk {
            message: message.into(),
            risk_score: None,
            rule_name: None,
            source: None,
        }
    }

    /// Create a new security error
    pub fn security<S: Into<String>>(message: S) -> Self {
        Self::Security {
            message: message.into(),
            user_id: None,
            source: None,
        }
    }

    /// Create a new validation error
    pub fn validation<S: Into<String>>(message: S) -> Self {
        Self::Validation {
            message: message.into(),
            field: None,
            value: None,
        }
    }

    /// Create a new internal error
    pub fn internal<S: Into<String>>(message: S) -> Self {
        Self::Internal {
            message: message.into(),
            component: None,
            source: None,
        }
    }

    /// Create a new timeout error
    pub fn timeout<S: Into<String>>(message: S, operation: S, duration_ms: u64) -> Self {
        Self::Timeout {
            message: message.into(),
            operation: operation.into(),
            duration_ms,
        }
    }

    /// Get the error category
    pub fn kind(&self) -> ErrorKind {
        match self {
            Self::Config { .. } => ErrorKind::Configuration,
            Self::Database { .. } => ErrorKind::Persistence,
            Self::Network { .. } | Self::Solana { .. } => ErrorKind::Network,
            Self::Trading { .. } | Self::Risk { .. } | Self::Analytics { .. } => ErrorKind::Business,
            Self::Security { .. } => ErrorKind::Security,
            Self::Telegram { .. } | Self::Dex { .. } | Self::ExternalService { .. } => ErrorKind::Integration,
            Self::Timeout { .. } | Self::RateLimit { .. } => ErrorKind::Performance,
            Self::Validation { .. } => ErrorKind::Validation,
            Self::Internal { .. } => ErrorKind::System,
        }
    }

    /// Get the error severity
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            Self::Validation { .. } | Self::RateLimit { .. } => ErrorSeverity::Low,
            Self::Network { .. } | Self::Timeout { .. } | Self::Analytics { .. } => ErrorSeverity::Medium,
            Self::Trading { .. } | Self::Risk { .. } | Self::Dex { .. } => ErrorSeverity::High,
            Self::Security { .. } | Self::Database { .. } | Self::Internal { .. } => ErrorSeverity::Critical,
            _ => ErrorSeverity::Medium,
        }
    }

    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Network { .. }
                | Self::Timeout { .. }
                | Self::RateLimit { .. }
                | Self::ExternalService { .. }
        )
    }

    /// Get suggested retry delay in seconds
    pub fn retry_delay_seconds(&self) -> Option<u64> {
        match self {
            Self::Network { retry_count, .. } => {
                // Exponential backoff: 1s, 2s, 4s, 8s, ...
                Some(2_u64.pow(*retry_count).min(60))
            }
            Self::RateLimit { retry_after_seconds, .. } => *retry_after_seconds,
            Self::Timeout { .. } => Some(5),
            Self::ExternalService { .. } => Some(10),
            _ => None,
        }
    }

    /// Add source error
    pub fn with_source(mut self, source: AppError) -> Self {
        match &mut self {
            Self::Config { source: s, .. }
            | Self::Database { source: s, .. }
            | Self::Network { source: s, .. }
            | Self::Trading { source: s, .. }
            | Self::Risk { source: s, .. }
            | Self::Security { source: s, .. }
            | Self::Telegram { source: s, .. }
            | Self::Solana { source: s, .. }
            | Self::Dex { source: s, .. }
            | Self::Analytics { source: s, .. }
            | Self::Internal { source: s, .. }
            | Self::ExternalService { source: s, .. } => {
                *s = Some(Box::new(source));
            }
            _ => {}
        }
        self
    }

    /// Convert to error context for logging
    pub fn to_context(&self, component: &str) -> ErrorContext {
        let mut metadata = std::collections::HashMap::new();

        // Add error-specific metadata
        match self {
            Self::Trading { trade_id, token_address, .. } => {
                if let Some(id) = trade_id {
                    metadata.insert("trade_id".to_string(), id.clone());
                }
                if let Some(addr) = token_address {
                    metadata.insert("token_address".to_string(), addr.clone());
                }
            }
            Self::Network { endpoint, retry_count, .. } => {
                if let Some(ep) = endpoint {
                    metadata.insert("endpoint".to_string(), ep.clone());
                }
                metadata.insert("retry_count".to_string(), retry_count.to_string());
            }
            Self::Solana { transaction_signature, slot, .. } => {
                if let Some(sig) = transaction_signature {
                    metadata.insert("transaction_signature".to_string(), sig.clone());
                }
                if let Some(s) = slot {
                    metadata.insert("slot".to_string(), s.to_string());
                }
            }
            _ => {}
        }

        ErrorContext {
            severity: self.severity(),
            component: component.to_string(),
            user_id: None,
            session_id: None,
            metadata,
            timestamp: chrono::Utc::now(),
        }
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        Self::internal(err.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        Self::validation(format!("JSON serialization error: {}", err))
    }
}

impl From<serde_yaml::Error> for AppError {
    fn from(err: serde_yaml::Error) -> Self {
        Self::validation(format!("YAML parsing error: {}", err))
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        Self::internal(format!("IO error: {}", err))
    }
}

impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self {
        Self::network(format!("HTTP request error: {}", err))
    }
}

impl From<tokio::time::error::Elapsed> for AppError {
    fn from(err: tokio::time::error::Elapsed) -> Self {
        Self::timeout("Operation timed out".to_string(), "unknown".to_string(), 0)
    }
}

/// Result type alias for the application
pub type AppResult<T> = std::result::Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error = AppError::config("Test configuration error");
        assert!(matches!(error, AppError::Config { .. }));
        assert_eq!(error.kind(), ErrorKind::Configuration);
        assert_eq!(error.severity(), ErrorSeverity::Critical);
    }

    #[test]
    fn test_error_with_source() {
        let source = AppError::network("Network failed");
        let error = AppError::trading("Trade failed").with_source(source);

        assert!(matches!(error, AppError::Trading { .. }));
        // Source should be set
        if let AppError::Trading { source, .. } = &error {
            assert!(source.is_some());
        }
    }

    #[test]
    fn test_retry_logic() {
        let network_error = AppError::network("Connection failed");
        assert!(network_error.is_retryable());
        assert!(network_error.retry_delay_seconds().is_some());

        let validation_error = AppError::validation("Invalid input");
        assert!(!validation_error.is_retryable());
        assert!(validation_error.retry_delay_seconds().is_none());
    }

    #[test]
    fn test_error_context() {
        let error = AppError::trading("Test trading error");
        let context = error.to_context("trading_engine");

        assert_eq!(context.component, "trading_engine");
        assert_eq!(context.severity, ErrorSeverity::High);
    }
}