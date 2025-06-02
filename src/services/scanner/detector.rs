//! New token detection algorithms
//!
//! This module implements various strategies for detecting new tokens
//! including polling-based detection and event-driven discovery.

use std::sync::Arc;
use std::collections::{HashSet, HashMap};
use tokio::sync::RwLock;
use tracing::{debug, info, warn, instrument};
use chrono::{DateTime, Utc};

use crate::config::models::ScannerConfig;
use crate::core::result::AppResult;
use crate::core::error::AppError;
use crate::core::types::{TokenAddress, Timestamp};
use crate::services::solana::SolanaService;
use super::{TokenMetadata, FilterResult};

/// Detected token information
#[derive(Debug, Clone)]
pub struct DetectedToken {
    /// Token address
    pub address: TokenAddress,

    /// Token metadata
    pub metadata: TokenMetadata,

    /// Filter results
    pub filter_result: FilterResult,

    /// Detection timestamp
    pub detected_at: Timestamp,

    /// Detection source
    pub event_source: String,

    /// Detection latency in milliseconds
    pub detection_latency_ms: u64,
}

/// Token detector for discovering new tokens
#[derive(Debug)]
pub struct TokenDetector {
    /// Configuration
    config: Arc<ScannerConfig>,

    /// Solana service
    solana: Arc<SolanaService>,

    /// Known tokens cache
    known_tokens: Arc<RwLock<HashSet<String>>>,

    /// Detection strategies
    strategies: Vec<Box<dyn DetectionStrategy>>,

    /// Detection history
    detection_history: Arc<RwLock<DetectionHistory>>,
}

/// Detection history tracking
#[derive(Debug)]
struct DetectionHistory {
    /// Recent detections by source
    recent_detections: HashMap<String, Vec<DetectionRecord>>,

    /// Total detections by source
    total_by_source: HashMap<String, u64>,
}

/// Detection record
#[derive(Debug, Clone)]
struct DetectionRecord {
    token_address: String,
    detected_at: DateTime<Utc>,
    source: String,
}

/// Detection strategy trait
#[async_trait::async_trait]
trait DetectionStrategy: Send + Sync + std::fmt::Debug {
    /// Strategy name
    fn name(&self) -> &str;

    /// Detect new tokens
    async fn detect(&self, max_tokens: u32) -> AppResult<Vec<TokenAddress>>;
}

/// Program account scanning strategy
#[derive(Debug)]
struct ProgramAccountScanner {
    solana: Arc<SolanaService>,
    last_scan_slot: Arc<RwLock<u64>>,
}

#[async_trait::async_trait]
impl DetectionStrategy for ProgramAccountScanner {
    fn name(&self) -> &str {
        "program_account_scanner"
    }

    async fn detect(&self, max_tokens: u32) -> AppResult<Vec<TokenAddress>> {
        debug!("Running program account scanner");

        // Get current slot
        let current_slot = self.solana.get_slot().await?;

        // Get last scanned slot
        let last_slot = {
            let last = self.last_scan_slot.read().await;
            *last
        };

        if current_slot <= last_slot {
            return Ok(Vec::new());
        }

        // Get new token accounts created between slots
        let new_accounts = self.solana.get_program_accounts_in_slot_range(
            "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
            last_slot + 1,
            current_slot,
            max_tokens,
        ).await?;

        // Update last scan slot
        {
            let mut last = self.last_scan_slot.write().await;
            *last = current_slot;
        }

        // Convert to token addresses
        let tokens: Vec<TokenAddress> = new_accounts
            .into_iter()
            .filter_map(|account| {
                account.get("pubkey")
                    .and_then(|p| p.as_str())
                    .and_then(|addr| TokenAddress::new(addr.to_string()).ok())
            })
            .collect();

        debug!("Found {} new token accounts", tokens.len());
        Ok(tokens)
    }
}

/// Recent liquidity pool scanner
#[derive(Debug)]
struct LiquidityPoolScanner {
    solana: Arc<SolanaService>,
    dex_programs: Vec<String>,
    last_scan_time: Arc<RwLock<DateTime<Utc>>>,
}

#[async_trait::async_trait]
impl DetectionStrategy for LiquidityPoolScanner {
    fn name(&self) -> &str {
        "liquidity_pool_scanner"
    }

    async fn detect(&self, max_tokens: u32) -> AppResult<Vec<TokenAddress>> {
        debug!("Running liquidity pool scanner");

        let mut all_tokens = Vec::new();

        for dex_program in &self.dex_programs {
            match self.scan_dex_pools(dex_program, max_tokens).await {
                Ok(tokens) => {
                    all_tokens.extend(tokens);
                    if all_tokens.len() >= max_tokens as usize {
                        break;
                    }
                }
                Err(e) => {
                    warn!("Failed to scan DEX {}: {}", dex_program, e);
                }
            }
        }

        // Update last scan time
        {
            let mut last_time = self.last_scan_time.write().await;
            *last_time = Utc::now();
        }

        Ok(all_tokens)
    }
}

impl LiquidityPoolScanner {
    async fn scan_dex_pools(&self, dex_program: &str, max_pools: u32) -> AppResult<Vec<TokenAddress>> {
        // This would scan for new pools on specific DEX
        // For now, returning empty
        Ok(Vec::new())
    }
}

/// Helius API scanner
#[derive(Debug)]
struct HeliusApiScanner {
    solana: Arc<SolanaService>,
    last_cursor: Arc<RwLock<Option<String>>>,
}

#[async_trait::async_trait]
impl DetectionStrategy for HeliusApiScanner {
    fn name(&self) -> &str {
        "helius_api_scanner"
    }

    async fn detect(&self, max_tokens: u32) -> AppResult<Vec<TokenAddress>> {
        debug!("Running Helius API scanner");

        // Get last cursor for pagination
        let last_cursor = {
            let cursor = self.last_cursor.read().await;
            cursor.clone()
        };

        // Fetch new tokens from Helius
        let response = self.solana.fetch_helius_new_tokens(last_cursor, max_tokens).await?;

        // Extract tokens and update cursor
        let tokens = self.parse_helius_response(response).await?;

        Ok(tokens)
    }
}

impl HeliusApiScanner {
    async fn parse_helius_response(&self, response: serde_json::Value) -> AppResult<Vec<TokenAddress>> {
        let tokens = response.get("tokens")
            .and_then(|t| t.as_array())
            .ok_or_else(|| AppError::internal("Invalid Helius response format"))?;

        // Update cursor if present
        if let Some(cursor) = response.get("nextCursor").and_then(|c| c.as_str()) {
            let mut last_cursor = self.last_cursor.write().await;
            *last_cursor = Some(cursor.to_string());
        }

        // Parse token addresses
        let addresses: Vec<TokenAddress> = tokens
            .iter()
            .filter_map(|token| {
                token.get("address")
                    .and_then(|a| a.as_str())
                    .and_then(|addr| TokenAddress::new(addr.to_string()).ok())
            })
            .collect();

        Ok(addresses)
    }
}

impl TokenDetector {
    /// Create a new token detector
    pub async fn new(
        config: Arc<ScannerConfig>,
        solana: Arc<SolanaService>,
    ) -> AppResult<Self> {
        info!("Initializing token detector");

        // Initialize detection strategies
        let mut strategies: Vec<Box<dyn DetectionStrategy>> = Vec::new();

        // Add program account scanner
        strategies.push(Box::new(ProgramAccountScanner {
            solana: solana.clone(),
            last_scan_slot: Arc::new(RwLock::new(0)),
        }));

        // Add liquidity pool scanner
        strategies.push(Box::new(LiquidityPoolScanner {
            solana: solana.clone(),
            dex_programs: vec![
                "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8".to_string(), // Raydium
                "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc".to_string(), // Orca
            ],
            last_scan_time: Arc::new(RwLock::new(Utc::now())),
        }));

        // Add Helius scanner if API key is available
        if solana.has_helius_api_key() {
            strategies.push(Box::new(HeliusApiScanner {
                solana: solana.clone(),
                last_cursor: Arc::new(RwLock::new(None)),
            }));
        }

        // Load known tokens from database
        let known_tokens = Self::load_known_tokens(&config).await?;

        Ok(Self {
            config,
            solana,
            known_tokens: Arc::new(RwLock::new(known_tokens)),
            strategies,
            detection_history: Arc::new(RwLock::new(DetectionHistory {
                recent_detections: HashMap::new(),
                total_by_source: HashMap::new(),
            })),
        })
    }

    /// Load known tokens from database or cache
    async fn load_known_tokens(config: &ScannerConfig) -> AppResult<HashSet<String>> {
        // This would load from database
        // For now, including blacklisted tokens
        let mut known = HashSet::new();

        for token in &config.blacklisted_tokens {
            known.insert(token.clone());
        }

        Ok(known)
    }

    /// Detect new tokens using all strategies
    #[instrument(skip(self))]
    pub async fn detect_new_tokens(&self, max_tokens: u32) -> AppResult<Vec<TokenAddress>> {
        debug!("Starting token detection, max_tokens: {}", max_tokens);

        let mut all_tokens = Vec::new();
        let tokens_per_strategy = (max_tokens as usize / self.strategies.len()).max(1);

        // Run all strategies in parallel
        let mut detection_tasks = Vec::new();

        for strategy in &self.strategies {
            let strategy_name = strategy.name().to_string();
            let strategy_ref = strategy.as_ref();

            detection_tasks.push(async move {
                let result = strategy_ref.detect(tokens_per_strategy as u32).await;
                (strategy_name, result)
            });
        }

        let results = futures::future::join_all(detection_tasks).await;

        // Process results
        for (strategy_name, result) in results {
            match result {
                Ok(tokens) => {
                    info!("Strategy '{}' found {} tokens", strategy_name, tokens.len());

                    // Record detections
                    self.record_detections(&strategy_name, &tokens).await;

                    all_tokens.extend(tokens);
                }
                Err(e) => {
                    warn!("Strategy '{}' failed: {}", strategy_name, e);
                }
            }
        }

        // Filter out known tokens
        let new_tokens = self.filter_new_tokens(all_tokens).await?;

        // Update known tokens
        self.update_known_tokens(&new_tokens).await;

        info!("Total new tokens detected: {}", new_tokens.len());
        Ok(new_tokens)
    }

    /// Record detections for history tracking
    async fn record_detections(&self, source: &str, tokens: &[TokenAddress]) {
        let mut history = self.detection_history.write().await;

        let records: Vec<DetectionRecord> = tokens
            .iter()
            .map(|token| DetectionRecord {
                token_address: token.to_string(),
                detected_at: Utc::now(),
                source: source.to_string(),
            })
            .collect();

        // Update recent detections
        history.recent_detections
            .entry(source.to_string())
            .or_insert_with(Vec::new)
            .extend(records);

        // Update total count
        *history.total_by_source.entry(source.to_string()).or_insert(0) += tokens.len() as u64;

        // Keep only recent detections (last 1000)
        if let Some(recent) = history.recent_detections.get_mut(source) {
            if recent.len() > 1000 {
                recent.drain(0..recent.len() - 1000);
            }
        }
    }

    /// Filter out known tokens
    async fn filter_new_tokens(&self, tokens: Vec<TokenAddress>) -> AppResult<Vec<TokenAddress>> {
        let known_tokens = self.known_tokens.read().await;

        let new_tokens: Vec<TokenAddress> = tokens
            .into_iter()
            .filter(|token| !known_tokens.contains(token.as_str()))
            .collect();

        Ok(new_tokens)
    }

    /// Update known tokens cache
    async fn update_known_tokens(&self, new_tokens: &[TokenAddress]) {
        let mut known_tokens = self.known_tokens.write().await;

        for token in new_tokens {
            known_tokens.insert(token.to_string());
        }

        // Limit cache size to prevent unbounded growth
        if known_tokens.len() > 100000 {
            // This is a simple strategy - in production, we'd use LRU or similar
            known_tokens.clear();
            warn!("Known tokens cache cleared due to size limit");
        }
    }

    /// Get detection statistics
    pub async fn get_statistics(&self) -> DetectionStatistics {
        let history = self.detection_history.read().await;
        let known_tokens_count = self.known_tokens.read().await.len();

        DetectionStatistics {
            total_by_source: history.total_by_source.clone(),
            known_tokens_count,
            available_strategies: self.strategies.iter().map(|s| s.name().to_string()).collect(),
        }
    }

    /// Clear detection history
    pub async fn clear_history(&self) {
        let mut history = self.detection_history.write().await;
        history.recent_detections.clear();
        history.total_by_source.clear();

        info!("Detection history cleared");
    }
}

/// Detection statistics
#[derive(Debug, Clone)]
pub struct DetectionStatistics {
    /// Total detections by source
    pub total_by_source: HashMap<String, u64>,

    /// Number of known tokens in cache
    pub known_tokens_count: usize,

    /// Available detection strategies
    pub available_strategies: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConfigLoader;

    #[tokio::test]
    async fn test_token_detector_creation() {
        let config = ConfigLoader::new().without_env().create_default_config();
        let solana_service = Arc::new(
            SolanaService::new(&config.solana, &config.helius)
                .await
                .unwrap()
        );

        let detector = TokenDetector::new(
            Arc::new(config.scanner),
            solana_service,
        ).await;

        assert!(detector.is_ok());

        let detector = detector.unwrap();
        let stats = detector.get_statistics().await;

        assert!(!stats.available_strategies.is_empty());
    }
}