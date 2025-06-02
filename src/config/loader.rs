//! Configuration loader with multi-source support
//!
//! This module provides a flexible configuration loader that can load and merge
//! configuration from multiple sources: environment variables, TOML files, YAML scenario files,
//! and command-line arguments.

use anyhow::{anyhow, Context, Result};
use config::{Config, ConfigBuilder, Environment, File, FileFormat};
use serde::de::DeserializeOwned;
use std::path::{Path, PathBuf};
use std::env;
use tracing::{debug, info, warn};

use crate::core::result::AppResult;
use crate::core::error::AppError;
use crate::utils::CliArgs;
use super::models::{AppConfig, ScenarioConfig};

/// Configuration loader with support for multiple sources
#[derive(Debug, Clone)]
pub struct ConfigLoader {
    /// Base configuration path
    config_path: Option<PathBuf>,

    /// CLI arguments
    cli_args: Option<CliArgs>,

    /// Environment prefix for variables
    env_prefix: String,

    /// Enable environment variable loading
    enable_env: bool,

    /// Additional configuration files
    additional_files: Vec<PathBuf>,
}

impl ConfigLoader {
    /// Create a new configuration loader
    pub fn new() -> Self {
        Self {
            config_path: None,
            cli_args: None,
            env_prefix: "SNIPER".to_string(),
            enable_env: true,
            additional_files: Vec::new(),
        }
    }

    /// Set the base configuration path
    pub fn with_config_path<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.config_path = Some(path.as_ref().to_path_buf());
        self
    }

    /// Set CLI arguments
    pub fn with_cli_args(mut self, args: CliArgs) -> Self {
        self.cli_args = Some(args);
        self
    }

    /// Set environment variable prefix
    pub fn with_env_prefix<S: Into<String>>(mut self, prefix: S) -> Self {
        self.env_prefix = prefix.into();
        self
    }

    /// Disable environment variable loading
    pub fn without_env(mut self) -> Self {
        self.enable_env = false;
        self
    }

    /// Add an additional configuration file
    pub fn with_additional_file<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.additional_files.push(path.as_ref().to_path_buf());
        self
    }

    /// Load and build the complete application configuration
    pub async fn load(self) -> AppResult<AppConfig> {
        info!("🔧 Starting configuration loading process");

        // Step 1: Load base configuration from TOML
        let mut config = self.load_base_config()
            .await
            .context("Failed to load base configuration")?;

        // Step 2: Apply environment variable overrides
        if self.enable_env {
            self.apply_environment_overrides(&mut config)
                .context("Failed to apply environment overrides")?;
        }

        // Step 3: Apply CLI argument overrides
        if let Some(ref cli_args) = self.cli_args {
            self.apply_cli_overrides(&mut config, cli_args)
                .context("Failed to apply CLI overrides")?;
        }

        // Step 4: Load and apply scenario configuration
        let scenario_mode = &config.trading.scenario_mode;
        if let Some(scenario_config) = self.load_scenario_config(scenario_mode).await? {
            config.apply_scenario_overrides(scenario_config)
                .context("Failed to apply scenario overrides")?;
        }

        // Step 5: Apply additional configuration files
        for file_path in &self.additional_files {
            self.apply_additional_config(&mut config, file_path)
                .await
                .with_context(|| format!("Failed to apply additional config from {:?}", file_path))?;
        }

        info!("✅ Configuration loading completed successfully");
        debug!("📊 Final configuration: scenario_mode={}, environment={}",
              config.trading.scenario_mode, config.environment.name);

        Ok(config)
    }

    /// Load base configuration from TOML file
    async fn load_base_config(&self) -> Result<AppConfig> {
        let config_path = self.resolve_config_path();

        info!("📄 Loading base configuration from: {}", config_path.display());

        if !config_path.exists() {
            warn!("⚠️  Configuration file not found: {}", config_path.display());
            warn!("⚠️  Using default configuration values");
            return Ok(self.create_default_config());
        }

        let config_content = tokio::fs::read_to_string(&config_path)
            .await
            .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;

        let config: AppConfig = toml::from_str(&config_content)
            .with_context(|| format!("Failed to parse TOML config: {}", config_path.display()))?;

        debug!("✅ Base configuration loaded successfully");
        Ok(config)
    }

    /// Resolve the configuration file path
    fn resolve_config_path(&self) -> PathBuf {
        if let Some(ref path) = self.config_path {
            return path.clone();
        }

        // Check CLI args for config path
        if let Some(ref cli_args) = self.cli_args {
            if let Some(ref path) = cli_args.config_path {
                return PathBuf::from(path);
            }
        }

        // Check environment variable
        if let Ok(path) = env::var("CONFIG_PATH") {
            return PathBuf::from(path);
        }

        // Default paths to check
        let default_paths = [
            "configs/config.toml",
            "config.toml",
            "./config.toml",
            "/etc/sniper-bot/config.toml",
        ];

        for path in &default_paths {
            let pb = PathBuf::from(path);
            if pb.exists() {
                debug!("📍 Found config file at: {}", pb.display());
                return pb;
            }
        }

        // Return default path even if it doesn't exist
        PathBuf::from("configs/config.toml")
    }

    /// Apply environment variable overrides
    fn apply_environment_overrides(&self, config: &mut AppConfig) -> Result<()> {
        debug!("🌍 Applying environment variable overrides");

        // Create a config builder for environment variables
        let env_config = Config::builder()
            .add_source(
                Environment::with_prefix(&self.env_prefix)
                    .prefix_separator("_")
                    .separator("__")
                    .try_parsing(true)
                    .ignore_empty(true)
            )
            .build()
            .context("Failed to build environment configuration")?;

        // Apply specific environment variable mappings
        self.apply_env_var(config, &env_config, "SOLANA_RPC_URL", |cfg, val: String| {
            cfg.solana.rpc_url = val;
        })?;

        self.apply_env_var(config, &env_config, "HELIUS_API_KEY", |cfg, val: String| {
            cfg.helius.api_key = val;
        })?;

        self.apply_env_var(config, &env_config, "BIRDEYE_API_KEY", |cfg, val: String| {
            cfg.birdeye.api_key = val;
        })?;

        self.apply_env_var(config, &env_config, "DATABASE_URL", |cfg, val: String| {
            cfg.database.url = val;
        })?;

        self.apply_env_var(config, &env_config, "REDIS_URL", |cfg, val: String| {
            cfg.redis.url = val;
        })?;

        self.apply_env_var(config, &env_config, "TELEGRAM_BOT_TOKEN", |cfg, val: String| {
            cfg.telegram.bot_token = val;
        })?;

        self.apply_env_var(config, &env_config, "TELEGRAM_ADMIN_CHAT_ID", |cfg, val: i64| {
            cfg.telegram.admin_chat_id = val;
        })?;

        self.apply_env_var(config, &env_config, "ENCRYPTION_KEY", |cfg, val: String| {
            cfg.security.encryption_key = val;
        })?;

        self.apply_env_var(config, &env_config, "SCENARIO_MODE", |cfg, val: String| {
            if let Ok(mode) = val.parse() {
                cfg.trading.scenario_mode = mode;
            }
        })?;

        self.apply_env_var(config, &env_config, "LOG_LEVEL", |cfg, val: String| {
            cfg.environment.log_level = val;
        })?;

        self.apply_env_var(config, &env_config, "LOG_FORMAT", |cfg, val: String| {
            cfg.environment.log_format = val;
        })?;

        // Apply allowed user IDs from environment
        if let Ok(user_ids) = env::var("ALLOWED_USER_IDS") {
            let ids: Vec<i64> = user_ids
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();
            if !ids.is_empty() {
                config.telegram.allowed_user_ids = ids;
            }
        }

        debug!("✅ Environment variable overrides applied");
        Ok(())
    }

    /// Apply a single environment variable with type conversion
    fn apply_env_var<T, F>(&self, config: &mut AppConfig, env_config: &Config, key: &str, applier: F) -> Result<()>
    where
        T: DeserializeOwned,
        F: FnOnce(&mut AppConfig, T),
    {
        if let Ok(value) = env_config.get::<T>(key) {
            applier(config, value);
            debug!("🔄 Applied environment override: {}", key);
        }
        Ok(())
    }

    /// Apply CLI argument overrides
    fn apply_cli_overrides(&self, config: &mut AppConfig, cli_args: &CliArgs) -> Result<()> {
        debug!("⌨️  Applying CLI argument overrides");

        // Override environment name
        if let Some(ref env_name) = cli_args.environment {
            config.environment.name = env_name.clone();
        }

        // Override log level
        config.environment.log_level = cli_args.log_level.clone();

        // Override log format
        config.environment.log_format = cli_args.log_format.clone();

        // Override metrics settings
        if cli_args.enable_metrics {
            config.analytics.enable_metrics = true;
        }

        if cli_args.metrics_port != 9090 {
            config.analytics.metrics_port = cli_args.metrics_port;
        }

        // Override health check settings
        if cli_args.enable_health_checks {
            config.monitoring.enable_health_checks = true;
        }

        if cli_args.health_port != 8080 {
            config.monitoring.health_check_port = cli_args.health_port;
        }

        debug!("✅ CLI argument overrides applied");
        Ok(())
    }

    /// Load scenario-specific configuration
    async fn load_scenario_config(&self, scenario_mode: &crate::core::types::ScenarioMode) -> AppResult<Option<ScenarioConfig>> {
        let scenario_name = match scenario_mode {
            crate::core::types::ScenarioMode::Development => "dev",
            crate::core::types::ScenarioMode::Production => "production",
            crate::core::types::ScenarioMode::Simulation => "simulation",
        };

        let scenario_paths = [
            format!("configs/scenarios/{}.yaml", scenario_name),
            format!("scenarios/{}.yaml", scenario_name),
            format!("./{}.yaml", scenario_name),
        ];

        for path_str in &scenario_paths {
            let path = PathBuf::from(path_str);
            if path.exists() {
                info!("📋 Loading scenario configuration: {}", path.display());
                return Ok(Some(self.load_scenario_from_path(&path).await?));
            }
        }

        warn!("⚠️  No scenario configuration found for mode: {}", scenario_mode);
        Ok(None)
    }

    /// Load scenario configuration from a specific path
    async fn load_scenario_from_path(&self, path: &Path) -> AppResult<ScenarioConfig> {
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| AppError::config(format!("Failed to read scenario file {}: {}", path.display(), e)))?;

        let scenario: ScenarioConfig = serde_yaml::from_str(&content)
            .map_err(|e| AppError::config(format!("Failed to parse scenario YAML {}: {}", path.display(), e)))?;

        debug!("✅ Scenario configuration loaded: {}", scenario.name);
        Ok(scenario)
    }

    /// Apply additional configuration file
    async fn apply_additional_config(&self, config: &mut AppConfig, file_path: &Path) -> Result<()> {
        if !file_path.exists() {
            warn!("⚠️  Additional config file not found: {}", file_path.display());
            return Ok(());
        }

        info!("📄 Applying additional configuration: {}", file_path.display());

        let file_extension = file_path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");

        match file_extension {
            "toml" => {
                let content = tokio::fs::read_to_string(file_path).await?;
                let additional_config: AppConfig = toml::from_str(&content)
                    .context("Failed to parse additional TOML config")?;
                self.merge_configs(config, additional_config);
            }
            "yaml" | "yml" => {
                let content = tokio::fs::read_to_string(file_path).await?;
                let scenario: ScenarioConfig = serde_yaml::from_str(&content)
                    .context("Failed to parse additional YAML config")?;
                config.apply_scenario_overrides(scenario)
                    .map_err(|e| anyhow!("Failed to apply scenario overrides: {}", e))?;
            }
            _ => {
                warn!("⚠️  Unsupported config file format: {}", file_extension);
            }
        }

        debug!("✅ Additional configuration applied");
        Ok(())
    }

    /// Merge two configurations (additional config overrides base config)
    fn merge_configs(&self, base: &mut AppConfig, additional: AppConfig) {
        // For simplicity, we'll override entire sections
        // In a more sophisticated implementation, we could do field-by-field merging

        // Only override non-empty/non-default values
        if additional.solana.rpc_url != "https://api.mainnet-beta.solana.com" {
            base.solana = additional.solana;
        }

        if !additional.helius.api_key.is_empty() {
            base.helius = additional.helius;
        }

        if !additional.birdeye.api_key.is_empty() {
            base.birdeye = additional.birdeye;
        }

        // Continue for other sections as needed...
    }

    /// Create default configuration when no config file is found
    pub fn create_default_config(&self) -> AppConfig {
        warn!("🏗️  Creating default configuration");

        AppConfig {
            environment: super::models::EnvironmentConfig {
                name: "development".to_string(),
                log_level: "info".to_string(),
                log_format: "pretty".to_string(),
                debug_mode: true,
            },
            solana: super::models::SolanaConfig {
                rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
                fallback_rpc_urls: vec![],
                connection_timeout_ms: 5000,
                max_retries: 3,
                retry_backoff_ms: 1000,
                commitment: "confirmed".to_string(),
            },
            helius: super::models::HeliusConfig {
                api_key: String::new(),
                base_url: "https://mainnet.helius-rpc.com".to_string(),
                webhook_url: None,
                enable_webhooks: false,
                rate_limit_per_second: 100,
            },
            birdeye: super::models::BirdeyeConfig {
                api_key: String::new(),
                base_url: "https://public-api.birdeye.so".to_string(),
                rate_limit_per_minute: 500,
                cache_ttl_seconds: 60,
            },
            database: super::models::DatabaseConfig {
                url: String::new(),
                max_connections: 20,
                min_connections: 5,
                connection_timeout_ms: 5000,
                idle_timeout_ms: 300000,
                auto_migrate: true,
                migration_path: "migrations".to_string(),
                schema: None,
                enable_query_logging: false,
                log_slow_queries: false,
                slow_query_threshold_ms: 1000,
            },
            redis: super::models::RedisConfig {
                url: String::new(),
                max_connections: 20,
                connection_timeout_ms: 3000,
                command_timeout_ms: 1000,
                default_ttl_seconds: 3600,
                key_prefix: "sniper:".to_string(),
                enable_debug_logging: false,
                enable_clustering: false,
                enable_persistence: false,
                persistence_interval_seconds: 300,
            },
            telegram: super::models::TelegramConfig {
                bot_token: String::new(),
                webhook_url: None,
                admin_chat_id: 0,
                allowed_user_ids: vec![],
                command_timeout_ms: 30000,
                max_message_length: 4096,
                enable_inline_keyboards: true,
            },
            trading: super::models::TradingConfig {
                scenario_mode: crate::core::types::ScenarioMode::Development,
                max_position_size_sol: rust_decimal_macros::dec!(1.0),
                default_slippage_percent: rust_decimal_macros::dec!(3.0),
                stop_loss_percent: rust_decimal_macros::dec!(20.0),
                take_profit_percent: rust_decimal_macros::dec!(100.0),
                max_concurrent_trades: 5,
                trade_execution_timeout_ms: 50,
                enable_auto_trading: false,
                enable_auto_selling: false,
                simulation_mode: true,
                virtual_capital_sol: Some(rust_decimal_macros::dec!(10.0)),
                enable_circuit_breaker: false,
                circuit_breaker_loss_percent: None,
                enable_position_limits: false,
                daily_trade_limit: None,
                preferred_dex_order: vec![],
            },
            risk: super::models::RiskConfig {
                risk_score_threshold: 7,
                max_daily_loss_percent: rust_decimal_macros::dec!(10.0),
                max_drawdown_percent: rust_decimal_macros::dec!(20.0),
                emergency_stop_loss_percent: rust_decimal_macros::dec!(50.0),
                enable_honeypot_detection: true,
                enable_holder_analysis: true,
                enable_liquidity_checks: true,
                min_liquidity_sol: rust_decimal_macros::dec!(1.0),
                min_holder_count: 10,
                max_token_age_seconds: 300,
                enable_smart_contract_verification: false,
                enable_rug_pull_detection: false,
                enable_whale_activity_monitoring: false,
                whale_threshold_percent: None,
            },
            scanner: super::models::ScannerConfig {
                enable_real_time_scanning: true,
                scan_interval_ms: 1000,
                max_tokens_per_scan: 100,
                min_market_cap_usd: None,
                max_market_cap_usd: None,
                blacklisted_tokens: vec![],
                blacklisted_developers: vec![],
                whitelisted_tokens: vec![],
                enable_contract_verification: false,
                require_social_links: false,
                min_trading_volume_24h: None,
            },
            analytics: super::models::AnalyticsConfig {
                enable_metrics: true,
                metrics_port: 9090,
                enable_performance_tracking: true,
                trade_history_retention_days: 90,
                daily_report_time: None,
                weekly_report_day: None,
                enable_detailed_logging: false,
                enable_debug_reports: false,
                report_frequency_minutes: None,
                enable_daily_reports: false,
                enable_weekly_reports: false,
                enable_monthly_reports: false,
                report_delivery_method: None,
                enable_audit_reports: false,
                audit_report_frequency: None,
            },
            monitoring: super::models::MonitoringConfig {
                enable_health_checks: true,
                health_check_port: 8080,
                health_check_interval_seconds: 30,
                enable_telegram_alerts: true,
                alert_on_errors: true,
                alert_on_trade_failures: true,
                alert_on_risk_events: false,
                alert_on_performance_degradation: false,
                error_threshold_per_minute: 10,
                enable_uptime_monitoring: false,
                enable_performance_monitoring: false,
                performance_alert_threshold_ms: None,
            },
            security: super::models::SecurityConfig {
                session_timeout_minutes: 60,
                max_failed_attempts: 3,
                lockout_duration_minutes: 5,
                encryption_key: String::new(),
                enable_audit_logging: true,
                audit_log_retention_days: 365,
                enable_2fa: false,
                enable_ip_whitelisting: false,
                allowed_ip_ranges: vec![],
                enable_rate_limiting: false,
                rate_limit_per_minute: None,
                encryption_algorithm: "AES-256-GCM".to_string(),
                key_rotation_days: None,
                enable_debug_mode: true,
                allow_unsafe_operations: false,
                log_sensitive_data: false,
            },
            performance: super::models::PerformanceConfig {
                max_memory_mb: 512,
                max_cpu_percent: 30,
                target_detection_latency_ms: 1000,
                target_execution_latency_ms: 50,
                connection_pool_size: 20,
                worker_thread_count: 4,
                enable_cpu_affinity: false,
                enable_memory_optimizations: false,
                garbage_collection_tuning: None,
            },
            scenario: None,
        }
    }
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to load configuration with default settings
pub async fn load_config() -> AppResult<AppConfig> {
    ConfigLoader::new().load().await
}

/// Load configuration with CLI arguments
pub async fn load_config_with_args(cli_args: CliArgs) -> AppResult<AppConfig> {
    ConfigLoader::new()
        .with_cli_args(cli_args)
        .load()
        .await
}

/// Load configuration from a specific path
pub async fn load_config_from_path<P: AsRef<Path>>(path: P) -> AppResult<AppConfig> {
    ConfigLoader::new()
        .with_config_path(path)
        .load()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio_test;

    #[tokio::test]
    async fn test_load_default_config() {
        let loader = ConfigLoader::new().without_env();
        let config = loader.load().await;

        assert!(config.is_ok());
        let config = config.unwrap();
        assert_eq!(config.environment.name, "development");
        assert!(config.is_development());
    }

    #[tokio::test]
    async fn test_config_path_resolution() {
        let loader = ConfigLoader::new();
        let path = loader.resolve_config_path();

        // Should default to configs/config.toml
        assert!(path.to_string_lossy().contains("config.toml"));
    }

    #[tokio::test]
    async fn test_cli_overrides() {
        let cli_args = CliArgs {
            config_path: None,
            log_level: "debug".to_string(),
            log_format: "json".to_string(),
            environment: Some("test".to_string()),
            enable_metrics: true,
            metrics_port: 9999,
            enable_health_checks: true,
            health_port: 8888,
        };

        let loader = ConfigLoader::new()
            .with_cli_args(cli_args)
            .without_env();

        let config = loader.load().await.unwrap();

        assert_eq!(config.environment.log_level, "debug");
        assert_eq!(config.environment.log_format, "json");
        assert_eq!(config.environment.name, "test");
        assert_eq!(config.analytics.metrics_port, 9999);
        assert_eq!(config.monitoring.health_check_port, 8888);
    }

    #[tokio::test]
    async fn test_environment_variable_override() {
        // Set a test environment variable
        env::set_var("SNIPER_LOG_LEVEL", "trace");

        let loader = ConfigLoader::new();
        let mut config = loader.create_default_config();

        // This would normally be called in the load process
        let result = loader.apply_environment_overrides(&mut config);

        // Clean up
        env::remove_var("SNIPER_LOG_LEVEL");

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_scenario_config_loading() {
        let temp_dir = TempDir::new().unwrap();
        let scenario_path = temp_dir.path().join("test.yaml");

        let scenario_content = r#"
scenario:
 name: "test"
 description: "Test scenario"
 version: "1.0.0"
 created_at: "2024-06-02T10:00:00Z"

overrides:
 trading.max_position_size_sol: 0.5
 risk.risk_score_threshold: 8
"#;

        tokio::fs::write(&scenario_path, scenario_content).await.unwrap();

        let loader = ConfigLoader::new();
        let scenario = loader.load_scenario_from_path(&scenario_path).await;

        assert!(scenario.is_ok());
        let scenario = scenario.unwrap();
        assert_eq!(scenario.name, "test");
        assert_eq!(scenario.description, "Test scenario");
    }
}