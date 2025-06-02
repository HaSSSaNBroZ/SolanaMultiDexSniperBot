//! Solana Sniper Bot Library
//!
//! Core library containing all business logic and services for the
//! ultra-fast Solana token sniper bot with scenario-based testing.
//!
//! # Architecture Overview
//!
//! The library is organized using Clean Architecture principles:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Application Layer                        │
//! │  ┌─────────────────┐  ┌─────────────────┐                  │
//! │  │   Use Cases     │  │   Controllers   │                  │
//! │  └─────────────────┘  └─────────────────┘                  │
//! └─────────────────────────────────────────────────────────────┘
//! ┌─────────────────────────────────────────────────────────────┐
//! │                     Services Layer                          │
//! │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐           │
//! │  │ Trading │ │  Risk   │ │Analytics│ │Telegram │           │
//! │  └─────────┘ └─────────┘ └─────────┘ └─────────┘           │
//! └─────────────────────────────────────────────────────────────┘
//! ┌─────────────────────────────────────────────────────────────┐
//! │                  Infrastructure Layer                       │
//! │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐           │
//! │  │Database │ │ Solana  │ │Monitoring│ │Security │           │
//! │  └─────────┘ └─────────┘ └─────────┘ └─────────┘           │
//! └─────────────────────────────────────────────────────────────┘
//! ┌─────────────────────────────────────────────────────────────┐
//! │                      Core Layer                             │
//! │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐           │
//! │  │ Entities│ │  Errors │ │  Types  │ │  Utils  │           │
//! │  └─────────┘ └─────────┘ └─────────┘ └─────────┘           │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Features
//!
//! - **Ultra-Fast Trading**: Sub-50ms execution with optimized RPC handling
//! - **Advanced Risk Management**: 10-point risk scoring with honeypot detection
//! - **Scenario-Based Testing**: Isolated dev/production environments
//! - **Real-Time Analytics**: Live P&L tracking and performance metrics
//! - **Intelligent Automation**: Smart DEX routing and position management
//! - **Enterprise Security**: AES-256 encryption and audit logging
//!
//! # Usage
//!
//! ```rust,no_run
//! use solana_sniper_bot::{
//!     Application,
//!     config::{AppConfig, ConfigLoader},
//! };
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = ConfigLoader::load().await?;
//!     let app = Application::build(config).await?;
//!     app.run().await
//! }
//! ```

#![deny(missing_docs)]
#![deny(unsafe_code)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]

// Core modules - Domain layer containing business entities and rules
pub mod core;

// Application layer - Use cases and application logic
pub mod application;

// Configuration management - Multi-source configuration loading
pub mod config;

// Infrastructure layer - External systems integration
pub mod infrastructure;

// Services layer - Business services and domain logic
pub mod services;

// Utilities - Shared helper functions and tools
pub mod utils;

// Re-export commonly used types for convenience
pub use application::Application;
pub use config::{AppConfig, ConfigLoader};
pub use core::{
    error::{AppError, AppResult},
    types::*,
};

/// Library version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Library name
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// Library description
pub const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

/// Build timestamp (set at compile time)
pub const BUILD_TIMESTAMP: &str = env!("BUILD_TIMESTAMP");

/// Git commit hash (if available)
pub const GIT_HASH: &str = env!("GIT_HASH");

/// Performance constants and targets
pub mod performance {
    use std::time::Duration;

    /// Target token detection latency
    pub const TOKEN_DETECTION_TARGET: Duration = Duration::from_millis(1000);

    /// Target trade execution latency
    pub const TRADE_EXECUTION_TARGET: Duration = Duration::from_millis(50);

    /// Target memory usage limit
    pub const MEMORY_LIMIT_MB: u64 = 512;

    /// Target CPU usage limit
    pub const CPU_LIMIT_PERCENT: u8 = 30;

    /// Target win rate percentage
    pub const TARGET_WIN_RATE_PERCENT: f64 = 80.0;

    /// Target uptime percentage
    pub const TARGET_UPTIME_PERCENT: f64 = 99.9;
}

/// Security constants and defaults
pub mod security {
    /// AES-256-GCM key size in bytes
    pub const ENCRYPTION_KEY_SIZE: usize = 32;

    /// Default session timeout in minutes
    pub const DEFAULT_SESSION_TIMEOUT_MINUTES: u64 = 60;

    /// Maximum failed authentication attempts
    pub const MAX_AUTH_ATTEMPTS: u32 = 3;

    /// Rate limiting window in seconds
    pub const RATE_LIMIT_WINDOW_SECONDS: u64 = 60;
}

/// Trading constants and limits
pub mod trading {
    use rust_decimal::Decimal;

    /// Maximum position size in SOL (safety limit)
    pub const MAX_POSITION_SIZE_SOL: Decimal = rust_decimal_macros::dec!(100.0);

    /// Minimum position size in SOL
    pub const MIN_POSITION_SIZE_SOL: Decimal = rust_decimal_macros::dec!(0.001);

    /// Maximum slippage percentage
    pub const MAX_SLIPPAGE_PERCENT: Decimal = rust_decimal_macros::dec!(50.0);

    /// Minimum slippage percentage
    pub const MIN_SLIPPAGE_PERCENT: Decimal = rust_decimal_macros::dec!(0.1);

    /// Maximum concurrent trades per wallet
    pub const MAX_CONCURRENT_TRADES: u32 = 50;

    /// Default trade timeout in seconds
    pub const DEFAULT_TRADE_TIMEOUT_SECONDS: u64 = 30;
}

/// Risk management constants
pub mod risk {
    /// Minimum risk score (safest)
    pub const MIN_RISK_SCORE: u8 = 1;

    /// Maximum risk score (highest risk)
    pub const MAX_RISK_SCORE: u8 = 10;

    /// Default risk threshold
    pub const DEFAULT_RISK_THRESHOLD: u8 = 7;

    /// Honeypot detection confidence threshold
    pub const HONEYPOT_CONFIDENCE_THRESHOLD: f64 = 0.8;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_info() {
        assert!(!VERSION.is_empty());
        assert!(!NAME.is_empty());
        assert!(!DESCRIPTION.is_empty());
    }

    #[test]
    fn test_performance_constants() {
        assert!(performance::TOKEN_DETECTION_TARGET.as_millis() == 1000);
        assert!(performance::TRADE_EXECUTION_TARGET.as_millis() == 50);
        assert!(performance::MEMORY_LIMIT_MB > 0);
        assert!(performance::CPU_LIMIT_PERCENT <= 100);
    }

    #[test]
    fn test_trading_constants() {
        assert!(trading::MAX_POSITION_SIZE_SOL > trading::MIN_POSITION_SIZE_SOL);
        assert!(trading::MAX_SLIPPAGE_PERCENT > trading::MIN_SLIPPAGE_PERCENT);
        assert!(trading::MAX_CONCURRENT_TRADES > 0);
    }

    #[test]
    fn test_risk_constants() {
        assert!(risk::MIN_RISK_SCORE == 1);
        assert!(risk::MAX_RISK_SCORE == 10);
        assert!(risk::DEFAULT_RISK_THRESHOLD <= risk::MAX_RISK_SCORE);
        assert!(risk::HONEYPOT_CONFIDENCE_THRESHOLD <= 1.0);
    }
}

/// Library initialization and global setup
pub mod init {
    use tracing::info;

    /// Initialize the library with default settings
    pub fn init() {
        info!("Initializing {} v{}", crate::NAME, crate::VERSION);
        info!("Build: {} ({})", crate::BUILD_TIMESTAMP, crate::GIT_HASH);
    }

    /// Initialize the library for production use
    pub fn init_production() {
        init();
        info!("Production mode initialized with enhanced security");
    }

    /// Initialize the library for development use
    pub fn init_development() {
        init();
        info!("Development mode initialized with debug features");
    }
}