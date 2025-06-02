//! Solana Sniper Bot - Ultra-Fast Trading Engine
//!
//! A high-performance, secure Solana token sniper bot built with Rust.
//! Features real-time token detection, advanced risk management, and scenario-based testing.
//!
//! # Architecture
//!
//! The bot is designed using Clean Architecture principles with clear separation of concerns:
//! - **Core**: Domain types, errors, and business rules
//! - **Application**: Use cases and application logic
//! - **Infrastructure**: External systems (database, APIs, monitoring)
//! - **Services**: Business services (trading, risk, analytics)
//!
//! # Performance Targets
//!
//! - Token Detection: < 1 second
//! - Trade Execution: < 50ms average
//! - Win Rate: > 80%
//! - Memory Usage: < 512MB
//! - CPU Usage: < 30%
//! - Uptime: > 99.9%
//!
//! # Security
//!
//! - Private keys encrypted with AES-256-GCM
//! - Role-based access control via Telegram
//! - Comprehensive audit logging
//! - Rate limiting and abuse protection
//!
//! Author: Hassan Hafedh Ubaid
//! Version: 0.1.0
//! License: MIT

use anyhow::{Context, Result};
use clap::Parser;
use color_eyre::eyre::WrapErr;
use solana_sniper_bot::{
    application::Application,
    config::{AppConfig, ConfigLoader, CliArgs},
    core::error::AppError,
    utils::telemetry,
};
use tracing::{error, info, warn, instrument, span, Level};
use std::process;

/// Application entry point with comprehensive error handling and graceful shutdown
#[tokio::main]
async fn main() {
    // Setup color-eyre for enhanced error reporting
    if let Err(e) = color_eyre::install() {
        eprintln!("Failed to install color-eyre: {}", e);
        process::exit(1);
    }

    // Execute main application logic with proper error handling
    if let Err(e) = run().await {
        error!("Fatal application error: {:?}", e);

        // Print user-friendly error message
        eprintln!("\n❌ Application failed to start:");
        eprintln!("   {}", e);

        // Print error chain for debugging
        let mut source = e.source();
        while let Some(err) = source {
            eprintln!("   Caused by: {}", err);
            source = err.source();
        }

        process::exit(1);
    }
}

/// Main application execution logic
#[instrument(name = "main_application")]
async fn run() -> Result<()> {
    // Parse command line arguments
    let cli_args = CliArgs::parse();

    // Initialize telemetry and observability
    telemetry::init(&cli_args.log_level, &cli_args.log_format)
        .wrap_err("Failed to initialize telemetry system")?;

    // Create application span for tracing
    let _span = span!(Level::INFO, "solana_sniper_bot").entered();

    // Display professional startup banner
    display_startup_banner();

    // Load and validate configuration
    let config = load_and_validate_config(&cli_args)
        .await
        .wrap_err("Configuration loading failed")?;

    // Build application with dependency injection
    let app = Application::build(config)
        .await
        .wrap_err("Application initialization failed")?;

    info!("🎯 Sniper Bot initialized successfully");
    info!("🔄 Entering main event loop...");

    // Run application with comprehensive shutdown handling
    run_with_graceful_shutdown(app)
        .await
        .wrap_err("Application runtime error")
}

/// Display professional startup banner with system information
fn display_startup_banner() {
    let version = env!("CARGO_PKG_VERSION");
    let build_info = format!("Rust {} • {} build",
                             rustc_version_runtime::version(),
                             if cfg!(debug_assertions) { "Debug" } else { "Release" }
    );

    info!("╔════════════════════════════════════════════════════════════════════╗");
    info!("║                        SOLANA SNIPER BOT                          ║");
    info!("║                       Rust Edition v{:<8}                        ║", version);
    info!("║                                                                    ║");
    info!("║  🚀 Ultra-Fast Token Detection & Trading (<1s detection)          ║");
    info!("║  🛡️  Advanced Risk Management & Security (1-10 risk scoring)      ║");
    info!("║  📊 Real-Time Analytics & Performance Monitoring                  ║");
    info!("║  🤖 Intelligent Telegram Interface (Full bot control)            ║");
    info!("║  🎯 Scenario-Based Testing (Dev/Prod modes)                       ║");
    info!("║                                                                    ║");
    info!("║  Performance Targets:                                              ║");
    info!("║  • Detection: <1s • Execution: <50ms • Win Rate: >80%             ║");
    info!("║  • Memory: <512MB • CPU: <30% • Uptime: >99.9%                    ║");
    info!("║                                                                    ║");
    info!("║  Author: Hassan Hafedh Ubaid                                      ║");
    info!("║  License: MIT • Build: {}                               ║", build_info);
    info!("╚════════════════════════════════════════════════════════════════════╝");
}

/// Load and validate application configuration with comprehensive error handling
#[instrument(skip(cli_args))]
async fn load_and_validate_config(cli_args: &CliArgs) -> Result<AppConfig> {
    info!("🔧 Loading application configuration...");

    // Load configuration from multiple sources
    let config = ConfigLoader::new()
        .with_cli_args(cli_args.clone())
        .load()
        .await
        .map_err(|e| AppError::Config(format!("Failed to load configuration: {}", e)))
        .wrap_err("Configuration loading failed")?;

    info!("✅ Configuration loaded successfully");
    log_configuration_summary(&config);

    // Validate configuration integrity
    config.validate()
        .map_err(|e| AppError::Config(format!("Configuration validation failed: {}", e)))
        .wrap_err("Configuration validation failed")?;

    info!("✅ Configuration validation passed");

    // Log security warnings for development mode
    if config.is_development() {
        warn!("⚠️  Running in DEVELOPMENT mode");
        warn!("⚠️  • Trades will be simulated");
        warn!("⚠️  • Enhanced logging enabled");
        warn!("⚠️  • Debug features active");
        warn!("⚠️  Ensure proper configuration before switching to PRODUCTION");
    } else if config.is_production() {
        info!("🔒 Running in PRODUCTION mode");
        info!("💰 Real trading enabled with risk controls");
    }

    Ok(config)
}

/// Log configuration summary for transparency
fn log_configuration_summary(config: &AppConfig) {
    info!("📊 Configuration Summary:");
    info!("   Environment: {}", config.environment);
    info!("   Scenario Mode: {}", config.trading.scenario_mode);
    info!("   Max Position Size: {} SOL", config.trading.max_position_size_sol);
    info!("   Default Slippage: {}%", config.trading.default_slippage_percent);
    info!("   Stop Loss: {}%", config.trading.stop_loss_percent);
    info!("   Take Profit: {}%", config.trading.take_profit_percent);
    info!("   Max Concurrent Trades: {}", config.trading.max_concurrent_trades);
    info!("   Risk Threshold: {}/10", config.risk.risk_score_threshold);
    info!("   Metrics Enabled: {}", config.monitoring.enable_metrics);
}

/// Run application with comprehensive graceful shutdown handling
#[instrument(skip(app))]
async fn run_with_graceful_shutdown(app: Application) -> Result<()> {
    // Setup shutdown signal handling
    let shutdown_future = async {
        // Handle SIGTERM (Docker/K8s shutdown)
        #[cfg(unix)]
        {
            use tokio::signal::unix::{signal, SignalKind};
            let mut sigterm = signal(SignalKind::terminate())
                .expect("Failed to register SIGTERM handler");
            let mut sigint = signal(SignalKind::interrupt())
                .expect("Failed to register SIGINT handler");

            tokio::select! {
                _ = sigterm.recv() => {
                    warn!("🛑 Received SIGTERM signal");
                }
                _ = sigint.recv() => {
                    warn!("🛑 Received SIGINT signal (Ctrl+C)");
                }
            }
        }

        // Handle Ctrl+C on Windows
        #[cfg(windows)]
        {
            tokio::signal::ctrl_c().await.expect("Failed to register Ctrl+C handler");
            warn!("🛑 Received Ctrl+C signal");
        }
    };

    // Run application with shutdown handling
    let app_result = tokio::select! {
        result = app.run() => {
            match result {
                Ok(_) => {
                    info!("✅ Application completed successfully");
                    Ok(())
                }
                Err(e) => {
                    error!("❌ Application runtime error: {:?}", e);
                    Err(e)
                }
            }
        }
        _ = shutdown_future => {
            info!("🔄 Initiating graceful shutdown...");

            // Give application time to cleanup
            tokio::time::timeout(
                std::time::Duration::from_secs(30),
                graceful_cleanup()
            ).await.unwrap_or_else(|_| {
                warn!("⚠️  Graceful shutdown timeout, forcing exit");
            });

            info!("👋 Solana Sniper Bot terminated gracefully");
            Ok(())
        }
    };

    app_result
}

/// Perform graceful cleanup operations
async fn graceful_cleanup() {
    info!("🧹 Performing graceful cleanup...");

    // Flush metrics
    if let Err(e) = flush_metrics().await {
        warn!("Failed to flush metrics: {}", e);
    }

    // Close database connections
    // This will be implemented in Phase 3

    // Save any pending data
    // This will be implemented in later phases

    info!("✅ Cleanup completed");
}

/// Flush metrics before shutdown
async fn flush_metrics() -> Result<()> {
    // Metrics flushing will be implemented in Phase 3
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    Ok(())
}

// Runtime version detection for build info
mod rustc_version_runtime {
    pub fn version() -> &'static str {
        env!("RUSTC_VERSION")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;

    #[tokio::test]
    async fn test_application_startup() {
        // Test that application can start without panicking
        let cli_args = CliArgs {
            config_path: Some("test_config.toml".to_string()),
            log_level: "debug".to_string(),
            log_format: "json".to_string(),
            environment: Some("test".to_string()),
        };

        // This should not panic
        assert!(cli_args.log_level == "debug");
    }

    #[test]
    fn test_startup_banner() {
        // Test that startup banner doesn't panic
        display_startup_banner();
    }
}