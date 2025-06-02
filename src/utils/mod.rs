//! Utility functions and helpers used throughout the application
//!
//! This module provides common utility functions for cryptography, time handling,
//! validation, formatting, and other cross-cutting concerns.

pub mod crypto;
pub mod time;
pub mod validation;

// Re-export commonly used utilities
pub use crypto::*;
pub use time::*;
pub use validation::*;

/// Telemetry and observability utilities
pub mod telemetry {
    use anyhow::Result;
    use tracing_subscriber::{
        fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry,
    };
    use tracing_appender::rolling::{RollingFileAppender, Rotation};

    /// Initialize global tracing with the specified log level and format
    pub fn init(log_level: &str, log_format: &str) -> Result<()> {
        let env_filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new(log_level));

        let registry = Registry::default().with(env_filter);

        match log_format {
            "json" => {
                registry
                    .with(
                        fmt::layer()
                            .json()
                            .with_target(true)
                            .with_thread_ids(true)
                            .with_file(true)
                            .with_line_number(true)
                    )
                    .init();
            }
            "compact" => {
                registry
                    .with(
                        fmt::layer()
                            .compact()
                            .with_target(false)
                            .with_thread_ids(false)
                    )
                    .init();
            }
            _ => {
                // Default pretty format
                registry
                    .with(
                        fmt::layer()
                            .pretty()
                            .with_target(true)
                            .with_thread_ids(true)
                            .with_file(true)
                            .with_line_number(true)
                    )
                    .init();
            }
        }

        Ok(())
    }

    /// Initialize file-based logging with rotation
    pub fn init_with_file_rotation(
        log_level: &str,
        log_format: &str,
        log_directory: &str,
        file_name_prefix: &str,
    ) -> Result<()> {
        let env_filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new(log_level));

        // Create rolling file appender (daily rotation)
        let file_appender = RollingFileAppender::new(
            Rotation::daily(),
            log_directory,
            file_name_prefix,
        );

        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

        let registry = Registry::default().with(env_filter);

        match log_format {
            "json" => {
                registry
                    .with(
                        fmt::layer()
                            .json()
                            .with_writer(non_blocking)
                            .with_target(true)
                            .with_thread_ids(true)
                            .with_file(true)
                            .with_line_number(true)
                    )
                    .with(
                        fmt::layer()
                            .pretty()
                            .with_target(true)
                            .with_thread_ids(true)
                    )
                    .init();
            }
            _ => {
                registry
                    .with(
                        fmt::layer()
                            .with_writer(non_blocking)
                            .with_target(true)
                            .with_thread_ids(true)
                            .with_file(true)
                            .with_line_number(true)
                    )
                    .with(
                        fmt::layer()
                            .pretty()
                            .with_target(true)
                            .with_thread_ids(true)
                    )
                    .init();
            }
        }

        Ok(())
    }
}

/// Configuration argument parsing utilities
pub mod cli {
    use clap::Parser;

    /// Command line arguments for the application
    #[derive(Parser, Debug, Clone)]
    #[command(
        name = "solana-sniper-bot",
        about = "Ultra-fast Solana token sniper bot with advanced risk management",
        version = env!("CARGO_PKG_VERSION"),
        author = "Hassan Hafedh Ubaid"
    )]
    pub struct CliArgs {
        /// Path to configuration file
        #[arg(short, long, env = "CONFIG_PATH")]
        pub config_path: Option<String>,

        /// Logging level (trace, debug, info, warn, error)
        #[arg(short, long, default_value = "info", env = "LOG_LEVEL")]
        pub log_level: String,

        /// Log format (json, pretty, compact)
        #[arg(long, default_value = "pretty", env = "LOG_FORMAT")]
        pub log_format: String,

        /// Environment (development, staging, production)
        #[arg(short, long, env = "ENVIRONMENT")]
        pub environment: Option<String>,

        /// Enable metrics collection
        #[arg(long, env = "ENABLE_METRICS")]
        pub enable_metrics: bool,

        /// Metrics server port
        #[arg(long, default_value = "9090", env = "METRICS_PORT")]
        pub metrics_port: u16,

        /// Enable health checks
        #[arg(long, env = "ENABLE_HEALTH_CHECKS")]
        pub enable_health_checks: bool,

        /// Health check server port
        #[arg(long, default_value = "8080", env = "HEALTH_PORT")]
        pub health_port: u16,
    }
}

// Re-export CLI utilities
pub use cli::CliArgs;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_args_parsing() {
        // Test that CLI args can be parsed
        let args = CliArgs {
            config_path: Some("test.toml".to_string()),
            log_level: "debug".to_string(),
            log_format: "json".to_string(),
            environment: Some("test".to_string()),
            enable_metrics: true,
            metrics_port: 9090,
            enable_health_checks: true,
            health_port: 8080,
        };

        assert_eq!(args.log_level, "debug");
        assert_eq!(args.log_format, "json");
        assert!(args.enable_metrics);
    }
}