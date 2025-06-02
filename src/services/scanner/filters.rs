//! Token filtering and pre-screening
//!
//! This module implements various filters to screen tokens based on
//! safety criteria, liquidity requirements, and other configurable rules.

use std::sync::Arc;
use std::collections::HashMap;
use tracing::{debug, warn, instrument};
use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;

use crate::config::models::ScannerConfig;
use crate::core::result::AppResult;
use crate::core::error::AppError;
use crate::infrastructure::database::DatabaseService;
use super::{ParsedToken, TokenMetadata, MarketData, OnChainData};

/// Filter criteria
#[derive(Debug, Clone)]
pub struct FilterCriteria {
    /// Minimum liquidity in SOL
    pub min_liquidity_sol: Option<Decimal>,

    /// Maximum token age in seconds
    pub max_token_age_seconds: Option<u64>,

    /// Minimum holder count
    pub min_holder_count: Option<u32>,

    /// Minimum market cap in USD
    pub min_market_cap_usd: Option<Decimal>,

    /// Maximum market cap in USD
    pub max_market_cap_usd: Option<Decimal>,

    /// Require social links
    pub require_social_links: bool,

    /// Require contract verification
    pub require_contract_verification: bool,

    /// Blacklisted tokens
    pub blacklisted_tokens: Vec<String>,

    /// Blacklisted developers
    pub blacklisted_developers: Vec<String>,

    /// Whitelisted tokens (bypass filters)
    pub whitelisted_tokens: Vec<String>,
}

/// Filter result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterResult {
    /// Whether token passed all filters
    pub passed: bool,

    /// Individual filter results
    pub filter_results: HashMap<String, FilterCheck>,

    /// Rejection reasons if failed
    pub rejection_reasons: Vec<String>,

    /// Risk warnings (non-blocking)
    pub warnings: Vec<String>,

    /// Overall safety score (0-100)
    pub safety_score: u8,
}

/// Individual filter check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterCheck {
    /// Filter name
    pub name: String,

    /// Whether filter passed
    pub passed: bool,

    /// Filter value
    pub value: serde_json::Value,

    /// Expected value/criteria
    pub expected: serde_json::Value,

    /// Additional details
    pub details: Option<String>,
}

/// Token filter implementation
#[derive(Debug)]
pub struct TokenFilter {
    /// Configuration
    config: Arc<ScannerConfig>,

    /// Database service
    database: Arc<DatabaseService>,

    /// Filter criteria
    criteria: FilterCriteria,

    /// Individual filters
    filters: Vec<Box<dyn Filter>>,
}

/// Filter trait
#[async_trait::async_trait]
trait Filter: Send + Sync + std::fmt::Debug {
    /// Filter name
    fn name(&self) -> &str;

    /// Apply filter to token
    async fn apply(&self, token: &ParsedToken) -> AppResult<FilterCheck>;

    /// Is this a blocking filter?
    fn is_blocking(&self) -> bool {
        true
    }
}

/// Liquidity filter
#[derive(Debug)]
struct LiquidityFilter {
    min_liquidity_sol: Decimal,
}

#[async_trait::async_trait]
impl Filter for LiquidityFilter {
    fn name(&self) -> &str {
        "liquidity"
    }

    async fn apply(&self, token: &ParsedToken) -> AppResult<FilterCheck> {
        let liquidity = token.market_data.liquidity_sol.unwrap_or(Decimal::ZERO);
        let passed = liquidity >= self.min_liquidity_sol;

        Ok(FilterCheck {
            name: self.name().to_string(),
            passed,
            value: serde_json::json!(liquidity.to_string()),
            expected: serde_json::json!(self.min_liquidity_sol.to_string()),
            details: if !passed {
                Some(format!("Liquidity {} SOL is below minimum {} SOL",
                             liquidity, self.min_liquidity_sol))
            } else {
                None
            },
        })
    }
}

/// Token age filter
#[derive(Debug)]
struct TokenAgeFilter {
    max_age_seconds: u64,
}

#[async_trait::async_trait]
impl Filter for TokenAgeFilter {
    fn name(&self) -> &str {
        "token_age"
    }

    async fn apply(&self, token: &ParsedToken) -> AppResult<FilterCheck> {
        let age = token.on_chain_data.age_seconds;
        let passed = age <= self.max_age_seconds;

        Ok(FilterCheck {
            name: self.name().to_string(),
            passed,
            value: serde_json::json!(age),
            expected: serde_json::json!(self.max_age_seconds),
            details: if !passed {
                Some(format!("Token age {}s exceeds maximum {}s",
                             age, self.max_age_seconds))
            } else {
                None
            },
        })
    }
}

/// Holder count filter
#[derive(Debug)]
struct HolderCountFilter {
    min_holders: u32,
}

#[async_trait::async_trait]
impl Filter for HolderCountFilter {
    fn name(&self) -> &str {
        "holder_count"
    }

    async fn apply(&self, token: &ParsedToken) -> AppResult<FilterCheck> {
        let holders = token.on_chain_data.holder_count;
        let passed = holders >= self.min_holders;

        Ok(FilterCheck {
            name: self.name().to_string(),
            passed,
            value: serde_json::json!(holders),
            expected: serde_json::json!(self.min_holders),
            details: if !passed {
                Some(format!("Holder count {} is below minimum {}",
                             holders, self.min_holders))
            } else {
                None
            },
        })
    }
}

/// Market cap filter
#[derive(Debug)]
struct MarketCapFilter {
    min_market_cap: Option<Decimal>,
    max_market_cap: Option<Decimal>,
}

#[async_trait::async_trait]
impl Filter for MarketCapFilter {
    fn name(&self) -> &str {
        "market_cap"
    }

    async fn apply(&self, token: &ParsedToken) -> AppResult<FilterCheck> {
        let market_cap = token.market_data.market_cap_usd;

        let passed = match (market_cap, &self.min_market_cap, &self.max_market_cap) {
            (Some(cap), Some(min), Some(max)) => cap >= *min && cap <= *max,
            (Some(cap), Some(min), None) => cap >= *min,
            (Some(cap), None, Some(max)) => cap <= *max,
            (None, _, _) => false, // No market cap data
            _ => true,
        };

        Ok(FilterCheck {
            name: self.name().to_string(),
            passed,
            value: serde_json::json!(market_cap.map(|c| c.to_string())),
            expected: serde_json::json!({
                "min": self.min_market_cap.map(|m| m.to_string()),
                "max": self.max_market_cap.map(|m| m.to_string()),
            }),
            details: if !passed && market_cap.is_none() {
                Some("No market cap data available".to_string())
            } else {
                None
            },
        })
    }
}

/// Blacklist filter
#[derive(Debug)]
struct BlacklistFilter {
    blacklisted_tokens: Vec<String>,
    blacklisted_developers: Vec<String>,
}

#[async_trait::async_trait]
impl Filter for BlacklistFilter {
    fn name(&self) -> &str {
        "blacklist"
    }

    async fn apply(&self, token: &ParsedToken) -> AppResult<FilterCheck> {
        let token_blacklisted = self.blacklisted_tokens.contains(&token.address.to_string());

        let dev_blacklisted = if let Some(ref creator) = token.on_chain_data.creator_address {
            self.blacklisted_developers.contains(creator)
        } else {
            false
        };

        let passed = !token_blacklisted && !dev_blacklisted;

        Ok(FilterCheck {
            name: self.name().to_string(),
            passed,
            value: serde_json::json!({
                "token_blacklisted": token_blacklisted,
                "developer_blacklisted": dev_blacklisted,
            }),
            expected: serde_json::json!({
                "blacklisted": false,
            }),
            details: if !passed {
                if token_blacklisted {
                    Some("Token is blacklisted".to_string())
                } else {
                    Some("Developer is blacklisted".to_string())
                }
            } else {
                None
            },
        })
    }
}

/// Social links filter
#[derive(Debug)]
struct SocialLinksFilter {
    required: bool,
}

#[async_trait::async_trait]
impl Filter for SocialLinksFilter {
    fn name(&self) -> &str {
        "social_links"
    }

    async fn apply(&self, token: &ParsedToken) -> AppResult<FilterCheck> {
        let has_social = token.metadata.social_links.website.is_some()
            || token.metadata.social_links.twitter.is_some()
            || token.metadata.social_links.telegram.is_some()
            || token.metadata.social_links.discord.is_some();

        let passed = !self.required || has_social;

        Ok(FilterCheck {
            name: self.name().to_string(),
            passed,
            value: serde_json::json!({
                "has_website": token.metadata.social_links.website.is_some(),
                "has_twitter": token.metadata.social_links.twitter.is_some(),
                "has_telegram": token.metadata.social_links.telegram.is_some(),
                "has_discord": token.metadata.social_links.discord.is_some(),
            }),
            expected: serde_json::json!({
                "required": self.required,
            }),
            details: if !passed {
                Some("No social links found".to_string())
            } else {
                None
            },
        })
    }

    fn is_blocking(&self) -> bool {
        false // This is a warning, not a block
    }
}

/// Authority status filter
#[derive(Debug)]
struct AuthorityStatusFilter;

#[async_trait::async_trait]
impl Filter for AuthorityStatusFilter {
    fn name(&self) -> &str {
        "authority_status"
    }

    async fn apply(&self, token: &ParsedToken) -> AppResult<FilterCheck> {
        let mint_disabled = matches!(
            token.on_chain_data.mint_authority_status,
            super::MintAuthorityStatus::Disabled
        );

        let freeze_safe = matches!(
            token.on_chain_data.freeze_authority_status,
            super::FreezeAuthorityStatus::Disabled
        );

        let passed = true; // Not blocking, just informational

        Ok(FilterCheck {
            name: self.name().to_string(),
            passed,
            value: serde_json::json!({
                "mint_disabled": mint_disabled,
                "freeze_disabled": freeze_safe,
            }),
            expected: serde_json::json!({
                "mint_disabled": true,
                "freeze_disabled": true,
            }),
            details: if !mint_disabled || !freeze_safe {
                Some("Token authorities are still active".to_string())
            } else {
                None
            },
        })
    }

    fn is_blocking(&self) -> bool {
        false
    }
}

impl TokenFilter {
    /// Create a new token filter
    pub async fn new(
        config: Arc<ScannerConfig>,
        database: Arc<DatabaseService>,
    ) -> AppResult<Self> {
        let criteria = FilterCriteria {
            min_liquidity_sol: Some(
                Decimal::try_from(config.min_liquidity_sol.unwrap_or(1.0))
                    .map_err(|e| AppError::config(format!("Invalid liquidity value: {}", e)))?
            ),
            max_token_age_seconds: Some(config.max_token_age_seconds),
            min_holder_count: Some(config.min_holder_count),
            min_market_cap_usd: config.min_market_cap_usd.map(|v| Decimal::from(v)),
            max_market_cap_usd: config.max_market_cap_usd.map(|v| Decimal::from(v)),
            require_social_links: config.require_social_links,
            require_contract_verification: config.enable_contract_verification,
            blacklisted_tokens: config.blacklisted_tokens.clone(),
            blacklisted_developers: config.blacklisted_developers.clone(),
            whitelisted_tokens: config.whitelisted_tokens.clone(),
        };

        // Build filter chain
        let mut filters: Vec<Box<dyn Filter>> = Vec::new();

        // Add liquidity filter
        if let Some(min_liquidity) = criteria.min_liquidity_sol {
            filters.push(Box::new(LiquidityFilter { min_liquidity_sol: min_liquidity }));
        }

        // Add age filter
        if let Some(max_age) = criteria.max_token_age_seconds {
            filters.push(Box::new(TokenAgeFilter { max_age_seconds: max_age }));
        }

        // Add holder filter
        if let Some(min_holders) = criteria.min_holder_count {
            filters.push(Box::new(HolderCountFilter { min_holders }));
        }

        // Add market cap filter
        if criteria.min_market_cap_usd.is_some() || criteria.max_market_cap_usd.is_some() {
            filters.push(Box::new(MarketCapFilter {
                min_market_cap: criteria.min_market_cap_usd,
                max_market_cap: criteria.max_market_cap_usd,
            }));
        }

        // Add blacklist filter
        if !criteria.blacklisted_tokens.is_empty() || !criteria.blacklisted_developers.is_empty() {
            filters.push(Box::new(BlacklistFilter {
                blacklisted_tokens: criteria.blacklisted_tokens.clone(),
                blacklisted_developers: criteria.blacklisted_developers.clone(),
            }));
        }

        // Add social filter
        if criteria.require_social_links {
            filters.push(Box::new(SocialLinksFilter { required: true }));
        }

        // Add authority filter
        filters.push(Box::new(AuthorityStatusFilter));

        Ok(Self {
            config,
            database,
            criteria,
            filters,
        })
    }

    /// Apply all filters to a token
    #[instrument(skip(self, token))]
    pub async fn apply_filters(&self, token: &ParsedToken) -> AppResult<FilterResult> {
        debug!("Applying filters to token: {}", token.address);

        // Check whitelist first
        if self.criteria.whitelisted_tokens.contains(&token.address.to_string()) {
            debug!("Token is whitelisted, bypassing filters");
            return Ok(FilterResult {
                passed: true,
                filter_results: HashMap::new(),
                rejection_reasons: Vec::new(),
                warnings: Vec::new(),
                safety_score: 100,
            });
        }

        let mut filter_results = HashMap::new();
        let mut rejection_reasons = Vec::new();
        let mut warnings = Vec::new();
        let mut passed_filters = 0;
        let mut total_filters = 0;

        // Apply each filter
        for filter in &self.filters {
            let filter_name = filter.name().to_string();

            match filter.apply(token).await {
                Ok(result) => {
                    if result.passed {
                        passed_filters += 1;
                    } else if filter.is_blocking() {
                        if let Some(ref details) = result.details {
                            rejection_reasons.push(format!("{}: {}", filter_name, details));
                        } else {
                            rejection_reasons.push(format!("{} check failed", filter_name));
                        }
                    } else {
                        // Non-blocking filter failed
                        if let Some(ref details) = result.details {
                            warnings.push(format!("{}: {}", filter_name, details));
                        }
                    }

                    filter_results.insert(filter_name, result);
                    total_filters += 1;
                }
                Err(e) => {
                    warn!("Filter '{}' error: {}", filter_name, e);
                    // Continue with other filters
                }
            }
        }

        // Calculate safety score
        let safety_score = if total_filters > 0 {
            ((passed_filters as f64 / total_filters as f64) * 100.0) as u8
        } else {
            0
        };

        let passed = rejection_reasons.is_empty();

        debug!("Filter result for {}: passed={}, score={}, rejections={:?}",
               token.address, passed, safety_score, rejection_reasons);

        Ok(FilterResult {
            passed,
            filter_results,
            rejection_reasons,
            warnings,
            safety_score,
        })
    }

    /// Update filter criteria
    pub async fn update_criteria(&mut self, criteria: FilterCriteria) -> AppResult<()> {
        self.criteria = criteria;

        // Rebuild filters with new criteria
        // This would be implemented similar to the constructor

        info!("Filter criteria updated");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::Timestamp;

    #[tokio::test]
    async fn test_liquidity_filter() {
        let filter = LiquidityFilter {
            min_liquidity_sol: Decimal::from(5),
        };

        let mut token = create_test_token();
        token.market_data.liquidity_sol = Some(Decimal::from(10));

        let result = filter.apply(&token).await.unwrap();
        assert!(result.passed);

        token.market_data.liquidity_sol = Some(Decimal::from(2));
        let result = filter.apply(&token).await.unwrap();
        assert!(!result.passed);
    }

    fn create_test_token() -> ParsedToken {
        ParsedToken {
            address: TokenAddress::new_unchecked("TestToken111111111111111111111111111111111".to_string()),
            metadata: TokenMetadata {
                symbol: Some("TEST".to_string()),
                name: Some("Test Token".to_string()),
                uri: None,
                decimals: 9,
                total_supply: 1_000_000_000_000_000,
                mint_authority: None,
                freeze_authority: None,
                metadata_program: None,
                is_verified: false,
                social_links: super::super::SocialLinks::default(),
            },
            market_data: MarketData {
                market_cap_usd: None,
                price_sol: None,
                price_usd: None,
                volume_24h_usd: None,
                liquidity_sol: None,
                liquidity_usd: None,
                pool_count: 0,
                primary_dex: None,
            },
            on_chain_data: OnChainData {
                age_seconds: 300,
                holder_count: 50,
                creator_address: None,
                first_tx_signature: None,
                program_id: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
                associated_token_program: None,
                is_mutable: true,
                mint_authority_status: super::super::MintAuthorityStatus::Disabled,
                freeze_authority_status: super::super::FreezeAuthorityStatus::Disabled,
            },
            parsed_at: Timestamp::now(),
        }
    }
}