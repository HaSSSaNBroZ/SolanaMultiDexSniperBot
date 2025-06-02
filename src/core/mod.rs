//! Core domain layer containing business entities, value objects, and domain rules
//!
//! This module defines the fundamental building blocks of the Solana Sniper Bot domain.
//! It contains error types, result definitions, common types, and domain rules that
//! are used throughout the application.
//!
//! # Architecture
//!
//! The core module follows Domain-Driven Design (DDD) principles:
//!
//! - **Entities**: Objects with identity (TradeId, SessionId, etc.)
//! - **Value Objects**: Immutable objects without identity (TokenAddress, Timestamp, etc.)
//! - **Domain Services**: Business logic that doesn't belong to a specific entity
//! - **Repository Interfaces**: Contracts for data persistence
//!
//! # Design Principles
//!
//! 1. **Independence**: Core domain should not depend on external frameworks
//! 2. **Immutability**: Value objects should be immutable where possible
//! 3. **Type Safety**: Use strong types to prevent invalid states
//! 4. **Domain Rules**: Encode business rules in the type system

pub mod error;
pub mod result;
pub mod types;

// Re-export commonly used types
pub use error::{AppError, ErrorContext, ErrorKind};
pub use result::AppResult;
pub use types::*;

/// Domain constants and business rules
pub mod domain {
    use std::time::Duration;
    use rust_decimal::Decimal;

    /// Trading domain rules and constants
    pub mod trading {
        use super::*;

        /// Maximum allowed slippage for any trade
        pub const MAX_SLIPPAGE: Decimal = rust_decimal_macros::dec!(50.0);

        /// Minimum trade amount in SOL to prevent dust trades
        pub const MIN_TRADE_AMOUNT: Decimal = rust_decimal_macros::dec!(0.001);

        /// Maximum trade amount in SOL for safety
        pub const MAX_TRADE_AMOUNT: Decimal = rust_decimal_macros::dec!(1000.0);

        /// Default trade timeout
        pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

        /// Maximum number of retries for failed trades
        pub const MAX_RETRIES: u32 = 3;
    }

    /// Risk management domain rules
    pub mod risk {
        /// Risk scores range from 1 (lowest risk) to 10 (highest risk)
        pub const MIN_RISK_SCORE: u8 = 1;
        pub const MAX_RISK_SCORE: u8 = 10;

        /// Default risk threshold above which trades are rejected
        pub const DEFAULT_RISK_THRESHOLD: u8 = 7;

        /// Honeypot confidence threshold (0.0 to 1.0)
        pub const HONEYPOT_THRESHOLD: f64 = 0.8;
    }

    /// Performance and timing domain rules
    pub mod performance {
        use super::*;

        /// Maximum acceptable token detection latency
        pub const MAX_DETECTION_LATENCY: Duration = Duration::from_millis(1000);

        /// Maximum acceptable trade execution latency
        pub const MAX_EXECUTION_LATENCY: Duration = Duration::from_millis(50);

        /// Health check interval
        pub const HEALTH_CHECK_INTERVAL: Duration = Duration::from_secs(30);

        /// Metrics collection interval
        pub const METRICS_INTERVAL: Duration = Duration::from_secs(10);

        /// Connection timeout for external services
        pub const CONNECTION_TIMEOUT: Duration = Duration::from_secs(5);

        /// Request timeout for RPC calls
        pub const RPC_TIMEOUT: Duration = Duration::from_secs(3);
    }

    /// Security domain rules
    pub mod security {
        use super::*;

        /// Session timeout duration
        pub const SESSION_TIMEOUT: Duration = Duration::from_secs(3600); // 1 hour

        /// Maximum failed login attempts before lockout
        pub const MAX_LOGIN_ATTEMPTS: u32 = 3;

        /// Lockout duration after max failed attempts
        pub const LOCKOUT_DURATION: Duration = Duration::from_secs(300); // 5 minutes

        /// Encryption key size in bytes (AES-256)
        pub const ENCRYPTION_KEY_SIZE: usize = 32;

        /// Salt size for password hashing
        pub const SALT_SIZE: usize = 16;
    }
}

/// Domain validation rules and helpers
pub mod validation {
    use super::types::*;
    use anyhow::{anyhow, Result};

    /// Validate a Solana address format
    pub fn validate_solana_address(address: &str) -> Result<()> {
        if address.len() < 32 || address.len() > 44 {
            return Err(anyhow!("Invalid Solana address length: {}", address.len()));
        }

        // Basic base58 validation
        if !address.chars().all(|c| {
            matches!(c, '1'..='9' | 'A'..='H' | 'J'..='N' | 'P'..='Z' | 'a'..='k' | 'm'..='z')
        }) {
            return Err(anyhow!("Invalid base58 characters in address"));
        }

        Ok(())
    }

    /// Validate token amount is within acceptable range
    pub fn validate_trade_amount(amount: rust_decimal::Decimal) -> Result<()> {
        use super::domain::trading::*;

        if amount < MIN_TRADE_AMOUNT {
            return Err(anyhow!("Trade amount too small: {} < {}", amount, MIN_TRADE_AMOUNT));
        }

        if amount > MAX_TRADE_AMOUNT {
            return Err(anyhow!("Trade amount too large: {} > {}", amount, MAX_TRADE_AMOUNT));
        }

        Ok(())
    }

    /// Validate risk score is within valid range
    pub fn validate_risk_score(score: u8) -> Result<()> {
        use super::domain::risk::*;

        if score < MIN_RISK_SCORE || score > MAX_RISK_SCORE {
            return Err(anyhow!(
               "Risk score out of range: {} (valid range: {}-{})",
               score, MIN_RISK_SCORE, MAX_RISK_SCORE
           ));
        }

        Ok(())
    }

    /// Validate slippage percentage
    pub fn validate_slippage(slippage: rust_decimal::Decimal) -> Result<()> {
        use super::domain::trading::MAX_SLIPPAGE;

        if slippage < rust_decimal::Decimal::ZERO {
            return Err(anyhow!("Slippage cannot be negative: {}", slippage));
        }

        if slippage > MAX_SLIPPAGE {
            return Err(anyhow!("Slippage too high: {} > {}", slippage, MAX_SLIPPAGE));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_constants() {
        assert!(domain::trading::MAX_SLIPPAGE > rust_decimal::Decimal::ZERO);
        assert!(domain::trading::MIN_TRADE_AMOUNT > rust_decimal::Decimal::ZERO);
        assert!(domain::trading::MAX_TRADE_AMOUNT > domain::trading::MIN_TRADE_AMOUNT);
        assert!(domain::risk::MAX_RISK_SCORE > domain::risk::MIN_RISK_SCORE);
    }

    #[test]
    fn test_validation_functions() {
        // Test valid Solana address
        assert!(validation::validate_solana_address("11111111111111111111111111111112").is_ok());

        // Test invalid address
        assert!(validation::validate_solana_address("invalid").is_err());

        // Test valid trade amount
        assert!(validation::validate_trade_amount(rust_decimal_macros::dec!(1.0)).is_ok());

        // Test invalid trade amounts
        assert!(validation::validate_trade_amount(rust_decimal_macros::dec!(0.0)).is_err());
        assert!(validation::validate_trade_amount(rust_decimal_macros::dec!(10000.0)).is_err());

        // Test valid risk score
        assert!(validation::validate_risk_score(5).is_ok());

        // Test invalid risk scores
        assert!(validation::validate_risk_score(0).is_err());
        assert!(validation::validate_risk_score(11).is_err());
    }
}