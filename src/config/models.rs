//! Configuration data structures and models
//!
//! This module defines the complete configuration structure for the Solana Sniper Bot,
//! including all subsystem configurations, validation rules, and serialization logic.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;
use rust_decimal::Decimal;
use chrono::{DateTime, Utc};
use crate::core::types::{ScenarioMode, DexType};

/// Main application configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Environment configuration
    pub environment: EnvironmentConfig,

    /// Solana blockchain configuration
    pub solana: SolanaConfig,

    /// Helius API configuration
    pub helius: HeliusConfig,

    /// Birdeye API configuration
    pub birdeye: BirdeyeConfig,

    /// Database configuration
    pub database: DatabaseConfig,

    /// Redis cache configuration
    pub redis: RedisConfig,

    /// Telegram bot configuration
    pub telegram: TelegramConfig,

    /// Trading parameters
    pub trading: TradingConfig,

    /// Risk management configuration
    pub risk: RiskConfig,

    /// Token scanning configuration
    pub scanner: ScannerConfig,

    /// Analytics configuration
    pub analytics: AnalyticsConfig,

    /// Monitoring configuration
    pub monitoring: MonitoringConfig,

    /// Security configuration
    pub security: SecurityConfig,

    /// Performance configuration
    pub performance: PerformanceConfig,

    /// Scenario-specific overrides
    #[serde(default)]
    pub scenario: Option<ScenarioConfig>,
}

/// Environment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentConfig {
    /// Environment name (development, staging, production)
    pub name: String,

    /// Logging level (trace, debug, info, warn, error)
    pub log_level: String,

    /// Log format (json, pretty, compact)
    pub log_format: String,

    /// Enable debug mode
    #[serde(default)]
    pub debug_mode: bool,
}

/// Solana blockchain configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolanaConfig {
    /// Primary RPC endpoint URL
    pub rpc_url: String,

    /// Fallback RPC endpoints
    #[serde(default)]
    pub fallback_rpc_urls: Vec<String>,

    /// Connection timeout in milliseconds
    #[serde(default = "default_connection_timeout")]
    pub connection_timeout_ms: u64,

    /// Maximum retry attempts
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// Retry backoff in milliseconds
    #[serde(default = "default_retry_backoff")]
    pub retry_backoff_ms: u64,

    /// Commitment level (processed, confirmed, finalized)
    #[serde(default = "default_commitment")]
    pub commitment: String,
}

/// Helius API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeliusConfig {
    /// Helius API key
    pub api_key: String,

    /// Helius base URL
    pub base_url: String,

    /// Webhook URL for real-time events
    #[serde(default)]
    pub webhook_url: Option<String>,

    /// Enable webhook functionality
    #[serde(default)]
    pub enable_webhooks: bool,

    /// Rate limit per second
    #[serde(default = "default_helius_rate_limit")]
    pub rate_limit_per_second: u32,
}

/// Birdeye API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BirdeyeConfig {
    /// Birdeye API key
    pub api_key: String,

    /// Birdeye base URL
    pub base_url: String,

    /// Rate limit per minute
    #[serde(default = "default_birdeye_rate_limit")]
    pub rate_limit_per_minute: u32,

    /// Cache TTL in seconds
    #[serde(default = "default_cache_ttl")]
    pub cache_ttl_seconds: u64,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database connection URL
    pub url: String,

    /// Maximum connections in pool
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,

    /// Minimum connections in pool
    #[serde(default = "default_min_connections")]
    pub min_connections: u32,

    /// Connection timeout in milliseconds
    #[serde(default = "default_connection_timeout")]
    pub connection_timeout_ms: u64,

    /// Idle timeout in milliseconds
    #[serde(default = "default_idle_timeout")]
    pub idle_timeout_ms: u64,

    /// Enable automatic migrations
    #[serde(default = "default_auto_migrate")]
    pub auto_migrate: bool,

    /// Migration path
    #[serde(default = "default_migration_path")]
    pub migration_path: String,

    /// Database schema name (optional)
    #[serde(default)]
    pub schema: Option<String>,

    /// Enable query logging
    #[serde(default)]
    pub enable_query_logging: bool,

    /// Log slow queries
    #[serde(default)]
    pub log_slow_queries: bool,

    /// Slow query threshold in milliseconds
    #[serde(default = "default_slow_query_threshold")]
    pub slow_query_threshold_ms: u64,
}

/// Redis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    /// Redis connection URL
    pub url: String,

    /// Maximum connections in pool
    #[serde(default = "default_redis_max_connections")]
    pub max_connections: u32,

    /// Connection timeout in milliseconds
    #[serde(default = "default_redis_connection_timeout")]
    pub connection_timeout_ms: u64,

    /// Command timeout in milliseconds
    #[serde(default = "default_redis_command_timeout")]
    pub command_timeout_ms: u64,

    /// Default TTL in seconds
    #[serde(default = "default_redis_ttl")]
    pub default_ttl_seconds: u64,

    /// Key prefix
    #[serde(default = "default_redis_key_prefix")]
    pub key_prefix: String,

    /// Enable debug logging
    #[serde(default)]
    pub enable_debug_logging: bool,

    /// Enable clustering
    #[serde(default)]
    pub enable_clustering: bool,

    /// Enable persistence
    #[serde(default)]
    pub enable_persistence: bool,

    /// Persistence interval in seconds
    #[serde(default = "default_persistence_interval")]
    pub persistence_interval_seconds: u64,
}

/// Telegram bot configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    /// Telegram bot token
    pub bot_token: String,

    /// Webhook URL (optional)
    #[serde(default)]
    pub webhook_url: Option<String>,

    /// Admin chat ID
    pub admin_chat_id: i64,

    /// Allowed user IDs
    #[serde(default)]
    pub allowed_user_ids: Vec<i64>,

    /// Command timeout in milliseconds
    #[serde(default = "default_command_timeout")]
    pub command_timeout_ms: u64,

    /// Maximum message length
    #[serde(default = "default_max_message_length")]
    pub max_message_length: usize,

    /// Enable inline keyboards
    #[serde(default = "default_enable_inline_keyboards")]
    pub enable_inline_keyboards: bool,
}

/// Trading configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingConfig {
    /// Scenario mode
    pub scenario_mode: ScenarioMode,

    /// Maximum position size in SOL
    pub max_position_size_sol: Decimal,

    /// Default slippage percentage
    pub default_slippage_percent: Decimal,

    /// Stop loss percentage
    pub stop_loss_percent: Decimal,

    /// Take profit percentage
    pub take_profit_percent: Decimal,

    /// Maximum concurrent trades
    pub max_concurrent_trades: u32,

    /// Trade execution timeout in milliseconds
    #[serde(default = "default_trade_execution_timeout")]
    pub trade_execution_timeout_ms: u64,

    /// Enable automatic trading
    #[serde(default)]
    pub enable_auto_trading: bool,

    /// Enable automatic selling
    #[serde(default)]
    pub enable_auto_selling: bool,

    /// Enable simulation mode
    #[serde(default)]
    pub simulation_mode: bool,

    /// Virtual capital for simulation
    #[serde(default)]
    pub virtual_capital_sol: Option<Decimal>,

    /// Enable circuit breaker
    #[serde(default)]
    pub enable_circuit_breaker: bool,

    /// Circuit breaker loss percentage
    #[serde(default)]
    pub circuit_breaker_loss_percent: Option<Decimal>,

    /// Enable position limits
    #[serde(default)]
    pub enable_position_limits: bool,

    /// Daily trade limit
    #[serde(default)]
    pub daily_trade_limit: Option<u32>,

    /// Preferred DEX ordering
    #[serde(default)]
    pub preferred_dex_order: Vec<DexType>,
}

/// Risk management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskConfig {
    /// Risk score threshold (1-10)
    pub risk_score_threshold: u8,

    /// Maximum daily loss percentage
    pub max_daily_loss_percent: Decimal,

    /// Maximum drawdown percentage
    pub max_drawdown_percent: Decimal,

    /// Emergency stop loss percentage
    pub emergency_stop_loss_percent: Decimal,

    /// Enable honeypot detection
    #[serde(default = "default_true")]
    pub enable_honeypot_detection: bool,

    /// Enable holder analysis
    #[serde(default = "default_true")]
    pub enable_holder_analysis: bool,

    /// Enable liquidity checks
    #[serde(default = "default_true")]
    pub enable_liquidity_checks: bool,

    /// Minimum liquidity in SOL
    pub min_liquidity_sol: Decimal,

    /// Minimum holder count
    pub min_holder_count: u32,

    /// Maximum token age in seconds
    pub max_token_age_seconds: u64,

    /// Enable smart contract verification
    #[serde(default)]
    pub enable_smart_contract_verification: bool,

    /// Enable rug pull detection
    #[serde(default)]
    pub enable_rug_pull_detection: bool,

    /// Enable whale activity monitoring
    #[serde(default)]
    pub enable_whale_activity_monitoring: bool,

    /// Whale threshold percentage
    #[serde(default)]
    pub whale_threshold_percent: Option<Decimal>,
}

/// Scanner configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannerConfig {
    /// Enable real-time scanning
    #[serde(default = "default_true")]
    pub enable_real_time_scanning: bool,

    /// Scan interval in milliseconds
    #[serde(default = "default_scan_interval")]
    pub scan_interval_ms: u64,

    /// Maximum tokens per scan
    #[serde(default = "default_max_tokens_per_scan")]
    pub max_tokens_per_scan: u32,

    /// Minimum market cap in USD
    #[serde(default)]
    pub min_market_cap_usd: Option<u64>,

    /// Maximum market cap in USD
    #[serde(default)]
    pub max_market_cap_usd: Option<u64>,

    /// Blacklisted token addresses
    #[serde(default)]
    pub blacklisted_tokens: Vec<String>,

    /// Blacklisted developer addresses
    #[serde(default)]
    pub blacklisted_developers: Vec<String>,

    /// Whitelisted token addresses (for testing)
    #[serde(default)]
    pub whitelisted_tokens: Vec<String>,

    /// Enable contract verification
    #[serde(default)]
    pub enable_contract_verification: bool,

    /// Require social links
    #[serde(default)]
    pub require_social_links: bool,

    /// Minimum 24h trading volume
    #[serde(default)]
    pub min_trading_volume_24h: Option<u64>,
}
/// Analytics configuration (continued)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsConfig {
    /// Enable metrics collection
    #[serde(default = "default_true")]
    pub enable_metrics: bool,

    /// Metrics server port
    #[serde(default = "default_metrics_port")]
    pub metrics_port: u16,

    /// Enable performance tracking
    #[serde(default = "default_true")]
    pub enable_performance_tracking: bool,

    /// Trade history retention in days
    #[serde(default = "default_retention_days")]
    pub trade_history_retention_days: u32,

    /// Daily report time (HH:MM format)
    #[serde(default)]
    pub daily_report_time: Option<String>,

    /// Weekly report day
    #[serde(default)]
    pub weekly_report_day: Option<String>,

    /// Enable detailed logging
    #[serde(default)]
    pub enable_detailed_logging: bool,

    /// Enable debug reports
    #[serde(default)]
    pub enable_debug_reports: bool,

    /// Report frequency in minutes
    #[serde(default)]
    pub report_frequency_minutes: Option<u32>,

    /// Enable daily reports
    #[serde(default)]
    pub enable_daily_reports: bool,

    /// Enable weekly reports
    #[serde(default)]
    pub enable_weekly_reports: bool,

    /// Enable monthly reports
    #[serde(default)]
    pub enable_monthly_reports: bool,

    /// Report delivery method
    #[serde(default)]
    pub report_delivery_method: Option<String>,

    /// Enable audit reports
    #[serde(default)]
    pub enable_audit_reports: bool,

    /// Audit report frequency
    #[serde(default)]
    pub audit_report_frequency: Option<String>,
}

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Enable health checks
    #[serde(default = "default_true")]
    pub enable_health_checks: bool,

    /// Health check server port
    #[serde(default = "default_health_port")]
    pub health_check_port: u16,

    /// Health check interval in seconds
    #[serde(default = "default_health_interval")]
    pub health_check_interval_seconds: u64,

    /// Enable Telegram alerts
    #[serde(default = "default_true")]
    pub enable_telegram_alerts: bool,

    /// Alert on errors
    #[serde(default = "default_true")]
    pub alert_on_errors: bool,

    /// Alert on trade failures
    #[serde(default = "default_true")]
    pub alert_on_trade_failures: bool,

    /// Alert on risk events
    #[serde(default)]
    pub alert_on_risk_events: bool,

    /// Alert on performance degradation
    #[serde(default)]
    pub alert_on_performance_degradation: bool,

    /// Error threshold per minute
    #[serde(default = "default_error_threshold")]
    pub error_threshold_per_minute: u32,

    /// Enable uptime monitoring
    #[serde(default)]
    pub enable_uptime_monitoring: bool,

    /// Enable performance monitoring
    #[serde(default)]
    pub enable_performance_monitoring: bool,

    /// Performance alert threshold in milliseconds
    #[serde(default)]
    pub performance_alert_threshold_ms: Option<u64>,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Session timeout in minutes
    #[serde(default = "default_session_timeout")]
    pub session_timeout_minutes: u64,

    /// Maximum failed login attempts
    #[serde(default = "default_max_failed_attempts")]
    pub max_failed_attempts: u32,

    /// Lockout duration in minutes
    #[serde(default = "default_lockout_duration")]
    pub lockout_duration_minutes: u64,

    /// Encryption key (base64 encoded)
    pub encryption_key: String,

    /// Enable audit logging
    #[serde(default = "default_true")]
    pub enable_audit_logging: bool,

    /// Audit log retention in days
    #[serde(default = "default_audit_retention")]
    pub audit_log_retention_days: u32,

    /// Enable two-factor authentication
    #[serde(default)]
    pub enable_2fa: bool,

    /// Enable IP whitelisting
    #[serde(default)]
    pub enable_ip_whitelisting: bool,

    /// Allowed IP ranges
    #[serde(default)]
    pub allowed_ip_ranges: Vec<String>,

    /// Enable rate limiting
    #[serde(default)]
    pub enable_rate_limiting: bool,

    /// Rate limit per minute
    #[serde(default)]
    pub rate_limit_per_minute: Option<u32>,

    /// Encryption algorithm
    #[serde(default = "default_encryption_algorithm")]
    pub encryption_algorithm: String,

    /// Key rotation in days
    #[serde(default)]
    pub key_rotation_days: Option<u32>,

    /// Enable debug mode (development only)
    #[serde(default)]
    pub enable_debug_mode: bool,

    /// Allow unsafe operations (development only)
    #[serde(default)]
    pub allow_unsafe_operations: bool,

    /// Log sensitive data (never enable in production)
    #[serde(default)]
    pub log_sensitive_data: bool,
}

/// Performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Maximum memory usage in MB
    #[serde(default = "default_max_memory")]
    pub max_memory_mb: u64,

    /// Maximum CPU usage percentage
    #[serde(default = "default_max_cpu")]
    pub max_cpu_percent: u8,

    /// Target detection latency in milliseconds
    #[serde(default = "default_detection_latency")]
    pub target_detection_latency_ms: u64,

    /// Target execution latency in milliseconds
    #[serde(default = "default_execution_latency")]
    pub target_execution_latency_ms: u64,

    /// Connection pool size
    #[serde(default = "default_connection_pool_size")]
    pub connection_pool_size: u32,

    /// Worker thread count
    #[serde(default = "default_worker_threads")]
    pub worker_thread_count: u32,

    /// Enable CPU affinity
    #[serde(default)]
    pub enable_cpu_affinity: bool,

    /// Enable memory optimizations
    #[serde(default)]
    pub enable_memory_optimizations: bool,

    /// Garbage collection tuning
    #[serde(default)]
    pub garbage_collection_tuning: Option<String>,
}

/// Scenario-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioConfig {
    /// Scenario name
    pub name: String,

    /// Scenario description
    pub description: String,

    /// Scenario version
    pub version: String,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Scenario-specific overrides
    #[serde(default)]
    pub overrides: HashMap<String, serde_yaml::Value>,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level
    pub level: String,

    /// Log format
    pub format: String,

    /// Enable file logging
    #[serde(default)]
    pub enable_file_logging: bool,

    /// Log directory
    #[serde(default)]
    pub log_directory: Option<String>,

    /// Maximum file size in MB
    #[serde(default)]
    pub max_file_size_mb: Option<u64>,

    /// Maximum number of log files
    #[serde(default)]
    pub max_files: Option<u32>,

    /// Enable console colors
    #[serde(default)]
    pub enable_console_colors: bool,

    /// Enable log rotation
    #[serde(default)]
    pub enable_log_rotation: bool,

    /// Rotation frequency
    #[serde(default)]
    pub rotation_frequency: Option<String>,

    /// Enable log compression
    #[serde(default)]
    pub enable_log_compression: bool,

    /// Enable remote logging
    #[serde(default)]
    pub enable_remote_logging: bool,
}

/// External services configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalServicesConfig {
    /// Datadog configuration
    #[serde(default)]
    pub datadog: Option<DatadogConfig>,

    /// PagerDuty configuration
    #[serde(default)]
    pub pagerduty: Option<PagerDutyConfig>,

    /// AWS S3 configuration
    #[serde(default)]
    pub aws_s3: Option<AwsS3Config>,
}

/// Datadog monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatadogConfig {
    /// Enable Datadog
    pub enabled: bool,

    /// Datadog API key
    pub api_key: String,
}

/// PagerDuty alerting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PagerDutyConfig {
    /// Enable PagerDuty
    pub enabled: bool,

    /// Integration key
    pub integration_key: String,
}

/// AWS S3 backup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwsS3Config {
    /// Enable S3
    pub enabled: bool,

    /// S3 bucket name
    pub bucket: String,

    /// AWS region
    pub region: String,
}

// Default value functions
fn default_connection_timeout() -> u64 { 5000 }
fn default_max_retries() -> u32 { 3 }
fn default_retry_backoff() -> u64 { 1000 }
fn default_commitment() -> String { "confirmed".to_string() }
fn default_helius_rate_limit() -> u32 { 100 }
fn default_birdeye_rate_limit() -> u32 { 500 }
fn default_cache_ttl() -> u64 { 60 }
fn default_max_connections() -> u32 { 20 }
fn default_min_connections() -> u32 { 5 }
fn default_idle_timeout() -> u64 { 300000 }
fn default_auto_migrate() -> bool { true }
fn default_migration_path() -> String { "migrations".to_string() }
fn default_slow_query_threshold() -> u64 { 1000 }
fn default_redis_max_connections() -> u32 { 20 }
fn default_redis_connection_timeout() -> u64 { 3000 }
fn default_redis_command_timeout() -> u64 { 1000 }
fn default_redis_ttl() -> u64 { 3600 }
fn default_redis_key_prefix() -> String { "sniper:".to_string() }
fn default_persistence_interval() -> u64 { 300 }
fn default_command_timeout() -> u64 { 30000 }
fn default_max_message_length() -> usize { 4096 }
fn default_enable_inline_keyboards() -> bool { true }
fn default_trade_execution_timeout() -> u64 { 50 }
fn default_scan_interval() -> u64 { 1000 }
fn default_max_tokens_per_scan() -> u32 { 100 }
fn default_metrics_port() -> u16 { 9090 }
fn default_retention_days() -> u32 { 90 }
fn default_health_port() -> u16 { 8080 }
fn default_health_interval() -> u64 { 30 }
fn default_error_threshold() -> u32 { 10 }
fn default_session_timeout() -> u64 { 60 }
fn default_max_failed_attempts() -> u32 { 3 }
fn default_lockout_duration() -> u64 { 5 }
fn default_audit_retention() -> u32 { 365 }
fn default_encryption_algorithm() -> String { "AES-256-GCM".to_string() }
fn default_max_memory() -> u64 { 512 }
fn default_max_cpu() -> u8 { 30 }
fn default_detection_latency() -> u64 { 1000 }
fn default_execution_latency() -> u64 { 50 }
fn default_connection_pool_size() -> u32 { 20 }
fn default_worker_threads() -> u32 { 4 }
fn default_true() -> bool { true }

impl AppConfig {
    /// Check if running in development mode
    pub fn is_development(&self) -> bool {
        matches!(self.trading.scenario_mode, ScenarioMode::Development)
    }

    /// Check if running in production mode
    pub fn is_production(&self) -> bool {
        matches!(self.trading.scenario_mode, ScenarioMode::Production)
    }

    /// Check if running in simulation mode
    pub fn is_simulation(&self) -> bool {
        matches!(self.trading.scenario_mode, ScenarioMode::Simulation) || self.trading.simulation_mode
    }

    /// Get the current environment name
    pub fn environment(&self) -> &str {
        &self.environment.name
    }

    /// Apply scenario-specific overrides
    pub fn apply_scenario_overrides(&mut self, scenario: ScenarioConfig) -> crate::core::result::AppResult<()> {
        self.scenario = Some(scenario.clone());

        // Apply overrides from scenario configuration
        for (key, value) in scenario.overrides {
            self.apply_override(&key, value)?;
        }

        Ok(())
    }

    /// Apply a single configuration override
    fn apply_override(&mut self, key: &str, value: serde_yaml::Value) -> crate::core::result::AppResult<()> {
        // Parse the key path (e.g., "trading.max_position_size_sol")
        let parts: Vec<&str> = key.split('.').collect();

        match parts.as_slice() {
            ["trading", field] => self.apply_trading_override(field, value)?,
            ["risk", field] => self.apply_risk_override(field, value)?,
            ["scanner", field] => self.apply_scanner_override(field, value)?,
            ["monitoring", field] => self.apply_monitoring_override(field, value)?,
            ["analytics", field] => self.apply_analytics_override(field, value)?,
            ["security", field] => self.apply_security_override(field, value)?,
            ["performance", field] => self.apply_performance_override(field, value)?,
            _ => {
                tracing::warn!("Unknown configuration override key: {}", key);
            }
        }

        Ok(())
    }

    fn apply_trading_override(&mut self, field: &str, value: serde_yaml::Value) -> crate::core::result::AppResult<()> {
        match field {
            "max_position_size_sol" => {
                if let Some(val) = value.as_f64() {
                    self.trading.max_position_size_sol = Decimal::try_from(val)
                        .map_err(|e| crate::core::error::AppError::config(format!("Invalid decimal value: {}", e)))?;
                }
            }
            "default_slippage_percent" => {
                if let Some(val) = value.as_f64() {
                    self.trading.default_slippage_percent = Decimal::try_from(val)
                        .map_err(|e| crate::core::error::AppError::config(format!("Invalid decimal value: {}", e)))?;
                }
            }
            "enable_auto_trading" => {
                if let Some(val) = value.as_bool() {
                    self.trading.enable_auto_trading = val;
                }
            }
            "enable_auto_selling" => {
                if let Some(val) = value.as_bool() {
                    self.trading.enable_auto_selling = val;
                }
            }
            "max_concurrent_trades" => {
                if let Some(val) = value.as_u64() {
                    self.trading.max_concurrent_trades = val as u32;
                }
            }
            _ => {
                tracing::warn!("Unknown trading configuration field: {}", field);
            }
        }
        Ok(())
    }

    fn apply_risk_override(&mut self, field: &str, value: serde_yaml::Value) -> crate::core::result::AppResult<()> {
        match field {
            "risk_score_threshold" => {
                if let Some(val) = value.as_u64() {
                    self.risk.risk_score_threshold = val as u8;
                }
            }
            "max_daily_loss_percent" => {
                if let Some(val) = value.as_f64() {
                    self.risk.max_daily_loss_percent = Decimal::try_from(val)
                        .map_err(|e| crate::core::error::AppError::config(format!("Invalid decimal value: {}", e)))?;
                }
            }
            "enable_honeypot_detection" => {
                if let Some(val) = value.as_bool() {
                    self.risk.enable_honeypot_detection = val;
                }
            }
            "enable_holder_analysis" => {
                if let Some(val) = value.as_bool() {
                    self.risk.enable_holder_analysis = val;
                }
            }
            _ => {
                tracing::warn!("Unknown risk configuration field: {}", field);
            }
        }
        Ok(())
    }

    fn apply_scanner_override(&mut self, field: &str, value: serde_yaml::Value) -> crate::core::result::AppResult<()> {
        match field {
            "scan_interval_ms" => {
                if let Some(val) = value.as_u64() {
                    self.scanner.scan_interval_ms = val;
                }
            }
            "max_tokens_per_scan" => {
                if let Some(val) = value.as_u64() {
                    self.scanner.max_tokens_per_scan = val as u32;
                }
            }
            "enable_real_time_scanning" => {
                if let Some(val) = value.as_bool() {
                    self.scanner.enable_real_time_scanning = val;
                }
            }
            _ => {
                tracing::warn!("Unknown scanner configuration field: {}", field);
            }
        }
        Ok(())
    }

    fn apply_monitoring_override(&mut self, field: &str, value: serde_yaml::Value) -> crate::core::result::AppResult<()> {
        match field {
            "health_check_port" => {
                if let Some(val) = value.as_u64() {
                    self.monitoring.health_check_port = val as u16;
                }
            }
            "enable_telegram_alerts" => {
                if let Some(val) = value.as_bool() {
                    self.monitoring.enable_telegram_alerts = val;
                }
            }
            _ => {
                tracing::warn!("Unknown monitoring configuration field: {}", field);
            }
        }
        Ok(())
    }

    fn apply_analytics_override(&mut self, field: &str, value: serde_yaml::Value) -> crate::core::result::AppResult<()> {
        match field {
            "metrics_port" => {
                if let Some(val) = value.as_u64() {
                    self.analytics.metrics_port = val as u16;
                }
            }
            "enable_detailed_logging" => {
                if let Some(val) = value.as_bool() {
                    self.analytics.enable_detailed_logging = val;
                }
            }
            _ => {
                tracing::warn!("Unknown analytics configuration field: {}", field);
            }
        }
        Ok(())
    }

    fn apply_security_override(&mut self, field: &str, value: serde_yaml::Value) -> crate::core::result::AppResult<()> {
        match field {
            "session_timeout_minutes" => {
                if let Some(val) = value.as_u64() {
                    self.security.session_timeout_minutes = val;
                }
            }
            "enable_debug_mode" => {
                if let Some(val) = value.as_bool() {
                    self.security.enable_debug_mode = val;
                }
            }
            _ => {
                tracing::warn!("Unknown security configuration field: {}", field);
            }
        }
        Ok(())
    }

    fn apply_performance_override(&mut self, field: &str, value: serde_yaml::Value) -> crate::core::result::AppResult<()> {
        match field {
            "max_memory_mb" => {
                if let Some(val) = value.as_u64() {
                    self.performance.max_memory_mb = val;
                }
            }
            "target_detection_latency_ms" => {
                if let Some(val) = value.as_u64() {
                    self.performance.target_detection_latency_ms = val;
                }
            }
            _ => {
                tracing::warn!("Unknown performance configuration field: {}", field);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_config_mode_detection() {
        let mut config = AppConfig {
            environment: EnvironmentConfig {
                name: "test".to_string(),
                log_level: "debug".to_string(),
                log_format: "pretty".to_string(),
                debug_mode: true,
            },
            trading: TradingConfig {
                scenario_mode: ScenarioMode::Development,
                max_position_size_sol: Decimal::ONE,
                default_slippage_percent: rust_decimal_macros::dec!(3.0),
                stop_loss_percent: rust_decimal_macros::dec!(20.0),
                take_profit_percent: rust_decimal_macros::dec!(100.0),
                max_concurrent_trades: 5,
                trade_execution_timeout_ms: 50,
                enable_auto_trading: false,
                enable_auto_selling: false,
                simulation_mode: false,
                virtual_capital_sol: None,
                enable_circuit_breaker: false,
                circuit_breaker_loss_percent: None,
                enable_position_limits: false,
                daily_trade_limit: None,
                preferred_dex_order: vec![],
            },
            // ... other required fields with default values
            solana: SolanaConfig {
                rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
                fallback_rpc_urls: vec![],
                connection_timeout_ms: 5000,
                max_retries: 3,
                retry_backoff_ms: 1000,
                commitment: "confirmed".to_string(),
            },
            helius: HeliusConfig {
                api_key: "test".to_string(),
                base_url: "https://mainnet.helius-rpc.com".to_string(),
                webhook_url: None,
                enable_webhooks: false,
                rate_limit_per_second: 100,
            },
            birdeye: BirdeyeConfig {
                api_key: "test".to_string(),
                base_url: "https://public-api.birdeye.so".to_string(),
                rate_limit_per_minute: 500,
                cache_ttl_seconds: 60,
            },
            database: DatabaseConfig {
                url: "postgres://test".to_string(),
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
            redis: RedisConfig {
                url: "redis://localhost".to_string(),
                max_connections: 20,
                connection_timeout_ms: 3000,
                command_timeout_ms: 1000,
                default_ttl_seconds: 3600,
                key_prefix: "test:".to_string(),
                enable_debug_logging: false,
                enable_clustering: false,
                enable_persistence: false,
                persistence_interval_seconds: 300,
            },
            telegram: TelegramConfig {
                bot_token: "test".to_string(),
                webhook_url: None,
                admin_chat_id: 123456789,
                allowed_user_ids: vec![],
                command_timeout_ms: 30000,
                max_message_length: 4096,
                enable_inline_keyboards: true,
            },
            risk: RiskConfig {
                risk_score_threshold: 7,
                max_daily_loss_percent: rust_decimal_macros::dec!(10.0),
                max_drawdown_percent: rust_decimal_macros::dec!(20.0),
                emergency_stop_loss_percent: rust_decimal_macros::dec!(50.0),
                enable_honeypot_detection: true,
                enable_holder_analysis: true,
                enable_liquidity_checks: true,
                min_liquidity_sol: Decimal::ONE,
                min_holder_count: 10,
                max_token_age_seconds: 300,
                enable_smart_contract_verification: false,
                enable_rug_pull_detection: false,
                enable_whale_activity_monitoring: false,
                whale_threshold_percent: None,
            },
            scanner: ScannerConfig {
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
            analytics: AnalyticsConfig {
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
            monitoring: MonitoringConfig {
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
            security: SecurityConfig {
                session_timeout_minutes: 60,
                max_failed_attempts: 3,
                lockout_duration_minutes: 5,
                encryption_key: "test_key".to_string(),
                enable_audit_logging: true,
                audit_log_retention_days: 365,
                enable_2fa: false,
                enable_ip_whitelisting: false,
                allowed_ip_ranges: vec![],
                enable_rate_limiting: false,
                rate_limit_per_minute: None,
                encryption_algorithm: "AES-256-GCM".to_string(),
                key_rotation_days: None,
                enable_debug_mode: false,
                allow_unsafe_operations: false,
                log_sensitive_data: false,
            },
            performance: PerformanceConfig {
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
        };

        assert!(config.is_development());
        assert!(!config.is_production());

        config.trading.scenario_mode = ScenarioMode::Production;
        assert!(!config.is_development());
        assert!(config.is_production());
    }

    #[test]
    fn test_scenario_override_application() {
        // This test would be more comprehensive with actual override values
        let config = AppConfig {
            // ... minimal config for testing
            environment: EnvironmentConfig {
                name: "test".to_string(),
                log_level: "debug".to_string(),
                log_format: "pretty".to_string(),
                debug_mode: true,
            },
            // ... other fields with default test values
        };

        // Test would verify that scenario overrides are applied correctly
        assert_eq!(config.environment(), "test");
    }
}