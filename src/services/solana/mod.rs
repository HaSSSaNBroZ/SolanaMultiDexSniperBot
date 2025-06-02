//! Solana blockchain service module
//!
//! This module provides Solana RPC connectivity, Helius API integration,
//! token metadata retrieval, and optimized blockchain interactions.

pub mod rpc;
pub mod helius;
pub mod types;

// Re-export commonly used types
pub use rpc::{RpcClient, RpcConnection, RpcPool};
pub use helius::{HeliusClient, HeliusWebhook};
pub use types::{TokenMetadata, AccountInfo, TokenAccount, LiquidityPool,TokenEvent};

use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error, instrument};

use crate::config::AppConfig;
use crate::core::result::AppResult;
use crate::core::error::AppError;
use crate::application::health::{ComponentHealth, HealthStatus};

/// Solana blockchain service coordinator
#[derive(Debug)]
pub struct SolanaService {
    /// RPC client pool
    rpc_pool: Arc<RpcPool>,
    /// Helius API client
    helius_client: Arc<HeliusClient>,
    /// Service configuration
    config: Arc<AppConfig>,
    /// Connection statistics
    stats: Arc<RwLock<ConnectionStats>>,
}

impl SolanaService {
    /// Create a new Solana service
    #[instrument(skip(config))]
    pub async fn new(config: &AppConfig) -> AppResult<Self> {
        info!("⛓️  Initializing Solana blockchain service");

        // Create RPC connection pool
        let rpc_pool = Arc::new(RpcPool::new(&config.solana).await?);

        // Initialize Helius client
        let helius_client = Arc::new(HeliusClient::new(&config.helius).await?);

        // Initialize statistics
        let stats = Arc::new(RwLock::new(ConnectionStats::new()));

        info!("✅ Solana service initialized successfully");
        info!("📊 RPC endpoints configured: primary + {} fallbacks",
              config.solana.fallback_rpc_urls.len());

        Ok(Self {
            rpc_pool,
            helius_client,
            config: Arc::new(config.clone()),
            stats,
        })
    }

    /// Get RPC client from pool
    pub async fn get_rpc_client(&self) -> AppResult<RpcConnection> {
        self.rpc_pool.get_connection().await
    }

    /// Get dedicated RPC client for heavy operations
    pub async fn get_dedicated_rpc_client(&self) -> AppResult<Arc<RpcClient>> {
        self.rpc_pool.get_dedicated_client().await
    }

    /// Get Helius client
    pub fn helius(&self) -> &Arc<HeliusClient> {
        &self.helius_client
    }

    /// Get token metadata
    #[instrument(skip(self))]
    pub async fn get_token_metadata(&self, mint_address: &str) -> AppResult<TokenMetadata> {
        let start_time = std::time::Instant::now();

        // Try Helius first for enhanced metadata
        match self.helius_client.get_token_metadata(mint_address).await {
            Ok(metadata) => {
                let elapsed = start_time.elapsed();
                self.update_stats("helius_metadata", true, elapsed).await;
                return Ok(metadata);
            }
            Err(e) => {
                warn!("Helius metadata fetch failed, falling back to RPC: {}", e);
                self.update_stats("helius_metadata", false, start_time.elapsed()).await;
            }
        }

        // Fallback to direct RPC query
        let conn = self.get_rpc_client().await?;
        let metadata = conn.get_token_metadata(mint_address).await?;

        let elapsed = start_time.elapsed();
        self.update_stats("rpc_metadata", true, elapsed).await;

        Ok(metadata)
    }

    /// Get account information
    #[instrument(skip(self))]
    pub async fn get_account_info(&self, address: &str) -> AppResult<AccountInfo> {
        let conn = self.get_rpc_client().await?;
        conn.get_account_info(address).await
    }

    /// Get token account balance
    #[instrument(skip(self))]
    pub async fn get_token_balance(
        &self,
        wallet_address: &str,
        mint_address: &str
    ) -> AppResult<types::TokenBalance> {
        let conn = self.get_rpc_client().await?;
        conn.get_token_balance(wallet_address, mint_address).await
    }

    /// Get SOL balance
    #[instrument(skip(self))]
    pub async fn get_sol_balance(&self, address: &str) -> AppResult<f64> {
        let conn = self.get_rpc_client().await?;
        conn.get_sol_balance(address).await
    }

    /// Subscribe to token events via Helius webhooks
    #[instrument(skip(self))]
    pub async fn subscribe_to_token_events(
        &self,
        callback_url: &str
    ) -> AppResult<String> {
        self.helius_client.subscribe_to_webhooks(callback_url).await
    }

    /// Health check for Solana services
    pub async fn health_check(&self) -> std::collections::HashMap<String, ComponentHealth> {
        let mut health_status = std::collections::HashMap::new();

        // Check RPC health
        health_status.insert("solana_rpc".to_string(), self.check_rpc_health().await);

        // Check Helius health
        health_status.insert("helius_api".to_string(), self.check_helius_health().await);

        health_status
    }

    /// Check RPC connection health
    async fn check_rpc_health(&self) -> ComponentHealth {
        let mut component = ComponentHealth::new("solana_rpc".to_string(), true);
        let start_time = std::time::Instant::now();

        match self.rpc_pool.health_check().await {
            Ok(info) => {
                let response_time = start_time.elapsed().as_millis() as u64;
                component.mark_healthy(
                    Some(format!("RPC healthy: {}", info)),
                    Some(response_time)
                );
            }
            Err(e) => {
                component.mark_unhealthy(format!("RPC health check failed: {}", e));
            }
        }

        component
    }

    /// Check Helius API health
    async fn check_helius_health(&self) -> ComponentHealth {
        let mut component = ComponentHealth::new("helius_api".to_string(), false);
        let start_time = std::time::Instant::now();

        match self.helius_client.health_check().await {
            Ok(info) => {
                let response_time = start_time.elapsed().as_millis() as u64;
                component.mark_healthy(
                    Some(format!("Helius API healthy: {}", info)),
                    Some(response_time)
                );
            }
            Err(e) => {
                if self.config.helius.api_key.is_empty() {
                    component.mark_degraded(
                        "Helius API key not configured".to_string(),
                        None
                    );
                } else {
                    component.mark_unhealthy(format!("Helius health check failed: {}", e));
                }
            }
        }

        component
    }

    /// Update connection statistics
    async fn update_stats(&self, operation: &str, success: bool, duration: std::time::Duration) {
        let mut stats = self.stats.write().await;
        stats.record_operation(operation, success, duration);
    }

    /// Get connection statistics
    pub async fn get_statistics(&self) -> ConnectionStats {
        self.stats.read().await.clone()
    }

    /// Shutdown the service
    pub async fn shutdown(&self) -> AppResult<()> {
        info!("🛑 Shutting down Solana service");

        self.rpc_pool.close().await?;
        self.helius_client.close().await?;

        info!("✅ Solana service shut down successfully");
        Ok(())
    }
}

/// Connection statistics
#[derive(Debug, Clone)]
pub struct ConnectionStats {
    /// Total operations
    pub total_operations: u64,
    /// Successful operations
    pub successful_operations: u64,
    /// Failed operations
    pub failed_operations: u64,
    /// Average response time in milliseconds
    pub avg_response_time_ms: f64,
    /// Operations by type
    pub operations_by_type: std::collections::HashMap<String, OperationStats>,
    /// Service start time
    pub started_at: chrono::DateTime<chrono::Utc>,
}

impl ConnectionStats {
    fn new() -> Self {
        Self {
            total_operations: 0,
            successful_operations: 0,
            failed_operations: 0,
            avg_response_time_ms: 0.0,
            operations_by_type: std::collections::HashMap::new(),
            started_at: chrono::Utc::now(),
        }
    }

    fn record_operation(&mut self, operation: &str, success: bool, duration: std::time::Duration) {
        self.total_operations += 1;

        if success {
            self.successful_operations += 1;
        } else {
            self.failed_operations += 1;
        }

        // Update average response time
        let duration_ms = duration.as_millis() as f64;
        self.avg_response_time_ms =
            (self.avg_response_time_ms * (self.total_operations - 1) as f64 + duration_ms)
                / self.total_operations as f64;

        // Update operation-specific stats
        let op_stats = self.operations_by_type
            .entry(operation.to_string())
            .or_insert(OperationStats::new());
        op_stats.record(success, duration);
    }
}

/// Operation-specific statistics
#[derive(Debug, Clone)]
pub struct OperationStats {
    pub total: u64,
    pub successful: u64,
    pub failed: u64,
    pub avg_duration_ms: f64,
    pub min_duration_ms: f64,
    pub max_duration_ms: f64,
}

impl OperationStats {
    fn new() -> Self {
        Self {
            total: 0,
            successful: 0,
            failed: 0,
            avg_duration_ms: 0.0,
            min_duration_ms: f64::MAX,
            max_duration_ms: 0.0,
        }
    }

    fn record(&mut self, success: bool, duration: std::time::Duration) {
        self.total += 1;

        if success {
            self.successful += 1;
        } else {
            self.failed += 1;
        }

        let duration_ms = duration.as_millis() as f64;

        // Update average
        self.avg_duration_ms =
            (self.avg_duration_ms * (self.total - 1) as f64 + duration_ms)
                / self.total as f64;

        // Update min/max
        self.min_duration_ms = self.min_duration_ms.min(duration_ms);
        self.max_duration_ms = self.max_duration_ms.max(duration_ms);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConfigLoader;

    #[tokio::test]
    async fn test_solana_service_creation() {
        let config = ConfigLoader::new().without_env().create_default_config();
        let result = SolanaService::new(&config).await;

        // Should succeed even without actual RPC connection in test
        assert!(result.is_ok());
    }

    #[test]
    fn test_connection_stats() {
        let mut stats = ConnectionStats::new();

        stats.record_operation("test_op", true, std::time::Duration::from_millis(100));
        stats.record_operation("test_op", false, std::time::Duration::from_millis(200));

        assert_eq!(stats.total_operations, 2);
        assert_eq!(stats.successful_operations, 1);
        assert_eq!(stats.failed_operations, 1);
        assert_eq!(stats.avg_response_time_ms, 150.0);
    }
}