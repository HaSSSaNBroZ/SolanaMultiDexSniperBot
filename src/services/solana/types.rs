//! Solana-specific type definitions
//!
//! This module contains all Solana blockchain-related types used throughout
//! the application, including token metadata, account information, and events.

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

/// Token metadata information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenMetadata {
    /// Token mint address
    pub address: String,
    /// Number of decimals
    pub decimals: u8,
    /// Total supply
    pub supply: u64,
    /// Token symbol
    pub symbol: Option<String>,
    /// Token name
    pub name: Option<String>,
    /// Metadata URI
    pub uri: Option<String>,
    /// Freeze authority
    pub freeze_authority: Option<String>,
    /// Mint authority
    pub mint_authority: Option<String>,
    /// Is initialized
    pub is_initialized: bool,
}

/// Account information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountInfo {
    /// Account address
    pub address: String,
    /// Lamports (1 SOL = 1e9 lamports)
    pub lamports: u64,
    /// Account data
    pub data: Vec<u8>,
    /// Owner program
    pub owner: String,
    /// Is executable
    pub executable: bool,
    /// Rent epoch
    pub rent_epoch: u64,
}

/// Token account information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenAccount {
    /// Account address
    pub address: String,
    /// Token mint
    pub mint: String,
    /// Owner wallet
    pub owner: String,
    /// Token amount
    pub amount: u64,
    /// Delegate
    pub delegate: Option<String>,
    /// Delegated amount
    pub delegated_amount: u64,
    /// Close authority
    pub close_authority: Option<String>,
}

/// Token balance information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBalance {
    /// Token mint
    pub mint: String,
    /// Owner address
    pub owner: String,
    /// Raw amount
    pub amount: u64,
    /// Decimals
    pub decimals: u8,
}

impl TokenBalance {
    /// Get UI amount (with decimals)
    pub fn ui_amount(&self) -> f64 {
        self.amount as f64 / 10f64.powi(self.decimals as i32)
    }

    /// Get UI amount as Decimal for precise calculations
    pub fn ui_amount_decimal(&self) -> Decimal {
        Decimal::new(self.amount as i64, self.decimals as u32)
    }
}

/// Liquidity pool information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityPool {
    /// Pool address
    pub address: String,
    /// DEX name (Raydium, Orca, etc.)
    pub dex: String,
    /// Token A mint
    pub token_a: String,
    /// Token B mint
    pub token_b: String,
    /// Token A reserves
    pub reserves_a: u64,
    /// Token B reserves
    pub reserves_b: u64,
    /// Pool liquidity in USD
    pub liquidity_usd: Option<f64>,
    /// 24h volume in USD
    pub volume_24h_usd: Option<f64>,
    /// Fee percentage
    pub fee_percent: f64,
    /// Pool creation time
    pub created_at: Option<DateTime<Utc>>,
}

impl LiquidityPool {
    /// Calculate price of token A in terms of token B
    pub fn price_a_to_b(&self) -> f64 {
        if self.reserves_a == 0 {
            return 0.0;
        }
        self.reserves_b as f64 / self.reserves_a as f64
    }

    /// Calculate price of token B in terms of token A
    pub fn price_b_to_a(&self) -> f64 {
        if self.reserves_b == 0 {
            return 0.0;
        }
        self.reserves_a as f64 / self.reserves_b as f64
    }

    /// Calculate price impact for a swap
    pub fn calculate_price_impact(&self, amount_in: u64, token_in_is_a: bool) -> f64 {
        let (reserves_in, reserves_out) = if token_in_is_a {
            (self.reserves_a, self.reserves_b)
        } else {
            (self.reserves_b, self.reserves_a)
        };

        // Using constant product formula: x * y = k
        let amount_out = (amount_in as f64 * reserves_out as f64) /
            (reserves_in as f64 + amount_in as f64);

        let new_price = (reserves_out as f64 - amount_out) /
            (reserves_in as f64 + amount_in as f64);
        let old_price = reserves_out as f64 / reserves_in as f64;

        ((new_price - old_price) / old_price * 100.0).abs()
    }
}

/// Token event from blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenEvent {
    /// Transaction signature
    pub signature: String,
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    /// Event type (TOKEN_MINT, CREATE_POOL, etc.)
    pub event_type: String,
    /// Involved accounts
    pub accounts: Vec<String>,
    /// Token transfers in the transaction
    pub token_transfers: Option<Vec<TokenTransferInfo>>,
    /// Additional metadata
    pub metadata: Option<serde_json::Value>,
}

/// Token transfer information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenTransferInfo {
    /// Source address
    pub from: String,
    /// Destination address
    pub to: String,
    /// Transfer amount
    pub amount: u64,
    /// Token mint
    pub mint: String,
}

/// Transaction information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionInfo {
    /// Transaction signature
    pub signature: String,
    /// Slot number
    pub slot: u64,
    /// Block time
    pub block_time: Option<DateTime<Utc>>,
    /// Transaction status
    pub status: TransactionStatus,
    /// Fee in lamports
    pub fee: u64,
    /// Instructions
    pub instructions: Vec<InstructionInfo>,
    /// Logs
    pub logs: Vec<String>,
}

/// Transaction status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransactionStatus {
    /// Transaction succeeded
    Success,
    /// Transaction failed
    Failed,
    /// Transaction is pending
    Pending,
}

/// Instruction information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionInfo {
    /// Program ID
    pub program_id: String,
    /// Instruction data
    pub data: Vec<u8>,
    /// Account keys
    pub accounts: Vec<String>,
}

/// Market data for a token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenMarketData {
    /// Token address
    pub address: String,
    /// Price in USD
    pub price_usd: f64,
    /// Market cap in USD
    pub market_cap_usd: f64,
    /// 24h volume in USD
    pub volume_24h_usd: f64,
    /// 24h price change percentage
    pub price_change_24h_percent: f64,
    /// Liquidity in USD
    pub liquidity_usd: f64,
    /// Number of holders
    pub holder_count: u32,
    /// Last update time
    pub last_updated: DateTime<Utc>,
}

/// Token creation event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenCreationEvent {
    /// Token mint address
    pub mint: String,
    /// Creator wallet
    pub creator: String,
    /// Initial supply
    pub initial_supply: u64,
    /// Decimals
    pub decimals: u8,
    /// Creation transaction
    pub transaction: String,
    /// Creation time
    pub created_at: DateTime<Utc>,
    /// Has freeze authority
    pub has_freeze_authority: bool,
    /// Has mint authority
    pub has_mint_authority: bool,
}

/// Pool creation event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolCreationEvent {
    /// Pool address
    pub pool_address: String,
    /// DEX name
    pub dex: String,
    /// Token A mint
    pub token_a: String,
    /// Token B mint
    pub token_b: String,
    /// Initial liquidity A
    pub initial_liquidity_a: u64,
    /// Initial liquidity B
    pub initial_liquidity_b: u64,
    /// Creator wallet
    pub creator: String,
    /// Creation transaction
    pub transaction: String,
    /// Creation time
    pub created_at: DateTime<Utc>,
}

/// Swap event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapEvent {
    /// Pool address
    pub pool_address: String,
    /// Trader wallet
    pub trader: String,
    /// Token in
    pub token_in: String,
    /// Token out
    pub token_out: String,
    /// Amount in
    pub amount_in: u64,
    /// Amount out
    pub amount_out: u64,
    /// Transaction signature
    pub transaction: String,
    /// Swap time
    pub swapped_at: DateTime<Utc>,
}

/// Program account filter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountFilter {
    /// Filter by owner program
    pub owner: Option<String>,
    /// Filter by data size
    pub data_size: Option<u64>,
    /// Filter by memcmp
    pub memcmp: Option<MemcmpFilter>,
}

/// Memcmp filter for account data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemcmpFilter {
    /// Offset in account data
    pub offset: usize,
    /// Bytes to compare (base58 encoded)
    pub bytes: String,
}

/// RPC error types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RpcError {
    /// Request timeout
    Timeout,
    /// Rate limit exceeded
    RateLimitExceeded,
    /// Invalid parameters
    InvalidParams(String),
    /// Node error
    NodeError(String),
    /// Network error
    NetworkError(String),
}

impl std::fmt::Display for RpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RpcError::Timeout => write!(f, "RPC request timed out"),
            RpcError::RateLimitExceeded => write!(f, "RPC rate limit exceeded"),
            RpcError::InvalidParams(msg) => write!(f, "Invalid RPC parameters: {}", msg),
            RpcError::NodeError(msg) => write!(f, "RPC node error: {}", msg),
            RpcError::NetworkError(msg) => write!(f, "Network error: {}", msg),
        }
    }
}

impl std::error::Error for RpcError {}

/// Birdeye API types (for token data enrichment)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BirdeyeTokenData {
    /// Token address
    pub address: String,
    /// Token symbol
    pub symbol: String,
    /// Token name
    pub name: String,
    /// Decimals
    pub decimals: u8,
    /// Price data
    pub price: BirdeyePriceData,
    /// Trade data
    pub trade: BirdeyeTradeData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BirdeyePriceData {
    /// Current price in USD
    pub value: f64,
    /// 24h change percentage
    pub change_24h: f64,
    /// Market cap
    pub market_cap: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BirdeyeTradeData {
    /// 24h volume
    pub volume_24h: f64,
    /// 24h trade count
    pub trade_24h: u32,
    /// Unique traders 24h
    pub unique_wallet_24h: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_balance_ui_amount() {
        let balance = TokenBalance {
            mint: "test_mint".to_string(),
            owner: "test_owner".to_string(),
            amount: 1_500_000_000, // 1.5 tokens with 9 decimals
            decimals: 9,
        };

        assert_eq!(balance.ui_amount(), 1.5);
        assert_eq!(balance.ui_amount_decimal(), Decimal::new(15, 1));
    }

    #[test]
    fn test_liquidity_pool_calculations() {
        let pool = LiquidityPool {
            address: "pool123".to_string(),
            dex: "Raydium".to_string(),
            token_a: "token_a".to_string(),
            token_b: "token_b".to_string(),
            reserves_a: 1000,
            reserves_b: 2000,
            liquidity_usd: Some(3000.0),
            volume_24h_usd: Some(500.0),
            fee_percent: 0.3,
            created_at: Some(Utc::now()),
        };

        assert_eq!(pool.price_a_to_b(), 2.0); // 1 token_a = 2 token_b
        assert_eq!(pool.price_b_to_a(), 0.5); // 1 token_b = 0.5 token_a

        // Test price impact calculation
        let impact = pool.calculate_price_impact(100, true);
        assert!(impact > 0.0); // Should have some price impact
    }

    #[test]
    fn test_transaction_status() {
        assert_eq!(
            serde_json::to_string(&TransactionStatus::Success).unwrap(),
            "\"Success\""
        );
        assert_eq!(
            serde_json::to_string(&TransactionStatus::Failed).unwrap(),
            "\"Failed\""
        );
    }
}