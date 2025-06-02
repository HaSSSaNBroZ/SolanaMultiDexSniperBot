//! Helius API client implementation
//!
//! This module provides integration with Helius RPC and enhanced APIs
//! for real-time token detection, webhooks, and enhanced metadata.

use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, Semaphore};
use tracing::{debug, info, warn, error, instrument};
use backoff::{ExponentialBackoff, backoff::Backoff};

use crate::config::models::HeliusConfig;
use crate::core::result::AppResult;
use crate::core::error::AppError;
use super::types::{TokenMetadata, TokenEvent, LiquidityPool};

/// Helius API base URL
const HELIUS_API_BASE: &str = "https://api.helius.xyz/v0";

/// Maximum concurrent Helius requests
const MAX_CONCURRENT_REQUESTS: usize = 5;

/// Helius API client
#[derive(Debug, Clone)]
pub struct HeliusClient {
    /// HTTP client
    http_client: Client,
    /// API configuration
    config: HeliusConfig,
    /// Request semaphore for rate limiting
    semaphore: Arc<Semaphore>,
    /// Rate limiter
    rate_limiter: Arc<RwLock<RateLimiter>>,
    /// Client statistics
    stats: Arc<RwLock<HeliusStats>>,
}

impl HeliusClient {
    /// Create a new Helius client
    pub async fn new(config: &HeliusConfig) -> AppResult<Self> {
        info!("🌟 Initializing Helius API client");

        if config.api_key.is_empty() {
            warn!("⚠️  Helius API key not configured - functionality will be limited");
        }

        let http_client = Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| AppError::network(format!("Failed to create HTTP client: {}", e)))?;

        let rate_limiter = Arc::new(RwLock::new(RateLimiter::new(config.rate_limit_per_second)));

        Ok(Self {
            http_client,
            config: config.clone(),
            semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_REQUESTS)),
            rate_limiter,
            stats: Arc::new(RwLock::new(HeliusStats::new())),
        })
    }

    /// Get enhanced token metadata
    #[instrument(skip(self))]
    pub async fn get_token_metadata(&self, mint_address: &str) -> AppResult<TokenMetadata> {
        self.check_api_key()?;

        let url = format!("{}/token-metadata", self.config.base_url);
        let params = [
            ("api-key", self.config.api_key.as_str()),
            ("mint-accounts", mint_address),
        ];

        let response: Vec<HeliusTokenMetadata> = self.execute_request(&url, &params).await?;

        response.into_iter()
            .next()
            .map(|m| m.into())
            .ok_or_else(|| AppError::network("No metadata found for token"))
    }

    /// Get token holders
    #[instrument(skip(self))]
    pub async fn get_token_holders(&self, mint_address: &str, limit: u32) -> AppResult<Vec<TokenHolder>> {
        self.check_api_key()?;

        let url = format!("{}/token-holders", self.config.base_url);
        let params = [
            ("api-key", self.config.api_key.as_str()),
            ("mint-account", mint_address),
            ("limit", &limit.to_string()),
        ];

        self.execute_request(&url, &params).await
    }

    /// Get recent token transactions
    #[instrument(skip(self))]
    pub async fn get_token_transactions(
        &self,
        mint_address: &str,
        limit: u32
    ) -> AppResult<Vec<TokenTransaction>> {
        self.check_api_key()?;

        let url = format!("{}/addresses/{}/transactions", self.config.base_url, mint_address);
        let params = [
            ("api-key", self.config.api_key.as_str()),
            ("limit", &limit.to_string()),
        ];

        self.execute_request(&url, &params).await
    }

    /// Subscribe to webhooks for real-time events
    #[instrument(skip(self))]
    pub async fn subscribe_to_webhooks(&self, callback_url: &str) -> AppResult<String> {
        self.check_api_key()?;

        let url = format!("{}/webhooks", self.config.base_url);

        let webhook_config = WebhookSubscription {
            webhook_url: callback_url.to_string(),
            transaction_types: vec![
                "TOKEN_MINT".to_string(),
                "CREATE_POOL".to_string(),
                "ADD_LIQUIDITY".to_string(),
            ],
            account_addresses: vec![],
            webhook_type: "enhanced".to_string(),
            auth_header: None,
        };

        let response: WebhookResponse = self.execute_post_request(&url, &webhook_config).await?;

        info!("✅ Webhook subscription created: {}", response.webhook_id);
        Ok(response.webhook_id)
    }

    /// Parse webhook event
    pub fn parse_webhook_event(&self, payload: &str) -> AppResult<TokenEvent> {
        let event: HeliusWebhookPayload = serde_json::from_str(payload)
            .map_err(|e| AppError::network(format!("Failed to parse webhook payload: {}", e)))?;

        Ok(event.into())
    }

    /// Get liquidity pool information
    #[instrument(skip(self))]
    pub async fn get_liquidity_pools(&self, token_address: &str) -> AppResult<Vec<LiquidityPool>> {
        self.check_api_key()?;

        let url = format!("{}/token/{}/pools", self.config.base_url, token_address);
        let params = [("api-key", self.config.api_key.as_str())];

        self.execute_request(&url, &params).await
    }

    /// Health check
    pub async fn health_check(&self) -> AppResult<String> {
        if self.config.api_key.is_empty() {
            return Err(AppError::network("Helius API key not configured"));
        }

        let url = format!("{}/health", self.config.base_url);
        let params = [("api-key", self.config.api_key.as_str())];

        let response = self.http_client
            .get(&url)
            .query(&params)
            .send()
            .await
            .map_err(|e| AppError::network(format!("Health check failed: {}", e)))?;

        if response.status().is_success() {
            Ok("Helius API is healthy".to_string())
        } else {
            Err(AppError::network(format!(
                "Helius API unhealthy: status {}",
                response.status()
            )))
        }
    }

    /// Execute GET request with retry logic
    async fn execute_request<T>(&self, url: &str, params: &[(&str, &str)]) -> AppResult<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let _permit = self.semaphore.acquire().await
            .map_err(|_| AppError::internal("Failed to acquire Helius semaphore"))?;

        // Wait for rate limit
        self.rate_limiter.write().await.wait_if_needed().await;

        let mut backoff = ExponentialBackoff::default();
        backoff.max_elapsed_time = Some(Duration::from_secs(30));

        let start_time = std::time::Instant::now();
        let mut last_error = None;

        loop {
            match self.http_client
                .get(url)
                .query(params)
                .send()
                .await
            {
                Ok(response) => {
                    let status = response.status();

                    if status.is_success() {
                        let data = response.json::<T>().await
                            .map_err(|e| AppError::network(format!("Failed to parse response: {}", e)))?;

                        self.record_success(start_time.elapsed()).await;
                        return Ok(data);
                    } else if status == StatusCode::TOO_MANY_REQUESTS {
                        warn!("⚠️  Helius rate limit hit");
                        self.rate_limiter.write().await.add_penalty();
                        last_error = Some(AppError::network("Rate limit exceeded"));
                    } else {
                        let error_text = response.text().await.unwrap_or_default();
                        last_error = Some(AppError::network(format!(
                            "Helius API error ({}): {}",
                            status, error_text
                        )));
                    }
                }
                Err(e) => {
                    last_error = Some(AppError::network(format!("Request failed: {}", e)));
                }
            }

            match backoff.next_backoff() {
                Some(duration) => {
                    debug!("🔄 Retrying Helius request after {:?}", duration);
                    tokio::time::sleep(duration).await;
                }
                None => {
                    self.record_failure().await;
                    return Err(last_error.unwrap_or_else(|| AppError::network("Request failed")));
                }
            }
        }
    }

    /// Execute POST request
    async fn execute_post_request<B, T>(&self, url: &str, body: &B) -> AppResult<T>
    where
        B: Serialize,
        T: for<'de> Deserialize<'de>,
    {
        let _permit = self.semaphore.acquire().await
            .map_err(|_| AppError::internal("Failed to acquire Helius semaphore"))?;

        self.rate_limiter.write().await.wait_if_needed().await;

        let response = self.http_client
            .post(url)
            .header("x-api-key", &self.config.api_key)
            .json(body)
            .send()
            .await
            .map_err(|e| AppError::network(format!("POST request failed: {}", e)))?;

        if response.status().is_success() {
            response.json::<T>().await
                .map_err(|e| AppError::network(format!("Failed to parse response: {}", e)))
        } else {
            let error_text = response.text().await.unwrap_or_default();
            Err(AppError::network(format!(
                "Helius API error ({}): {}",
                response.status(), error_text
            )))
        }
    }

    /// Check if API key is configured
    fn check_api_key(&self) -> AppResult<()> {
        if self.config.api_key.is_empty() {
            Err(AppError::config("Helius API key not configured"))
        } else {
            Ok(())
        }
    }

    /// Record successful request
    async fn record_success(&self, duration: Duration) {
        let mut stats = self.stats.write().await;
        stats.total_requests += 1;
        stats.successful_requests += 1;
        stats.total_duration += duration;
    }

    /// Record failed request
    async fn record_failure(&self) {
        let mut stats = self.stats.write().await;
        stats.total_requests += 1;
        stats.failed_requests += 1;
    }

    /// Get client statistics
    pub async fn get_statistics(&self) -> HeliusStats {
        self.stats.read().await.clone()
    }

    /// Close the client
    pub async fn close(&self) -> AppResult<()> {
        info!("🔌 Closing Helius client");
        Ok(())
    }
}

/// Rate limiter for API requests
#[derive(Debug)]
struct RateLimiter {
    /// Requests per second limit
    limit: u32,
    /// Last request timestamp
    last_request: Option<std::time::Instant>,
    /// Penalty delay for rate limit hits
    penalty_until: Option<std::time::Instant>,
}

impl RateLimiter {
    fn new(limit: u32) -> Self {
        Self {
            limit,
            last_request: None,
            penalty_until: None,
        }
    }

    async fn wait_if_needed(&mut self) {
        // Check if we're in penalty
        if let Some(penalty_until) = self.penalty_until {
            if penalty_until > std::time::Instant::now() {
                let wait_duration = penalty_until - std::time::Instant::now();
                tokio::time::sleep(wait_duration).await;
                self.penalty_until = None;
            }
        }

        // Check rate limit
        if let Some(last_request) = self.last_request {
            let min_interval = Duration::from_millis(1000 / self.limit as u64);
            let elapsed = last_request.elapsed();

            if elapsed < min_interval {
                tokio::time::sleep(min_interval - elapsed).await;
            }
        }

        self.last_request = Some(std::time::Instant::now());
    }

    fn add_penalty(&mut self) {
        // Add 1 second penalty for rate limit hit
        self.penalty_until = Some(std::time::Instant::now() + Duration::from_secs(1));
    }
}

/// Helius statistics
#[derive(Debug, Clone)]
pub struct HeliusStats {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub total_duration: Duration,
    pub started_at: chrono::DateTime<chrono::Utc>,
}

impl HeliusStats {
    fn new() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            total_duration: Duration::ZERO,
            started_at: chrono::Utc::now(),
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            self.successful_requests as f64 / self.total_requests as f64 * 100.0
        }
    }

    pub fn avg_response_time(&self) -> Duration {
        if self.successful_requests == 0 {
            Duration::ZERO
        } else {
            self.total_duration / self.successful_requests as u32
        }
    }
}

/// Helius webhook types and structures

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeliusWebhook {
    pub webhook_id: String,
    pub webhook_url: String,
    pub transaction_types: Vec<String>,
}

#[derive(Debug, Serialize)]
struct WebhookSubscription {
    webhook_url: String,
    transaction_types: Vec<String>,
    account_addresses: Vec<String>,
    webhook_type: String,
    auth_header: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WebhookResponse {
    webhook_id: String,
}

#[derive(Debug, Deserialize)]
pub struct HeliusWebhookPayload {
    pub timestamp: String,
    pub transaction_type: String,
    pub signature: String,
    pub accounts: Vec<String>,
    pub token_transfers: Option<Vec<TokenTransfer>>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct TokenTransfer {
    pub from: String,
    pub to: String,
    pub amount: u64,
    pub mint: String,
}

/// Helius API response types

#[derive(Debug, Deserialize)]
struct HeliusTokenMetadata {
    pub account: String,
    pub onchain_metadata: Option<OnChainMetadata>,
    pub offchain_metadata: Option<OffChainMetadata>,
    pub mint_authority: Option<String>,
    pub supply: Option<u64>,
    pub decimals: u8,
    pub token_program: String,
}

#[derive(Debug, Deserialize)]
struct OnChainMetadata {
    pub symbol: String,
    pub name: String,
    pub uri: String,
}

#[derive(Debug, Deserialize)]
struct OffChainMetadata {
    pub symbol: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub image: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenHolder {
    pub address: String,
    pub balance: u64,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenTransaction {
    pub signature: String,
    pub timestamp: i64,
    pub transaction_type: String,
    pub amount: Option<u64>,
    pub from: Option<String>,
    pub to: Option<String>,
}

/// Conversion implementations

impl From<HeliusTokenMetadata> for TokenMetadata {
    fn from(helius: HeliusTokenMetadata) -> Self {
        let (symbol, name, uri) = if let Some(onchain) = helius.onchain_metadata {
            (Some(onchain.symbol), Some(onchain.name), Some(onchain.uri))
        } else if let Some(offchain) = helius.offchain_metadata {
            (offchain.symbol, offchain.name, None)
        } else {
            (None, None, None)
        };

        TokenMetadata {
            address: helius.account,
            decimals: helius.decimals,
            supply: helius.supply.unwrap_or(0),
            symbol,
            name,
            uri,
            freeze_authority: None,
            mint_authority: helius.mint_authority,
            is_initialized: true,
        }
    }
}

impl From<HeliusWebhookPayload> for TokenEvent {
    fn from(payload: HeliusWebhookPayload) -> Self {
        TokenEvent {
            signature: payload.signature,
            timestamp: chrono::Utc::now(), // Parse from payload.timestamp
            event_type: payload.transaction_type,
            accounts: payload.accounts,
            token_transfers: payload.token_transfers.map(|transfers| {
                transfers.into_iter()
                    .map(|t| super::types::TokenTransferInfo {
                        from: t.from,
                        to: t.to,
                        amount: t.amount,
                        mint: t.mint,
                    })
                    .collect()
            }),
            metadata: payload.metadata,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConfigLoader;

    #[tokio::test]
    async fn test_helius_client_creation() {
        let config = ConfigLoader::new().without_env().create_default_config();
        let result = HeliusClient::new(&config.helius).await;

        assert!(result.is_ok());
        let client = result.unwrap();

        // Without API key, most operations should fail gracefully
        let metadata_result = client.get_token_metadata("test_mint").await;
        assert!(metadata_result.is_err());
    }

    #[test]
    fn test_rate_limiter() {
        let limiter = RateLimiter::new(10); // 10 requests per second
        // Basic creation test
        assert_eq!(limiter.limit, 10);
    }

    #[test]
    fn test_helius_stats() {
        let mut stats = HeliusStats::new();
        stats.total_requests = 100;
        stats.successful_requests = 95;
        stats.failed_requests = 5;

        assert_eq!(stats.success_rate(), 95.0);
    }
}