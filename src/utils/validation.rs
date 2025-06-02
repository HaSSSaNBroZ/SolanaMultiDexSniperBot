//! Input validation utilities and helpers
//!
//! This module provides comprehensive validation functions for various input types
//! used throughout the Solana Sniper Bot application.

use anyhow::{anyhow, Result};
use regex::Regex;
use rust_decimal::Decimal;
use std::collections::HashSet;
use url::Url;
use once_cell::sync::Lazy;

/// Regex for validating email addresses
static EMAIL_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap()
});

/// Regex for validating Solana addresses (base58, 32-44 characters)
static SOLANA_ADDRESS_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[1-9A-HJ-NP-Za-km-z]{32,44}$").unwrap()
});

/// Regex for validating transaction signatures
static TX_SIGNATURE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[1-9A-HJ-NP-Za-km-z]{87,88}$").unwrap()
});

/// Regex for validating hexadecimal strings
static HEX_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[0-9a-fA-F]+$").unwrap()
});

/// Regex for validating alphanumeric identifiers
static ALPHANUMERIC_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[a-zA-Z0-9_-]+$").unwrap()
});

/// Validate an email address
pub fn validate_email(email: &str) -> Result<()> {
    if email.is_empty() {
        return Err(anyhow!("Email cannot be empty"));
    }

    if email.len() > 254 {
        return Err(anyhow!("Email too long (max 254 characters)"));
    }

    if !EMAIL_REGEX.is_match(email) {
        return Err(anyhow!("Invalid email format"));
    }

    Ok(())
}

/// Validate a Solana address (wallet or token address)
pub fn validate_solana_address(address: &str) -> Result<()> {
    if address.is_empty() {
        return Err(anyhow!("Address cannot be empty"));
    }

    if !SOLANA_ADDRESS_REGEX.is_match(address) {
        return Err(anyhow!("Invalid Solana address format"));
    }

    // Additional validation for specific known addresses
    validate_not_system_address(address)?;

    Ok(())
}

/// Validate that an address is not a system address
fn validate_not_system_address(address: &str) -> Result<()> {
    const SYSTEM_ADDRESSES: &[&str] = &[
        "11111111111111111111111111111112", // System Program
        "Vote111111111111111111111111111111111111111", // Vote Program
    ];

    if SYSTEM_ADDRESSES.contains(&address) {
        return Err(anyhow!("Cannot use system program address"));
    }

    Ok(())
}

/// Validate a transaction signature
pub fn validate_transaction_signature(signature: &str) -> Result<()> {
    if signature.is_empty() {
        return Err(anyhow!("Transaction signature cannot be empty"));
    }

    if !TX_SIGNATURE_REGEX.is_match(signature) {
        return Err(anyhow!("Invalid transaction signature format"));
    }

    Ok(())
}

/// Validate a URL
pub fn validate_url(url_str: &str) -> Result<Url> {
    if url_str.is_empty() {
        return Err(anyhow!("URL cannot be empty"));
    }

    let url = Url::parse(url_str)
        .map_err(|e| anyhow!("Invalid URL format: {}", e))?;

    // Validate scheme
    match url.scheme() {
        "http" | "https" | "ws" | "wss" => {},
        _ => return Err(anyhow!("URL must use http, https, ws, or wss scheme")),
    }

    // Validate host
    if url.host().is_none() {
        return Err(anyhow!("URL must have a host"));
    }

    Ok(url)
}

/// Validate a port number
pub fn validate_port(port: u16) -> Result<()> {
    if port == 0 {
        return Err(anyhow!("Port cannot be 0"));
    }

    if port < 1024 {
        return Err(anyhow!("Port must be >= 1024 (non-privileged ports only)"));
    }

    Ok(())
}

/// Validate a decimal amount
pub fn validate_amount(amount: Decimal, min: Option<Decimal>, max: Option<Decimal>) -> Result<()> {
    if amount.is_sign_negative() {
        return Err(anyhow!("Amount cannot be negative"));
    }

    if let Some(min_val) = min {
        if amount < min_val {
            return Err(anyhow!("Amount {} is below minimum {}", amount, min_val));
        }
    }

    if let Some(max_val) = max {
        if amount > max_val {
            return Err(anyhow!("Amount {} exceeds maximum {}", amount, max_val));
        }
    }

    Ok(())
}

/// Validate a percentage (0-100)
pub fn validate_percentage(percentage: Decimal) -> Result<()> {
    validate_amount(
        percentage,
        Some(Decimal::ZERO),
        Some(rust_decimal_macros::dec!(100.0)),
    )?;
    Ok(())
}

/// Validate a risk score (1-10)
pub fn validate_risk_score(score: u8) -> Result<()> {
    if score < 1 || score > 10 {
        return Err(anyhow!("Risk score must be between 1 and 10, got: {}", score));
    }
    Ok(())
}

/// Validate a string length
pub fn validate_string_length(
    value: &str,
    min_length: Option<usize>,
    max_length: Option<usize>,
    field_name: &str,
) -> Result<()> {
    let len = value.len();

    if let Some(min) = min_length {
        if len < min {
            return Err(anyhow!(
                "{} too short: {} characters (minimum: {})",
                field_name, len, min
            ));
        }
    }

    if let Some(max) = max_length {
        if len > max {
            return Err(anyhow!(
                "{} too long: {} characters (maximum: {})",
                field_name, len, max
            ));
        }
    }

    Ok(())
}

/// Validate that a string is alphanumeric with allowed special characters
pub fn validate_alphanumeric(value: &str, field_name: &str) -> Result<()> {
    if value.is_empty() {
        return Err(anyhow!("{} cannot be empty", field_name));
    }

    if !ALPHANUMERIC_REGEX.is_match(value) {
        return Err(anyhow!(
            "{} must contain only letters, numbers, underscores, and hyphens",
            field_name
        ));
    }

    Ok(())
}

/// Validate a hexadecimal string
pub fn validate_hex_string(value: &str, expected_length: Option<usize>) -> Result<()> {
    if value.is_empty() {
        return Err(anyhow!("Hex string cannot be empty"));
    }

    // Remove 0x prefix if present
    let hex_value = value.strip_prefix("0x").unwrap_or(value);

    if !HEX_REGEX.is_match(hex_value) {
        return Err(anyhow!("Invalid hexadecimal format"));
    }

    if let Some(expected) = expected_length {
        if hex_value.len() != expected {
            return Err(anyhow!(
                "Hex string length mismatch: expected {}, got {}",
                expected, hex_value.len()
            ));
        }
    }

    Ok(())
}

/// Validate a list of items has no duplicates
pub fn validate_no_duplicates<T: std::hash::Hash + Eq + std::fmt::Debug>(
    items: &[T],
    field_name: &str,
) -> Result<()> {
    let mut seen = HashSet::new();

    for item in items {
        if !seen.insert(item) {
            return Err(anyhow!("Duplicate item found in {}: {:?}", field_name, item));
        }
    }

    Ok(())
}

/// Validate that a value is within a specified range
pub fn validate_range<T>(
    value: T,
    min: T,
    max: T,
    field_name: &str,
) -> Result<()>
where
    T: PartialOrd + std::fmt::Display,
{
    if value < min {
        return Err(anyhow!(
            "{} {} is below minimum {}",
            field_name, value, min
        ));
    }

    if value > max {
        return Err(anyhow!(
            "{} {} exceeds maximum {}",
            field_name, value, max
        ));
    }

    Ok(())
}

/// Validate a JSON string
pub fn validate_json(json_str: &str) -> Result<serde_json::Value> {
    serde_json::from_str(json_str)
        .map_err(|e| anyhow!("Invalid JSON format: {}", e))
}

/// Validate a base64 string
pub fn validate_base64(value: &str) -> Result<Vec<u8>> {
    use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

    BASE64.decode(value)
        .map_err(|e| anyhow!("Invalid base64 format: {}", e))
}

/// Validate a timestamp is within reasonable bounds
pub fn validate_timestamp(timestamp: i64) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    let one_year_ago = now - (365 * 24 * 60 * 60);
    let one_year_future = now + (365 * 24 * 60 * 60);

    if timestamp < one_year_ago {
        return Err(anyhow!("Timestamp is too far in the past"));
    }

    if timestamp > one_year_future {
        return Err(anyhow!("Timestamp is too far in the future"));
    }

    Ok(())
}

/// Validate that required fields are present
pub fn validate_required_fields(fields: &[(&str, &str)]) -> Result<()> {
    for (field_name, field_value) in fields {
        if field_value.trim().is_empty() {
            return Err(anyhow!("Required field '{}' is missing or empty", field_name));
        }
    }
    Ok(())
}

/// Validate configuration values
pub mod config {
    use super::*;

    /// Validate scenario mode
    pub fn validate_scenario_mode(mode: &str) -> Result<()> {
        match mode.to_lowercase().as_str() {
            "development" | "dev" | "production" | "prod" | "simulation" | "sim" => Ok(()),
            _ => Err(anyhow!("Invalid scenario mode: {}. Must be 'development', 'production', or 'simulation'", mode)),
        }
    }

    /// Validate log level
    pub fn validate_log_level(level: &str) -> Result<()> {
        match level.to_lowercase().as_str() {
            "trace" | "debug" | "info" | "warn" | "error" => Ok(()),
            _ => Err(anyhow!("Invalid log level: {}. Must be 'trace', 'debug', 'info', 'warn', or 'error'", level)),
        }
    }

    /// Validate environment name
    pub fn validate_environment(env: &str) -> Result<()> {
        validate_alphanumeric(env, "Environment")?;
        validate_string_length(env, Some(1), Some(50), "Environment")?;
        Ok(())
    }

    /// Validate database URL
    pub fn validate_database_url(url: &str) -> Result<()> {
        if !url.starts_with("postgres://") && !url.starts_with("postgresql://") {
            return Err(anyhow!("Database URL must start with 'postgres://' or 'postgresql://'"));
        }
        validate_url(url)?;
        Ok(())
    }

    /// Validate Redis URL
    pub fn validate_redis_url(url: &str) -> Result<()> {
        if !url.starts_with("redis://") && !url.starts_with("rediss://") {
            return Err(anyhow!("Redis URL must start with 'redis://' or 'rediss://'"));
        }
        validate_url(url)?;
        Ok(())
    }
}

/// Trading-specific validation
pub mod trading {
    use super::*;

    /// Validate slippage percentage
    pub fn validate_slippage(slippage: Decimal) -> Result<()> {
        validate_amount(
            slippage,
            Some(rust_decimal_macros::dec!(0.1)),
            Some(rust_decimal_macros::dec!(50.0)),
        )?;
        Ok(())
    }

    /// Validate position size
    pub fn validate_position_size(size: Decimal) -> Result<()> {
        validate_amount(
            size,
            Some(rust_decimal_macros::dec!(0.001)),
            Some(rust_decimal_macros::dec!(1000.0)),
        )?;
        Ok(())
    }

    /// Validate concurrent trades limit
    pub fn validate_concurrent_trades(count: u32) -> Result<()> {
        if count == 0 {
            return Err(anyhow!("Concurrent trades count cannot be zero"));
        }
        if count > 100 {
            return Err(anyhow!("Concurrent trades count cannot exceed 100"));
        }
        Ok(())
    }

    /// Validate timeout value
    pub fn validate_timeout_ms(timeout_ms: u64) -> Result<()> {
        if timeout_ms == 0 {
            return Err(anyhow!("Timeout cannot be zero"));
        }
        if timeout_ms > 300_000 {
            return Err(anyhow!("Timeout cannot exceed 5 minutes (300,000ms)"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_validation() {
        assert!(validate_email("test@example.com").is_ok());
        assert!(validate_email("user.name+tag@domain.co.uk").is_ok());

        assert!(validate_email("").is_err());
        assert!(validate_email("invalid-email").is_err());
        assert!(validate_email("@domain.com").is_err());
        assert!(validate_email("user@").is_err());
    }

    #[test]
    fn test_solana_address_validation() {
        assert!(validate_solana_address("11111111111111111111111111111112").is_ok());
        assert!(validate_solana_address("So11111111111111111111111111111111111111112").is_ok());

        assert!(validate_solana_address("").is_err());
        assert!(validate_solana_address("invalid").is_err());
        assert!(validate_solana_address("11111111111111111111111111111111").is_err()); // too short
    }

    #[test]
    fn test_url_validation() {
        assert!(validate_url("https://example.com").is_ok());
        assert!(validate_url("http://localhost:8080").is_ok());
        assert!(validate_url("wss://api.example.com").is_ok());

        assert!(validate_url("").is_err());
        assert!(validate_url("invalid-url").is_err());
        assert!(validate_url("ftp://example.com").is_err());
    }

    #[test]
    fn test_amount_validation() {
        assert!(validate_amount(
            rust_decimal_macros::dec!(50.0),
            Some(rust_decimal_macros::dec!(0.0)),
            Some(rust_decimal_macros::dec!(100.0))
        ).is_ok());

        assert!(validate_amount(
            rust_decimal_macros::dec!(-1.0),
            None,
            None
        ).is_err());

        assert!(validate_amount(
            rust_decimal_macros::dec!(150.0),
            Some(rust_decimal_macros::dec!(0.0)),
            Some(rust_decimal_macros::dec!(100.0))
        ).is_err());
    }

    #[test]
    fn test_string_length_validation() {
        assert!(validate_string_length("hello", Some(3), Some(10), "test").is_ok());
        assert!(validate_string_length("hi", Some(3), Some(10), "test").is_err());
        assert!(validate_string_length("this is too long", Some(3), Some(10), "test").is_err());
    }

    #[test]
    fn test_hex_validation() {
        assert!(validate_hex_string("deadbeef", None).is_ok());
        assert!(validate_hex_string("0xdeadbeef", None).is_ok());
        assert!(validate_hex_string("DEADBEEF", None).is_ok());

        assert!(validate_hex_string("", None).is_err());
        assert!(validate_hex_string("invalid", None).is_err());
        assert!(validate_hex_string("deadbeef", Some(4)).is_err()); // wrong length
    }

    #[test]
    fn test_no_duplicates_validation() {
        assert!(validate_no_duplicates(&[1, 2, 3, 4], "numbers").is_ok());
        assert!(validate_no_duplicates(&[1, 2, 3, 2], "numbers").is_err());
    }

    #[test]
    fn test_range_validation() {
        assert!(validate_range(5, 1, 10, "value").is_ok());
        assert!(validate_range(0, 1, 10, "value").is_err());
        assert!(validate_range(15, 1, 10, "value").is_err());
    }

    #[test]
    fn test_json_validation() {
        assert!(validate_json(r#"{"key": "value"}"#).is_ok());
        assert!(validate_json("invalid json").is_err());
    }

    #[test]
    fn test_base64_validation() {
        assert!(validate_base64("SGVsbG8gV29ybGQ=").is_ok());
        assert!(validate_base64("invalid base64!").is_err());
    }

    #[test]
    fn test_config_validations() {
        assert!(config::validate_scenario_mode("development").is_ok());
        assert!(config::validate_scenario_mode("production").is_ok());
        assert!(config::validate_scenario_mode("invalid").is_err());

        assert!(config::validate_log_level("info").is_ok());
        assert!(config::validate_log_level("invalid").is_err());
    }

    #[test]
    fn test_trading_validations() {
        assert!(trading::validate_slippage(rust_decimal_macros::dec!(5.0)).is_ok());
        assert!(trading::validate_slippage(rust_decimal_macros::dec!(100.0)).is_err());

        assert!(trading::validate_concurrent_trades(10).is_ok());
        assert!(trading::validate_concurrent_trades(0).is_err());
        assert!(trading::validate_concurrent_trades(200).is_err());
    }
}