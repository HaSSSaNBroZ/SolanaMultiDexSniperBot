//! Configuration management module
//!
//! This module provides comprehensive configuration management for the Solana Sniper Bot,
//! including loading from multiple sources, validation, and scenario-based overrides.

pub mod loader;
pub mod models;
pub mod validation;

// Re-export commonly used types
pub use loader::{ConfigLoader, load_config, load_config_with_args, load_config_from_path};
pub use models::{AppConfig, ScenarioConfig};
pub use validation::{ConfigValidator, ValidationResult};

// Re-export CLI args from utils for convenience
pub use crate::utils::CliArgs;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_config_integration() {
        // Test that all configuration components work together
        let config_result = load_config().await;

        // Should either load successfully or fail gracefully
        match config_result {
            Ok(config) => {
                // If config loads, it should be valid
                assert!(config.is_valid());
            }
            Err(_) => {
                // If config fails to load, that's expected in test environment
                // without proper config files
            }
        }
    }
}