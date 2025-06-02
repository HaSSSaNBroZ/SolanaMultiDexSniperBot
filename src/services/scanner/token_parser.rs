//! Token metadata extraction and parsing
//!
//! This module handles parsing token metadata from on-chain accounts,
//! including SPL token data, Metaplex metadata, and liquidity information.

use std::sync::Arc;
use tracing::{debug, warn, error, instrument};
use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;

use crate::core::result::AppResult;
use crate::core::error::AppError;
use crate::core::types::{TokenAddress, Timestamp};
use crate::services::solana::SolanaService;

/// Parsed token information
#[derive(Debug, Clone)]
pub struct ParsedToken {
    /// Token address
    pub address: TokenAddress,

    /// Token metadata
    pub metadata: TokenMetadata,

    /// Market data
    pub market_data: MarketData,

    /// On-chain data
    pub on_chain_data: OnChainData,

    /// Parse timestamp
    pub parsed_at: Timestamp,
}

/// Token metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenMetadata {
    /// Token symbol
    pub symbol: Option<String>,

    /// Token name
    pub name: Option<String>,

    /// Token URI (for metadata)
    pub uri: Option<String>,

    /// Decimals
    pub decimals: u8,

    /// Total supply
    pub total_supply: u64,

    /// Mint authority
    pub mint_authority: Option<String>,

    /// Freeze authority
    pub freeze_authority: Option<String>,

    /// Metadata program
    pub metadata_program: Option<String>,

    /// Is verified collection
    pub is_verified: bool,

    /// Social links
    pub social_links: SocialLinks,
}

/// Social links extracted from metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SocialLinks {
    pub website: Option<String>,
    pub twitter: Option<String>,
    pub telegram: Option<String>,
    pub discord: Option<String>,
}

/// Market data for the token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketData {
    /// Market cap in USD
    pub market_cap_usd: Option<Decimal>,

    /// Price per token in SOL
    pub price_sol: Option<Decimal>,

    /// Price per token in USD
    pub price_usd: Option<Decimal>,

    /// 24h volume in USD
    pub volume_24h_usd: Option<Decimal>,

    /// Liquidity in SOL
    pub liquidity_sol: Option<Decimal>,

    /// Liquidity in USD
    pub liquidity_usd: Option<Decimal>,

    /// Number of liquidity pools
    pub pool_count: u32,

    /// Primary DEX
    pub primary_dex: Option<String>,
}

/// On-chain data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnChainData {
    /// Token age in seconds
    pub age_seconds: u64,

    /// Number of holders
    pub holder_count: u32,

    /// Creator/deployer address
    pub creator_address: Option<String>,

    /// First transaction signature
    pub first_tx_signature: Option<String>,

    /// Token program ID
    pub program_id: String,

    /// Associated token program
    pub associated_token_program: Option<String>,

    /// Is mutable metadata
    pub is_mutable: bool,

    /// Mint authority status
    pub mint_authority_status: MintAuthorityStatus,

    /// Freeze authority status
    pub freeze_authority_status: FreezeAuthorityStatus,
}

/// Mint authority status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MintAuthorityStatus {
    Active(String),
    Disabled,
    Unknown,
}

/// Freeze authority status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FreezeAuthorityStatus {
    Active(String),
    Disabled,
    Unknown,
}

/// Token parser for extracting metadata
#[derive(Debug)]
pub struct TokenParser {
    /// Solana service
    solana: Arc<SolanaService>,
}

impl TokenParser {
    /// Create a new token parser
    pub async fn new(solana: Arc<SolanaService>) -> AppResult<Self> {
        Ok(Self { solana })
    }

    /// Parse token information
    #[instrument(skip(self))]
    pub async fn parse_token(&self, token_address: &TokenAddress) -> AppResult<ParsedToken> {
        debug!("Parsing token: {}", token_address);

        // Fetch token account data
        let token_account = self.fetch_token_account(token_address).await?;

        // Parse basic token info
        let basic_info = self.parse_basic_info(&token_account)?;

        // Fetch and parse metadata
        let metadata = self.fetch_token_metadata(token_address, &basic_info).await?;

        // Fetch market data
        let market_data = self.fetch_market_data(token_address).await?;

        // Fetch on-chain data
        let on_chain_data = self.fetch_on_chain_data(token_address, &token_account).await?;

        Ok(ParsedToken {
            address: token_address.clone(),
            metadata,
            market_data,
            on_chain_data,
            parsed_at: Timestamp::now(),
        })
    }

    /// Fetch token account data
    async fn fetch_token_account(&self, token_address: &TokenAddress) -> AppResult<serde_json::Value> {
        self.solana.get_account_info(token_address.as_str()).await
            .map_err(|e| AppError::internal(format!("Failed to fetch token account: {}", e)))
    }

    /// Parse basic token information
    fn parse_basic_info(&self, account_data: &serde_json::Value) -> AppResult<BasicTokenInfo> {
        // Extract mint data from account
        let data = account_data.get("data")
            .ok_or_else(|| AppError::internal("Missing account data"))?;

        // This would decode the actual mint account data
        // For now, using placeholder values
        Ok(BasicTokenInfo {
            decimals: 9,
            supply: 1_000_000_000_000_000, // 1M tokens with 9 decimals
            mint_authority: None,
            freeze_authority: None,
        })
    }

    /// Fetch token metadata (Metaplex)
    async fn fetch_token_metadata(
        &self,
        token_address: &TokenAddress,
        basic_info: &BasicTokenInfo,
    ) -> AppResult<TokenMetadata> {
        // Try to fetch Metaplex metadata
        let metadata_account = self.get_metadata_account(token_address).await?;

        if let Some(metadata) = metadata_account {
            self.parse_metaplex_metadata(metadata, basic_info).await
        } else {
            // No Metaplex metadata, return basic info
            Ok(TokenMetadata {
                symbol: None,
                name: None,
                uri: None,
                decimals: basic_info.decimals,
                total_supply: basic_info.supply,
                mint_authority: basic_info.mint_authority.clone(),
                freeze_authority: basic_info.freeze_authority.clone(),
                metadata_program: None,
                is_verified: false,
                social_links: SocialLinks::default(),
            })
        }
    }

    /// Get metadata account address
    async fn get_metadata_account(&self, token_address: &TokenAddress) -> AppResult<Option<serde_json::Value>> {
        // Derive metadata PDA
        let metadata_program_id = "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s";

        // This would calculate the PDA and fetch the account
        // For now, returning None
        Ok(None)
    }

    /// Parse Metaplex metadata
    async fn parse_metaplex_metadata(
        &self,
        metadata_account: serde_json::Value,
        basic_info: &BasicTokenInfo,
    ) -> AppResult<TokenMetadata> {
        // This would decode Metaplex metadata
        // For now, using placeholder

        Ok(TokenMetadata {
            symbol: Some("TOKEN".to_string()),
            name: Some("Test Token".to_string()),
            uri: None,
            decimals: basic_info.decimals,
            total_supply: basic_info.supply,
            mint_authority: basic_info.mint_authority.clone(),
            freeze_authority: basic_info.freeze_authority.clone(),
            metadata_program: Some("metaplex".to_string()),
            is_verified: false,
            social_links: SocialLinks::default(),
        })
    }

    /// Fetch market data from external sources
    async fn fetch_market_data(&self, token_address: &TokenAddress) -> AppResult<MarketData> {
        // Fetch from multiple sources in parallel
        let (birdeye_data, helius_data) = tokio::join!(
            self.fetch_birdeye_data(token_address),
            self.fetch_helius_market_data(token_address)
        );

        // Merge data from multiple sources
        self.merge_market_data(birdeye_data.ok(), helius_data.ok())
    }

    /// Fetch data from Birdeye
    async fn fetch_birdeye_data(&self, token_address: &TokenAddress) -> AppResult<BirdeyeTokenData> {
        self.solana.fetch_birdeye_token_data(token_address.as_str()).await
    }

    /// Fetch market data from Helius
    async fn fetch_helius_market_data(&self, token_address: &TokenAddress) -> AppResult<serde_json::Value> {
        self.solana.fetch_helius_token_data(token_address.as_str()).await
    }

    /// Merge market data from multiple sources
    fn merge_market_data(
        &self,
        birdeye_data: Option<BirdeyeTokenData>,
        helius_data: Option<serde_json::Value>,
    ) -> AppResult<MarketData> {
        let mut market_data = MarketData {
            market_cap_usd: None,
            price_sol: None,
            price_usd: None,
            volume_24h_usd: None,
            liquidity_sol: None,
            liquidity_usd: None,
            pool_count: 0,
            primary_dex: None,
        };

        // Merge Birdeye data
        if let Some(birdeye) = birdeye_data {
            market_data.price_usd = birdeye.price_usd.and_then(|p| Decimal::try_from(p).ok());
            market_data.volume_24h_usd = birdeye.volume_24h_usd.and_then(|v| Decimal::try_from(v).ok());
            market_data.liquidity_usd = birdeye.liquidity_usd.and_then(|l| Decimal::try_from(l).ok());
            market_data.market_cap_usd = birdeye.market_cap_usd.and_then(|m| Decimal::try_from(m).ok());
        }

        // Merge Helius data
        if let Some(_helius) = helius_data {
            // Parse Helius-specific fields
        }

        Ok(market_data)
    }

    /// Fetch on-chain data
    async fn fetch_on_chain_data(
        &self,
        token_address: &TokenAddress,
        token_account: &serde_json::Value,
    ) -> AppResult<OnChainData> {
        // Get token creation time
        let creation_data = self.get_token_creation_data(token_address).await?;

        // Get holder count
        let holder_count = self.get_holder_count(token_address).await?;

        Ok(OnChainData {
            age_seconds: creation_data.age_seconds,
            holder_count,
            creator_address: creation_data.creator,
            first_tx_signature: creation_data.first_tx,
            program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
            associated_token_program: Some("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL".to_string()),
            is_mutable: true, // Would check from metadata
            mint_authority_status: MintAuthorityStatus::Unknown,
            freeze_authority_status: FreezeAuthorityStatus::Unknown,
        })
    }

    /// Get token creation data
    async fn get_token_creation_data(&self, token_address: &TokenAddress) -> AppResult<CreationData> {
        // Would fetch transaction history to find creation
        // For now, using placeholder
        Ok(CreationData {
            age_seconds: 300, // 5 minutes
            creator: Some("Creator11111111111111111111111111111111111".to_string()),
            first_tx: Some("FirstTx111111111111111111111111111111111111".to_string()),
        })
    }

    /// Get holder count
    async fn get_holder_count(&self, token_address: &TokenAddress) -> AppResult<u32> {
        // Would query token accounts
        // For now, returning placeholder
        Ok(42)
    }
}

/// Basic token information
struct BasicTokenInfo {
    decimals: u8,
    supply: u64,
    mint_authority: Option<String>,
    freeze_authority: Option<String>,
}

/// Birdeye token data structure
#[derive(Debug, Deserialize)]
pub struct BirdeyeTokenData {
    pub address: String,
    pub symbol: Option<String>,
    pub name: Option<String>,
    pub decimals: Option<u8>,
    pub price_usd: Option<f64>,
    pub volume_24h_usd: Option<f64>,
    pub liquidity_usd: Option<f64>,
    pub market_cap_usd: Option<f64>,
    pub holder_count: Option<u32>,
}

/// Token creation data
struct CreationData {
    age_seconds: u64,
    creator: Option<String>,
    first_tx: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_serialization() {
        let metadata = TokenMetadata {
            symbol: Some("TEST".to_string()),
            name: Some("Test Token".to_string()),
            uri: None,
            decimals: 9,
            total_supply: 1_000_000_000_000_000,
            mint_authority: None,
            freeze_authority: None,
            metadata_program: None,
            is_verified: false,
            social_links: SocialLinks::default(),
        };

        let json = serde_json::to_string(&metadata).unwrap();
        assert!(json.contains("TEST"));
    }
}