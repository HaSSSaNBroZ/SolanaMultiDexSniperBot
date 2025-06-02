//! Configuration validation logic
//!
//! This module provides comprehensive validation for all configuration values
//! to ensure they are within acceptable ranges and formats before the application starts.

use anyhow::{anyhow, Result};
use url::Url;
use std::net::SocketAddr;
use tracing::{debug, warn};

use crate::core::result::AppResult;
use crate::core::error::AppError;
use crate::utils::validation;
use super::models::AppConfig;

/// Configuration validator
pub struct ConfigValidator {
    /// Strict validation mode (fails on warnings)
    strict_mode: bool,

    /// Collect all validation errors instead of failing fast
    collect_all_errors: bool,
}

/// Validation result with warnings and errors
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Fatal validation errors
    pub errors: Vec<String>,

    /// Non-fatal warnings
    pub warnings: Vec<String>,

    /// Validation passed
    pub is_valid: bool,
}

impl ConfigValidator {
    /// Create a new validator with default settings
    pub fn new() -> Self {
        Self {
            strict_mode: false,
            collect_all_errors: true,
        }
    }

    /// Enable strict validation mode
    pub fn with_strict_mode(mut self) -> Self {
        self.strict_mode = true;
        self
    }

    /// Enable fail-fast mode (stop on first error)
    pub fn with_fail_fast(mut self) -> Self {
        self.collect_all_errors = false;
        self
    }

    /// Validate the complete application configuration
    pub fn validate(&self, config: &AppConfig) -> AppResult<ValidationResult> {
        debug!("🔍 Starting configuration validation");

        let mut result = ValidationResult {
            errors: Vec::new(),
            warnings: Vec::new(),
            is_valid: true,
        };

        // Validate each configuration section
        self.validate_environment(&config.environment, &mut result)?;
        self.validate_solana(&config.solana, &mut result)?;
        self.validate_helius(&config.helius, &mut result)?;
        self.validate_birdeye(&config.birdeye, &mut result)?;
        self.validate_database(&config.database, &mut result)?;
        self.validate_redis(&config.redis, &mut result)?;
        self.validate_telegram(&config.telegram, &mut result)?;
        self.validate_trading(&config.trading, &mut result)?;
        self.validate_risk(&config.risk, &mut result)?;
        self.validate_scanner(&config.scanner, &mut result)?;
        self.validate_analytics(&config.analytics, &mut result)?;
        self.validate_monitoring(&config.monitoring, &mut result)?;
        self.validate_security(&config.security, &mut result)?;
        self.validate_performance(&config.performance, &mut result)?;

        // Cross-configuration validation
        self.validate_cross_config(config, &mut result)?;

        // Determine overall validation result
        result.is_valid = result.errors.is_empty() && (!self.strict_mode || result.warnings.is_empty());

        if result.is_valid {
            debug!("✅ Configuration validation passed");
        } else {
            warn!("❌ Configuration validation failed");
            for error in &result.errors {
                warn!("   Error: {}", error);
            }
            for warning in &result.warnings {
                warn!("   Warning: {}", warning);
            }
        }

        Ok(result)
    }

    /// Validate environment configuration
    fn validate_environment(&self, config: &super::models::EnvironmentConfig, result: &mut ValidationResult) -> AppResult<()> {
        // Validate environment name
        validation::config::validate_environment(&config.name)
            .map_err(|e| self.add_error(result, format!("Environment name: {}", e)))?;

        // Validate log level
        validation::config::validate_log_level(&config.log_level)
            .map_err(|e| self.add_error(result, format!("Log level: {}", e)))?;

        // Validate log format
        match config.log_format.as_str() {
            "json" | "pretty" | "compact" => {},
            _ => self.add_error(result, format!("Invalid log format '{}'. Must be 'json', 'pretty', or 'compact'", config.log_format))?,
        }

        // Warnings
        if config.debug_mode && config.name == "production" {
            self.add_warning(result, "Debug mode enabled in production environment");
        }

        Ok(())
    }

    /// Validate Solana configuration
    fn validate_solana(&self, config: &super::models::SolanaConfig, result: &mut ValidationResult) -> AppResult<()> {
        // Validate RPC URL
        validation::validate_url(&config.rpc_url)
            .map_err(|e| self.add_error(result, format!("Solana RPC URL: {}", e)))?;

        // Validate fallback URLs
        for (i, url) in config.fallback_rpc_urls.iter().enumerate() {
            validation::validate_url(url)
                .map_err(|e| self.add_error(result, format!("Fallback RPC URL {}: {}", i, e)))?;
        }

        validation::trading::validate_timeout_ms(config.connection_timeout_ms)
            .map_err(|e| self.add_error(result, format!("Connection timeout: {}", e)))?;

        validation::trading::validate_timeout_ms(config.retry_backoff_ms)
            .map_err(|e| self.add_error(result, format!("Retry backoff: {}", e)))?;

        // Validate max retries
        if config.max_retries > 10 {
            self.add_warning(result, "High retry count may cause delays");
        }

        // Validate commitment level
        match config.commitment.as_str() {
            "processed" | "confirmed" | "finalized" => {},
            _ => self.add_error(result, format!("Invalid commitment level '{}'. Must be 'processed', 'confirmed', or 'finalized'", config.commitment))?,
        }

        Ok(())
    }

    /// Validate Helius configuration
    fn validate_helius(&self, config: &super::models::HeliusConfig, result: &mut ValidationResult) -> AppResult<()> {
        // Validate API key
        if config.api_key.is_empty() {
            self.add_warning(result, "Helius API key is empty - functionality will be limited");
        } else {
            validation::validate_string_length(&config.api_key, Some(10), Some(100), "Helius API key")
                .map_err(|e| self.add_error(result, format!("Helius API key: {}", e)))?;
        }

        // Validate base URL
        validation::validate_url(&config.base_url)
            .map_err(|e| self.add_error(result, format!("Helius base URL: {}", e)))?;

        // Validate webhook URL if provided
        if let Some(ref webhook_url) = config.webhook_url {
            validation::validate_url(webhook_url)
                .map_err(|e| self.add_error(result, format!("Helius webhook URL: {}", e)))?;
        }

        // Validate rate limits
        if config.rate_limit_per_second == 0 {
            self.add_error(result, "Helius rate limit cannot be zero")?;
        }

        if config.rate_limit_per_second > 1000 {
            self.add_warning(result, "Very high Helius rate limit - ensure your plan supports this");
        }

        // Check webhook configuration consistency
        if config.enable_webhooks && config.webhook_url.is_none() {
            self.add_error(result, "Webhooks enabled but no webhook URL provided")?;
        }

        Ok(())
    }

    /// Validate Birdeye configuration
    fn validate_birdeye(&self, config: &super::models::BirdeyeConfig, result: &mut ValidationResult) -> AppResult<()> {
        // Validate API key
        if config.api_key.is_empty() {
            self.add_warning(result, "Birdeye API key is empty - functionality will be limited");
        } else {
            validation::validate_string_length(&config.api_key, Some(10), Some(100), "Birdeye API key")
                .map_err(|e| self.add_error(result, format!("Birdeye API key: {}", e)))?;
        }

        // Validate base URL
        validation::validate_url(&config.base_url)
            .map_err(|e| self.add_error(result, format!("Birdeye base URL: {}", e)))?;

        // Validate rate limits
        if config.rate_limit_per_minute == 0 {
            self.add_error(result, "Birdeye rate limit cannot be zero")?;
        }

        if config.rate_limit_per_minute > 10000 {
            self.add_warning(result, "Very high Birdeye rate limit - ensure your plan supports this");
        }

        // Validate cache TTL
        if config.cache_ttl_seconds == 0 {
            self.add_warning(result, "Cache TTL is zero - may cause excessive API calls");
        }

        if config.cache_ttl_seconds > 3600 {
            self.add_warning(result, "Cache TTL over 1 hour - data may become stale");
        }

        Ok(())
    }

    /// Validate database configuration
    fn validate_database(&self, config: &super::models::DatabaseConfig, result: &mut ValidationResult) -> AppResult<()> {
        // Validate database URL
        if config.url.is_empty() {
            self.add_error(result, "Database URL is required")?;
        } else {
            validation::config::validate_database_url(&config.url)
                .map_err(|e| self.add_error(result, format!("Database URL: {}", e)))?;
        }

        // Validate connection pool settings
        if config.max_connections == 0 {
            self.add_error(result, "Database max connections cannot be zero")?;
        }

        if config.min_connections > config.max_connections {
            self.add_error(result, "Database min connections cannot exceed max connections")?;
        }

        if config.max_connections > 100 {
            self.add_warning(result, "Very high database connection count - ensure database can handle this");
        }

        // Validate timeouts
        validation::trading::validate_timeout_ms(config.connection_timeout_ms)
            .map_err(|e| self.add_error(result, format!("Database connection timeout: {}", e)))?;

        validation::trading::validate_timeout_ms(config.idle_timeout_ms)
            .map_err(|e| self.add_error(result, format!("Database idle timeout: {}", e)))?;

        // Validate migration settings
        if !config.migration_path.is_empty() {
            validation::validate_string_length(&config.migration_path, Some(1), Some(255), "Migration path")
                .map_err(|e| self.add_error(result, format!("Migration path: {}", e)))?;
        }

        // Validate schema name if provided
        if let Some(ref schema) = config.schema {
            validation::validate_alphanumeric(schema, "Database schema")
                .map_err(|e| self.add_error(result, format!("Database schema: {}", e)))?;
        }

        // Validate slow query threshold
        if config.slow_query_threshold_ms > 10000 {
            self.add_warning(result, "Slow query threshold over 10 seconds - may not catch performance issues");
        }

        Ok(())
    }

    /// Validate Redis configuration
    fn validate_redis(&self, config: &super::models::RedisConfig, result: &mut ValidationResult) -> AppResult<()> {
        // Validate Redis URL
        if config.url.is_empty() {
            self.add_error(result, "Redis URL is required")?;
        } else {
            validation::config::validate_redis_url(&config.url)
                .map_err(|e| self.add_error(result, format!("Redis URL: {}", e)))?;
        }

        // Validate connection settings
        if config.max_connections == 0 {
            self.add_error(result, "Redis max connections cannot be zero")?;
        }

        if config.max_connections > 1000 {
            self.add_warning(result, "Very high Redis connection count");
        }

        // Validate timeouts
        validation::trading::validate_timeout_ms(config.connection_timeout_ms)
            .map_err(|e| self.add_error(result, format!("Redis connection timeout: {}", e)))?;

        validation::trading::validate_timeout_ms(config.command_timeout_ms)
            .map_err(|e| self.add_error(result, format!("Redis command timeout: {}", e)))?;

        // Validate TTL
        if config.default_ttl_seconds == 0 {
            self.add_warning(result, "Redis TTL is zero - keys will not expire");
        }

        // Validate key prefix
        validation::validate_alphanumeric(&config.key_prefix.replace(':', ""), "Redis key prefix")
            .map_err(|e| self.add_error(result, format!("Redis key prefix: {}", e)))?;

        // Validate persistence settings
        if config.enable_persistence && config.persistence_interval_seconds == 0 {
            self.add_error(result, "Redis persistence enabled but interval is zero")?;
        }

        Ok(())
    }

    /// Validate Telegram configuration
    fn validate_telegram(&self, config: &super::models::TelegramConfig, result: &mut ValidationResult) -> AppResult<()> {
        // Validate bot token
        if config.bot_token.is_empty() {
            self.add_error(result, "Telegram bot token is required")?;
        } else {
            // Basic Telegram bot token format validation
            if !config.bot_token.contains(':') || config.bot_token.len() < 20 {
                self.add_error(result, "Invalid Telegram bot token format")?;
            }
        }

        // Validate webhook URL if provided
        if let Some(ref webhook_url) = config.webhook_url {
            validation::validate_url(webhook_url)
                .map_err(|e| self.add_error(result, format!("Telegram webhook URL: {}", e)))?;
        }

        // Validate admin chat ID
        if config.admin_chat_id == 0 {
            self.add_warning(result, "Admin chat ID not set - admin features will not work");
        }

        // Validate allowed user IDs
        if config.allowed_user_ids.is_empty() {
            self.add_warning(result, "No allowed user IDs set - bot will be open to all users");
        }

        validation::validate_no_duplicates(&config.allowed_user_ids, "Allowed user IDs")
            .map_err(|e| self.add_error(result, format!("Allowed user IDs: {}", e)))?;

        // Validate timeouts
        validation::trading::validate_timeout_ms(config.command_timeout_ms)
            .map_err(|e| self.add_error(result, format!("Telegram command timeout: {}", e)))?;

        // Validate message length
        if config.max_message_length == 0 {
            self.add_error(result, "Max message length cannot be zero")?;
        }

        if config.max_message_length > 4096 {
            self.add_warning(result, "Max message length exceeds Telegram limit (4096)");
        }

        Ok(())
    }

    /// Validate trading configuration
    fn validate_trading(&self, config: &super::models::TradingConfig, result: &mut ValidationResult) -> AppResult<()> {
        // Validate position size
        validation::trading::validate_position_size(config.max_position_size_sol)
            .map_err(|e| self.add_error(result, format!("Max position size: {}", e)))?;

        // Validate slippage
        validation::trading::validate_slippage(config.default_slippage_percent)
            .map_err(|e| self.add_error(result, format!("Default slippage: {}", e)))?;

        // Validate stop loss and take profit
        validation::validate_percentage(config.stop_loss_percent)
            .map_err(|e| self.add_error(result, format!("Stop loss: {}", e)))?;

        validation::validate_percentage(config.take_profit_percent)
            .map_err(|e| self.add_error(result, format!("Take profit: {}", e)))?;

        // Validate concurrent trades
        validation::trading::validate_concurrent_trades(config.max_concurrent_trades)
            .map_err(|e| self.add_error(result, format!("Max concurrent trades: {}", e)))?;

        // Validate execution timeout
        validation::trading::validate_timeout_ms(config.trade_execution_timeout_ms)
            .map_err(|e| self.add_error(result, format!("Trade execution timeout: {}", e)))?;

        // Validate simulation settings
        if config.simulation_mode {
            if let Some(virtual_capital) = config.virtual_capital_sol {
                validation::trading::validate_position_size(virtual_capital)
                    .map_err(|e| self.add_error(result, format!("Virtual capital: {}", e)))?;
            } else {
                self.add_warning(result, "Simulation mode enabled but no virtual capital set");
            }
        }

        // Validate circuit breaker settings
        if config.enable_circuit_breaker {
            if let Some(loss_percent) = config.circuit_breaker_loss_percent {
                validation::validate_percentage(loss_percent)
                    .map_err(|e| self.add_error(result, format!("Circuit breaker loss: {}", e)))?;
            } else {
                self.add_error(result, "Circuit breaker enabled but no loss percentage set")?;
            }
        }

        // Validate daily trade limits
        if config.enable_position_limits {
            if let Some(limit) = config.daily_trade_limit {
                if limit == 0 {
                    self.add_error(result, "Daily trade limit cannot be zero when position limits are enabled")?;
                }
                if limit > 1000 {
                    self.add_warning(result, "Very high daily trade limit");
                }
            } else {
                self.add_error(result, "Position limits enabled but no daily trade limit set")?;
            }
        }

        // Logic validation
        if config.stop_loss_percent > config.take_profit_percent {
            self.add_warning(result, "Stop loss percentage is higher than take profit percentage");
        }

        // Production mode warnings
        if matches!(config.scenario_mode, crate::core::types::ScenarioMode::Production) {
            if config.simulation_mode {
                self.add_error(result, "Cannot run in simulation mode when scenario mode is production")?;
            }
            if !config.enable_circuit_breaker {
                self.add_warning(result, "Circuit breaker disabled in production mode");
            }
        }

        Ok(())
    }

    /// Validate risk configuration
    fn validate_risk(&self, config: &super::models::RiskConfig, result: &mut ValidationResult) -> AppResult<()> {
        // Validate risk score threshold
        validation::validate_risk_score(config.risk_score_threshold)
            .map_err(|e| self.add_error(result, format!("Risk score threshold: {}", e)))?;

        // Validate loss percentages
        validation::validate_percentage(config.max_daily_loss_percent)
            .map_err(|e| self.add_error(result, format!("Max daily loss: {}", e)))?;

        validation::validate_percentage(config.max_drawdown_percent)
            .map_err(|e| self.add_error(result, format!("Max drawdown: {}", e)))?;

        validation::validate_percentage(config.emergency_stop_loss_percent)
            .map_err(|e| self.add_error(result, format!("Emergency stop loss: {}", e)))?;

        // Validate liquidity requirements
        validation::trading::validate_position_size(config.min_liquidity_sol)
            .map_err(|e| self.add_error(result, format!("Min liquidity: {}", e)))?;

        // Validate holder count
        if config.min_holder_count == 0 {
            self.add_warning(result, "Minimum holder count is zero - may allow risky tokens");
        }

        if config.min_holder_count > 1000 {
            self.add_warning(result, "Very high minimum holder count - may exclude legitimate new tokens");
        }

        // Validate token age
        if config.max_token_age_seconds == 0 {
            self.add_warning(result, "Max token age is zero - only brand new tokens will be considered");
        }

        if config.max_token_age_seconds > 86400 {
            self.add_warning(result, "Max token age over 24 hours - may not be optimal for sniping");
        }

        // Validate whale monitoring settings
        if config.enable_whale_activity_monitoring {
            if let Some(threshold) = config.whale_threshold_percent {
                validation::validate_percentage(threshold)
                    .map_err(|e| self.add_error(result, format!("Whale threshold: {}", e)))?;
            } else {
                self.add_error(result, "Whale monitoring enabled but no threshold set")?;
            }
        }

        // Logic validation
        if config.max_daily_loss_percent > config.max_drawdown_percent {
            self.add_warning(result, "Daily loss limit exceeds drawdown limit");
        }

        if config.max_drawdown_percent > config.emergency_stop_loss_percent {
            self.add_warning(result, "Drawdown limit exceeds emergency stop loss");
        }

        // Feature dependency validation
        if !config.enable_liquidity_checks && config.min_liquidity_sol > rust_decimal::Decimal::ZERO {
            self.add_warning(result, "Liquidity minimum set but liquidity checks are disabled");
        }

        if !config.enable_holder_analysis && config.min_holder_count > 0 {
            self.add_warning(result, "Minimum holder count set but holder analysis is disabled");
        }

        Ok(())
    }

    /// Validate scanner configuration
    fn validate_scanner(&self, config: &super::models::ScannerConfig, result: &mut ValidationResult) -> AppResult<()> {
        // Validate scan interval
        if config.scan_interval_ms == 0 {
            self.add_error(result, "Scan interval cannot be zero")?;
        }

        if config.scan_interval_ms < 100 {
            self.add_warning(result, "Very fast scan interval - may cause rate limiting");
        }

        if config.scan_interval_ms > 60000 {
            self.add_warning(result, "Slow scan interval - may miss opportunities");
        }

        // Validate tokens per scan
        if config.max_tokens_per_scan == 0 {
            self.add_error(result, "Max tokens per scan cannot be zero")?;
        }

        if config.max_tokens_per_scan > 1000 {
            self.add_warning(result, "Very high tokens per scan - may cause performance issues");
        }

        // Validate market cap ranges
        if let (Some(min), Some(max)) = (config.min_market_cap_usd, config.max_market_cap_usd) {
            if min >= max {
                self.add_error(result, "Minimum market cap must be less than maximum market cap")?;
            }
        }

        // Validate blacklisted tokens
        for (i, token) in config.blacklisted_tokens.iter().enumerate() {
            validation::validate_solana_address(token)
                .map_err(|e| self.add_error(result, format!("Blacklisted token {}: {}", i, e)))?;
        }

        validation::validate_no_duplicates(&config.blacklisted_tokens, "Blacklisted tokens")
            .map_err(|e| self.add_error(result, format!("Blacklisted tokens: {}", e)))?;

        // Validate blacklisted developers
        for (i, dev) in config.blacklisted_developers.iter().enumerate() {
            validation::validate_solana_address(dev)
                .map_err(|e| self.add_error(result, format!("Blacklisted developer {}: {}", i, e)))?;
        }

        validation::validate_no_duplicates(&config.blacklisted_developers, "Blacklisted developers")
            .map_err(|e| self.add_error(result, format!("Blacklisted developers: {}", e)))?;

        // Validate whitelisted tokens
        for (i, token) in config.whitelisted_tokens.iter().enumerate() {
            validation::validate_solana_address(token)
                .map_err(|e| self.add_error(result, format!("Whitelisted token {}: {}", i, e)))?;
        }

        validation::validate_no_duplicates(&config.whitelisted_tokens, "Whitelisted tokens")
            .map_err(|e| self.add_error(result, format!("Whitelisted tokens: {}", e)))?;

        // Validate trading volume requirement
        if let Some(volume) = config.min_trading_volume_24h {
            if volume == 0 {
                self.add_warning(result, "Minimum trading volume is zero");
            }
        }

        // Logic validation
        if !config.enable_real_time_scanning && config.scan_interval_ms < 1000 {
            self.add_warning(result, "Fast scanning disabled but short interval set");
        }

        Ok(())
    }

    /// Validate analytics configuration
    fn validate_analytics(&self, config: &super::models::AnalyticsConfig, result: &mut ValidationResult) -> AppResult<()> {
        // Validate metrics port
        validation::validate_port(config.metrics_port)
            .map_err(|e| self.add_error(result, format!("Metrics port: {}", e)))?;

        // Validate retention period
        if config.trade_history_retention_days == 0 {
            self.add_warning(result, "Trade history retention is zero - no historical data will be kept");
        }

        if config.trade_history_retention_days > 3650 {
            self.add_warning(result, "Trade history retention over 10 years - consider storage costs");
        }

        // Validate report time format
        if let Some(ref time) = config.daily_report_time {
            if !time.contains(':') || time.len() != 5 {
                self.add_error(result, "Daily report time must be in HH:MM format")?;
            }
        }

        // Validate report frequency
        if let Some(frequency) = config.report_frequency_minutes {
            if frequency == 0 {
                self.add_error(result, "Report frequency cannot be zero")?;
            }
            if frequency < 5 {
                self.add_warning(result, "Very frequent reporting - may cause spam");
            }
        }

        // Validate delivery method
        if let Some(ref method) = config.report_delivery_method {
            match method.as_str() {
                "telegram" | "email" | "webhook" => {},
                _ => self.add_warning(result, "Unknown report delivery method"),
            }
        }

        Ok(())
    }

    /// Validate monitoring configuration
    fn validate_monitoring(&self, config: &super::models::MonitoringConfig, result: &mut ValidationResult) -> AppResult<()> {
        // Validate health check port
        validation::validate_port(config.health_check_port)
            .map_err(|e| self.add_error(result, format!("Health check port: {}", e)))?;

        // Validate health check interval
        if config.health_check_interval_seconds == 0 {
            self.add_error(result, "Health check interval cannot be zero")?;
        }

        if config.health_check_interval_seconds < 5 {
            self.add_warning(result, "Very frequent health checks - may impact performance");
        }

        // Validate error threshold
        if config.error_threshold_per_minute == 0 {
            self.add_warning(result, "Error threshold is zero - alerts may be too sensitive");
        }

        if config.error_threshold_per_minute > 1000 {
            self.add_warning(result, "Very high error threshold - may miss issues");
        }

        // Validate performance alert threshold
        if let Some(threshold) = config.performance_alert_threshold_ms {
            if threshold == 0 {
                self.add_error(result, "Performance alert threshold cannot be zero")?;
            }
        }

        // Logic validation
        if !config.enable_telegram_alerts && (config.alert_on_errors || config.alert_on_trade_failures) {
            self.add_warning(result, "Alerts enabled but Telegram alerts are disabled");
        }

        Ok(())
    }

    /// Validate security configuration
    fn validate_security(&self, config: &super::models::SecurityConfig, result: &mut ValidationResult) -> AppResult<()> {
        // Validate session timeout
        if config.session_timeout_minutes == 0 {
            self.add_error(result, "Session timeout cannot be zero")?;
        }

        if config.session_timeout_minutes > 1440 {
            self.add_warning(result, "Session timeout over 24 hours - may be a security risk");
        }

        // Validate failed attempts
        if config.max_failed_attempts == 0 {
            self.add_error(result, "Max failed attempts cannot be zero")?;
        }

        if config.max_failed_attempts > 10 {
            self.add_warning(result, "High failed attempt threshold - may allow brute force");
        }

        // Validate lockout duration
        if config.lockout_duration_minutes == 0 {
            self.add_warning(result, "Lockout duration is zero - no protection against brute force");
        }

        // Validate encryption key
        if config.encryption_key.is_empty() {
            self.add_error(result, "Encryption key is required")?;
        } else {
            // Basic validation - should be base64 encoded
            if let Err(_) = validation::validate_base64(&config.encryption_key) {
                self.add_error(result, "Encryption key must be valid base64")?;
            }
        }

        // Validate audit log retention
        if config.enable_audit_logging && config.audit_log_retention_days == 0 {
            self.add_warning(result, "Audit logging enabled but retention is zero");
        }

        // Validate IP ranges
        for (i, ip_range) in config.allowed_ip_ranges.iter().enumerate() {
            // Basic IP range validation
            if !ip_range.contains('/') && ip_range.parse::<std::net::IpAddr>().is_err() {
                self.add_error(result, format!("Invalid IP range {}: {}", i, ip_range))?;
            }
        }

        // Validate rate limiting
        if config.enable_rate_limiting {
            if let Some(limit) = config.rate_limit_per_minute {
                if limit == 0 {
                    self.add_error(result, "Rate limit cannot be zero when rate limiting is enabled")?;
                }
            } else {
                self.add_error(result, "Rate limiting enabled but no limit set")?;
            }
        }

        // Validate encryption algorithm
        match config.encryption_algorithm.as_str() {
            "AES-256-GCM" | "AES-256-CBC" | "ChaCha20-Poly1305" => {},
            _ => self.add_warning(result, "Non-standard encryption algorithm"),
        }

        // Production security warnings
        if config.enable_debug_mode {
            self.add_warning(result, "Debug mode enabled - disable in production");
        }

        if config.allow_unsafe_operations {
            self.add_error(result, "Unsafe operations enabled - never allow in production")?;
        }

        if config.log_sensitive_data {
            self.add_error(result, "Sensitive data logging enabled - never allow in production")?;
        }

        Ok(())
    }

    /// Validate performance configuration
    fn validate_performance(&self, config: &super::models::PerformanceConfig, result: &mut ValidationResult) -> AppResult<()> {
        // Validate memory limit
        if config.max_memory_mb == 0 {
            self.add_error(result, "Max memory cannot be zero")?;
        }

        if config.max_memory_mb < 128 {
            self.add_warning(result, "Very low memory limit - may cause performance issues");
        }

        if config.max_memory_mb > 8192 {
            self.add_warning(result, "Very high memory limit - ensure system has sufficient RAM");
        }

        // Validate CPU limit
        if config.max_cpu_percent == 0 {
            self.add_error(result, "Max CPU percentage cannot be zero")?;
        }

        if config.max_cpu_percent > 100 {
            self.add_error(result, "Max CPU percentage cannot exceed 100")?;
        }

        // Validate latency targets
        if config.target_detection_latency_ms == 0 {
            self.add_error(result, "Target detection latency cannot be zero")?;
        }

        if config.target_execution_latency_ms == 0 {
            self.add_error(result, "Target execution latency cannot be zero")?;
        }

        if config.target_detection_latency_ms > 10000 {
            self.add_warning(result, "Detection latency target over 10 seconds - may miss opportunities");
        }

        if config.target_execution_latency_ms > 1000 {
            self.add_warning(result, "Execution latency target over 1 second - may not be competitive");
        }

        // Validate thread configuration
        if config.worker_thread_count == 0 {
            self.add_error(result, "Worker thread count cannot be zero")?;
        }

        if config.worker_thread_count > 64 {
            self.add_warning(result, "Very high thread count - may cause context switching overhead");
        }

        // Validate connection pool size
        if config.connection_pool_size == 0 {
            self.add_error(result, "Connection pool size cannot be zero")?;
        }

        if config.connection_pool_size > 1000 {
            self.add_warning(result, "Very large connection pool - may exceed system limits");
        }

        Ok(())
    }

    /// Cross-configuration validation
    fn validate_cross_config(&self, config: &AppConfig, result: &mut ValidationResult) -> AppResult<()> {
        // Port conflicts
        let ports = vec![
            ("metrics", config.analytics.metrics_port),
            ("health_check", config.monitoring.health_check_port),
        ];

        for i in 0..ports.len() {
            for j in (i + 1)..ports.len() {
                if ports[i].1 == ports[j].1 {
                    self.add_error(result, format!("Port conflict: {} and {} both use port {}",
                                                   ports[i].0, ports[j].0, ports[i].1))?;
                }
            }
        }

        // Trading and risk configuration consistency
        if config.trading.max_position_size_sol < config.risk.min_liquidity_sol {
            self.add_warning(result, "Max position size is less than minimum liquidity requirement");
        }

        // Scanner and risk configuration consistency
        if config.scanner.max_tokens_per_scan > config.trading.max_concurrent_trades * 10 {
            self.add_warning(result, "Scanner may find more tokens than trading engine can handle");
        }

        // Performance and trading consistency
        if config.trading.trade_execution_timeout_ms > config.performance.target_execution_latency_ms {
            self.add_warning(result, "Trade timeout exceeds performance target");
        }

        // Database and analytics consistency
        if config.analytics.trade_history_retention_days > 0 && config.database.url.is_empty() {
            self.add_warning(result, "Trade history retention configured but no database URL provided");
        }

        // Security and environment consistency
        if config.environment.name == "production" {
            if config.security.enable_debug_mode {
                self.add_error(result, "Debug mode enabled in production environment")?;
            }
            if config.security.allow_unsafe_operations {
                self.add_error(result, "Unsafe operations enabled in production environment")?;
            }
            if config.trading.simulation_mode {
                self.add_error(result, "Simulation mode enabled in production environment")?;
            }
        }

        // Monitoring and alerting consistency
        if config.monitoring.enable_telegram_alerts && config.telegram.bot_token.is_empty() {
            self.add_error(result, "Telegram alerts enabled but no bot token provided")?;
        }

        // Scanner and external service consistency
        if config.scanner.enable_real_time_scanning && config.helius.api_key.is_empty() {
            self.add_warning(result, "Real-time scanning enabled but no Helius API key provided");
        }

        Ok(())
    }

    /// Add an error to the validation result
    fn add_error(&self, result: &mut ValidationResult, message: String) -> AppResult<()> {
        result.errors.push(message);
        result.is_valid = false;

        if !self.collect_all_errors {
            return Err(AppError::validation(result.errors.last().unwrap().clone()));
        }

        Ok(())
    }

    /// Add a warning to the validation result
    fn add_warning(&self, result: &mut ValidationResult, message: &str) {
        result.warnings.push(message.to_string());

        if self.strict_mode {
            result.is_valid = false;
        }
    }
}

impl Default for ConfigValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl AppConfig {
    /// Validate this configuration using the default validator
    pub fn validate(&self) -> AppResult<ValidationResult> {
        ConfigValidator::new().validate(self)
    }

    /// Validate this configuration with strict mode
    pub fn validate_strict(&self) -> AppResult<ValidationResult> {
        ConfigValidator::new().with_strict_mode().validate(self)
    }

    /// Quick validation that returns only a boolean result
    pub fn is_valid(&self) -> bool {
        self.validate().map(|r| r.is_valid).unwrap_or(false)
    }

    /// Validate and return errors if any
    pub fn validation_errors(&self) -> Vec<String> {
        self.validate()
            .map(|r| r.errors)
            .unwrap_or_else(|e| vec![e.to_string()])
    }

    /// Validate and return warnings if any
    pub fn validation_warnings(&self) -> Vec<String> {
        self.validate()
            .map(|r| r.warnings)
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::ScenarioMode;

    fn create_test_config() -> AppConfig {
        // Create a minimal valid configuration for testing
        AppConfig {
            environment: super::models::EnvironmentConfig {
                name: "test".to_string(),
                log_level: "info".to_string(),
                log_format: "pretty".to_string(),
                debug_mode: false,
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
                api_key: "test_key_1234567890".to_string(),
                base_url: "https://mainnet.helius-rpc.com".to_string(),
                webhook_url: None,
                enable_webhooks: false,
                rate_limit_per_second: 100,
            },
            birdeye: super::models::BirdeyeConfig {
                api_key: "test_key_1234567890".to_string(),
                base_url: "https://public-api.birdeye.so".to_string(),
                rate_limit_per_minute: 500,
                cache_ttl_seconds: 60,
            },
            database: super::models::DatabaseConfig {
                url: "postgres://user:pass@localhost/test".to_string(),
                max_connections: 20,
                min_connections: 5,
                connection_timeout_ms: 5000,
                idle_timeout_ms: 300000,
                auto_migrate: true,
                migration_path: "migrations".to_string(),
                schema: Some("test".to_string()),
                enable_query_logging: false,
                log_slow_queries: false,
                slow_query_threshold_ms: 1000,
            },
            redis: super::models::RedisConfig {
                url: "redis://localhost:6379".to_string(),
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
            telegram: super::models::TelegramConfig {
                bot_token: "123456789:ABCdefGHIjklMNOpqrsTUVwxyz".to_string(),
                webhook_url: None,
                admin_chat_id: 123456789,
                allowed_user_ids: vec![123456789],
                command_timeout_ms: 30000,
                max_message_length: 4096,
                enable_inline_keyboards: true,
            },
            trading: super::models::TradingConfig {
                scenario_mode: ScenarioMode::Development,
                max_position_size_sol: rust_decimal_macros::dec!(1.0),
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
                min_market_cap_usd: Some(1000),
                max_market_cap_usd: Some(1000000),
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
                daily_report_time: Some("09:00".to_string()),
                weekly_report_day: Some("monday".to_string()),
                enable_detailed_logging: false,
                enable_debug_reports: false,
                report_frequency_minutes: None,
                enable_daily_reports: false,
                enable_weekly_reports: false,
                enable_monthly_reports: false,
                report_delivery_method: Some("telegram".to_string()),
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
                encryption_key: "dGVzdF9lbmNyeXB0aW9uX2tleQ==".to_string(), // base64 encoded
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

    #[test]
    fn test_valid_configuration() {
        let config = create_test_config();
        let validator = ConfigValidator::new();
        let result = validator.validate(&config).unwrap();

        assert!(result.is_valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_invalid_configuration() {
        let mut config = create_test_config();
        config.solana.rpc_url = "invalid_url".to_string();
        config.telegram.bot_token = "invalid".to_string();

        let validator = ConfigValidator::new();
        let result = validator.validate(&config).unwrap();

        assert!(!result.is_valid);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_port_conflict_detection() {
        let mut config = create_test_config();
        config.analytics.metrics_port = 8080;  // Same as health check port

        let validator = ConfigValidator::new();
        let result = validator.validate(&config).unwrap();

        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("Port conflict")));
    }

    #[test]
    fn test_production_security_validation() {
        let mut config = create_test_config();
        config.environment.name = "production".to_string();
        config.security.enable_debug_mode = true;
        config.security.allow_unsafe_operations = true;

        let validator = ConfigValidator::new();
        let result = validator.validate(&config).unwrap();

        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("production environment")));
    }

    #[test]
    fn test_strict_mode_validation() {
        let mut config = create_test_config();
        config.risk.min_holder_count = 0;  // This should generate a warning

        let validator = ConfigValidator::new().with_strict_mode();
        let result = validator.validate(&config).unwrap();

        // In strict mode, warnings cause validation to fail
        assert!(!result.is_valid);
    }

    #[test]
    fn test_config_helper_methods() {
        let config = create_test_config();

        assert!(config.is_valid());
        assert!(config.validation_errors().is_empty());

        let mut invalid_config = config.clone();
        invalid_config.solana.rpc_url = "invalid".to_string();

        assert!(!invalid_config.is_valid());
        assert!(!invalid_config.validation_errors().is_empty());
    }

    #[test]
    fn test_scenario_mode_consistency() {
        let mut config = create_test_config();
        config.trading.scenario_mode = ScenarioMode::Production;
        config.trading.simulation_mode = true;  // Inconsistent

        let validator = ConfigValidator::new();
        let result = validator.validate(&config).unwrap();

        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("simulation mode")));
    }
}