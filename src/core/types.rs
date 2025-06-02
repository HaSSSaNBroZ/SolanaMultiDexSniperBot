//! Core type definitions and value objects for the domain model
//!
//! This module contains strongly-typed wrappers around primitive types
//! to ensure type safety and prevent invalid states in the domain model.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

/// Unique identifier for a trading session
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub Uuid);

impl SessionId {
    /// Create a new random session ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Get the inner UUID value
    pub fn into_inner(self) -> Uuid {
        self.0
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for SessionId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::from_str(s)?))
    }
}

/// Unique identifier for a trade
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TradeId(pub Uuid);

impl TradeId {
    /// Create a new random trade ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Get the inner UUID value
    pub fn into_inner(self) -> Uuid {
        self.0
    }
}

impl Default for TradeId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for TradeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for TradeId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::from_str(s)?))
    }
}

/// Solana token mint address
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TokenAddress(pub String);

impl TokenAddress {
    /// Create a new token address with validation
    pub fn new(address: String) -> Result<Self, crate::core::error::AppError> {
        crate::core::validation::validate_solana_address(&address)?;
        Ok(Self(address))
    }

    /// Create without validation (use with caution)
    pub fn new_unchecked(address: String) -> Self {
        Self(address)
    }

    /// Get the address as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get the inner string value
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Display for TokenAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for TokenAddress {
    type Err = crate::core::error::AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s.to_string())
    }
}

/// Solana wallet address
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WalletAddress(pub String);

impl WalletAddress {
    /// Create a new wallet address with validation
    pub fn new(address: String) -> Result<Self, crate::core::error::AppError> {
        crate::core::validation::validate_solana_address(&address)?;
        Ok(Self(address))
    }

    /// Create without validation (use with caution)
    pub fn new_unchecked(address: String) -> Self {
        Self(address)
    }

    /// Get the address as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get the inner string value
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Display for WalletAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for WalletAddress {
    type Err = crate::core::error::AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s.to_string())
    }
}

/// Solana transaction signature
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TransactionSignature(pub String);

impl TransactionSignature {
    /// Create a new transaction signature
    pub fn new(signature: String) -> Self {
        Self(signature)
    }

    /// Get the signature as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get the inner string value
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Display for TransactionSignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for TransactionSignature {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(s.to_string()))
    }
}

/// Timestamp wrapper for consistent time handling
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Timestamp(pub DateTime<Utc>);

impl Timestamp {
    /// Create a timestamp for the current moment
    pub fn now() -> Self {
        Self(Utc::now())
    }

    /// Create a timestamp from a DateTime<Utc>
    pub fn from_datetime(datetime: DateTime<Utc>) -> Self {
        Self(datetime)
    }

    /// Get the inner DateTime<Utc> value
    pub fn into_inner(self) -> DateTime<Utc> {
        self.0
    }

    /// Get seconds since Unix epoch
    pub fn timestamp(&self) -> i64 {
        self.0.timestamp()
    }

    /// Get milliseconds since Unix epoch
    pub fn timestamp_millis(&self) -> i64 {
        self.0.timestamp_millis()
    }

    /// Get duration since another timestamp
    pub fn duration_since(&self, other: Timestamp) -> chrono::Duration {
        self.0 - other.0
    }
}

impl Default for Timestamp {
    fn default() -> Self {
        Self::now()
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.format("%Y-%m-%d %H:%M:%S UTC"))
    }
}

/// Token amount with proper decimal handling
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TokenAmount(pub Decimal);

impl TokenAmount {
    /// Create a new token amount with validation
    pub fn new(amount: Decimal) -> Result<Self, crate::core::error::AppError> {
        crate::core::validation::validate_trade_amount(amount)?;
        Ok(Self(amount))
    }

    /// Create a zero amount
    pub fn zero() -> Self {
        Self(Decimal::ZERO)
    }

    /// Create from SOL amount
    pub fn from_sol(sol: f64) -> Result<Self, crate::core::error::AppError> {
        let decimal = Decimal::try_from(sol)
            .map_err(|e| crate::core::error::AppError::validation(format!("Invalid SOL amount: {}", e)))?;
        Self::new(decimal)
    }

    /// Get the inner decimal value
    pub fn into_inner(self) -> Decimal {
        self.0
    }

    /// Convert to f64 (use with caution for precision-sensitive operations)
    pub fn to_f64(&self) -> f64 {
        self.0.try_into().unwrap_or(0.0)
    }

    /// Check if amount is zero
    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    /// Add two amounts
    pub fn add(&self, other: TokenAmount) -> Result<TokenAmount, crate::core::error::AppError> {
        let result = self.0 + other.0;
        Self::new(result)
    }

    /// Subtract two amounts
    pub fn subtract(&self, other: TokenAmount) -> Result<TokenAmount, crate::core::error::AppError> {
        if other.0 > self.0 {
            return Err(crate::core::error::AppError::validation("Cannot subtract larger amount"));
        }
        let result = self.0 - other.0;
        Self::new(result)
    }

    /// Multiply by a factor
    pub fn multiply(&self, factor: Decimal) -> Result<TokenAmount, crate::core::error::AppError> {
        let result = self.0 * factor;
        Self::new(result)
    }
}

impl fmt::Display for TokenAmount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} SOL", self.0)
    }
}

/// Percentage value with validation
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Percentage(pub Decimal);

impl Percentage {
    /// Create a new percentage with validation (0-100)
    pub fn new(value: Decimal) -> Result<Self, crate::core::error::AppError> {
        if value < Decimal::ZERO || value > rust_decimal_macros::dec!(100.0) {
            return Err(crate::core::error::AppError::validation(
                format!("Percentage must be between 0 and 100, got: {}", value)
            ));
        }
        Ok(Self(value))
    }

    /// Create from basis points (1/100th of a percent)
    pub fn from_basis_points(bp: u32) -> Result<Self, crate::core::error::AppError> {
        let decimal = Decimal::from(bp) / rust_decimal_macros::dec!(100.0);
        Self::new(decimal)
    }

    /// Get the inner decimal value
    pub fn into_inner(self) -> Decimal {
        self.0
    }

    /// Convert to decimal ratio (0.0 to 1.0)
    pub fn to_ratio(&self) -> Decimal {
        self.0 / rust_decimal_macros::dec!(100.0)
    }

    /// Convert to basis points
    pub fn to_basis_points(&self) -> u32 {
        (self.0 * rust_decimal_macros::dec!(100.0)).to_u32().unwrap_or(0)
    }
}

impl fmt::Display for Percentage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}%", self.0)
    }
}

/// Risk score (1-10 scale)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RiskScore(pub u8);

impl RiskScore {
    /// Create a new risk score with validation
    pub fn new(score: u8) -> Result<Self, crate::core::error::AppError> {
        crate::core::validation::validate_risk_score(score)?;
        Ok(Self(score))
    }

    /// Get the raw score value
    pub fn value(&self) -> u8 {
        self.0
    }

    /// Check if this is a low risk score (1-3)
    pub fn is_low_risk(&self) -> bool {
        self.0 <= 3
    }

    /// Check if this is a medium risk score (4-6)
    pub fn is_medium_risk(&self) -> bool {
        self.0 >= 4 && self.0 <= 6
    }

    /// Check if this is a high risk score (7-10)
    pub fn is_high_risk(&self) -> bool {
        self.0 >= 7
    }

    /// Get risk level as string
    pub fn risk_level(&self) -> &'static str {
        match self.0 {
            1..=3 => "Low",
            4..=6 => "Medium",
            7..=10 => "High",
            _ => "Invalid",
        }
    }
}

impl fmt::Display for RiskScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/10 ({})", self.0, self.risk_level())
    }
}

/// Trading scenario mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScenarioMode {
    /// Development mode with simulation
    Development,
    /// Production mode with real trading
    Production,
    /// Simulation mode for testing
    Simulation,
}

impl fmt::Display for ScenarioMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Development => write!(f, "development"),
            Self::Production => write!(f, "production"),
            Self::Simulation => write!(f, "simulation"),
        }
    }
}

impl FromStr for ScenarioMode {
    type Err = crate::core::error::AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "development" | "dev" => Ok(Self::Development),
            "production" | "prod" => Ok(Self::Production),
            "simulation" | "sim" => Ok(Self::Simulation),
            _ => Err(crate::core::error::AppError::validation(
                format!("Invalid scenario mode: {}", s)
            )),
        }
    }
}

/// DEX type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DexType {
    /// Raydium DEX
    Raydium,
    /// Jupiter Aggregator
    Jupiter,
    /// Pump.fun DEX
    PumpFun,
    /// Orca DEX
    Orca,
    /// Meteora DEX
    Meteora,
}

impl fmt::Display for DexType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Raydium => write!(f, "Raydium"),
            Self::Jupiter => write!(f, "Jupiter"),
            Self::PumpFun => write!(f, "Pump.fun"),
            Self::Orca => write!(f, "Orca"),
            Self::Meteora => write!(f, "Meteora"),
        }
    }
}

impl FromStr for DexType {
    type Err = crate::core::error::AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "raydium" => Ok(Self::Raydium),
            "jupiter" => Ok(Self::Jupiter),
            "pump.fun" | "pumpfun" => Ok(Self::PumpFun),
            "orca" => Ok(Self::Orca),
            "meteora" => Ok(Self::Meteora),
            _ => Err(crate::core::error::AppError::validation(
                format!("Invalid DEX type: {}", s)
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_id() {
        let id1 = SessionId::new();
        let id2 = SessionId::new();
        assert_ne!(id1, id2);

        let id_str = id1.to_string();
        let parsed = SessionId::from_str(&id_str).unwrap();
        assert_eq!(id1, parsed);
    }

    #[test]
    fn test_token_address() {
        let valid_address = "11111111111111111111111111111112";
        let addr = TokenAddress::new(valid_address.to_string()).unwrap();
        assert_eq!(addr.as_str(), valid_address);

        let invalid_address = "invalid";
        assert!(TokenAddress::new(invalid_address.to_string()).is_err());
    }

    #[test]
    fn test_token_amount() {
        let amount = TokenAmount::from_sol(1.5).unwrap();
        assert_eq!(amount.to_f64(), 1.5);

        let zero = TokenAmount::zero();
        assert!(zero.is_zero());

        let sum = amount.add(zero).unwrap();
        assert_eq!(sum, amount);
    }

    #[test]
    fn test_percentage() {
        let pct = Percentage::new(rust_decimal_macros::dec!(25.5)).unwrap();
        assert_eq!(pct.to_ratio(), rust_decimal_macros::dec!(0.255));

        let invalid = Percentage::new(rust_decimal_macros::dec!(150.0));
        assert!(invalid.is_err());
    }

    #[test]
    fn test_risk_score() {
        let low_risk = RiskScore::new(2).unwrap();
        assert!(low_risk.is_low_risk());
        assert_eq!(low_risk.risk_level(), "Low");

        let high_risk = RiskScore::new(8).unwrap();
        assert!(high_risk.is_high_risk());
        assert_eq!(high_risk.risk_level(), "High");

        let invalid = RiskScore::new(15);
        assert!(invalid.is_err());
    }

    #[test]
    fn test_scenario_mode() {
        let dev = ScenarioMode::from_str("development").unwrap();
        assert_eq!(dev, ScenarioMode::Development);

        let prod = ScenarioMode::from_str("production").unwrap();
        assert_eq!(prod, ScenarioMode::Production);

        let invalid = ScenarioMode::from_str("invalid");
        assert!(invalid.is_err());
    }

    #[test]
    fn test_dex_type() {
        let raydium = DexType::from_str("raydium").unwrap();
        assert_eq!(raydium, DexType::Raydium);
        assert_eq!(raydium.to_string(), "Raydium");

        let pump = DexType::from_str("pump.fun").unwrap();
        assert_eq!(pump, DexType::PumpFun);
    }
}