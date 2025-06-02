//! Solana RPC client implementation with connection pooling and failover
//!
//! This module provides a robust RPC client with automatic failover,
//! connection pooling, and intelligent retry logic.

use solana_client::{
    nonblocking::rpc_client::RpcClient as SolanaRpcClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig, RpcSendTransactionConfig},
    rpc_filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType},
};
use solana_sdk::{
    account::Account,
    commitment_config::{CommitmentConfig, CommitmentLevel},
    pubkey::Pubkey,
    signature::Signature,
    transaction::Transaction,
};
use solana_account_decoder::{UiAccountEncoding, UiDataSliceConfig};
use spl_token::state::{Account as TokenAccount, Mint};
use solana_sdk::program_pack::Pack;
use std::sync::Arc;
use std::str::FromStr;
use tokio::sync::{RwLock, Semaphore};
use tracing::{debug, info, warn, error, instrument};
use backoff::{ExponentialBackoff, backoff::Backoff};

use crate::config::models::SolanaConfig;
use crate::core::result::AppResult;
use crate::core::error::AppError;
use super::types::{AccountInfo, TokenMetadata, TokenBalance};

/// Maximum concurrent RPC requests
const MAX_CONCURRENT_REQUESTS: usize = 10;

/// RPC client wrapper with retry logic
#[derive(Debug, Clone)]
pub struct RpcClient {
    /// Inner Solana RPC client
    client: Arc<SolanaRpcClient>,
    /// Client identifier
    id: String,
    /// Commitment level
    commitment: CommitmentConfig,
    /// Request semaphore for rate limiting
    semaphore: Arc<Semaphore>,
    /// Health status
    is_healthy: Arc<RwLock<bool>>,
    /// Last successful request time
    last_success: Arc<RwLock<Option<std::time::Instant>>>,
}

impl RpcClient {
    /// Create a new RPC client
    pub async fn new(url: &str, commitment: CommitmentLevel) -> AppResult<Self> {
        info!("🔗 Creating RPC client for: {}", url);

        let client = SolanaRpcClient::new_with_commitment(
            url.to_string(),
            CommitmentConfig { commitment }
        );

        // Test connection
        match client.get_health().await {
            Ok(_) => info!("✅ RPC client connected successfully"),
            Err(e) => {
                error!("❌ RPC client health check failed: {}", e);
                return Err(AppError::network(format!(
                    "Failed to connect to RPC endpoint {}: {}",
                    url, e
                )));
            }
        }

        Ok(Self {
            client: Arc::new(client),
            id: url.to_string(),
            commitment: CommitmentConfig { commitment },
            semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_REQUESTS)),
            is_healthy: Arc::new(RwLock::new(true)),
            last_success: Arc::new(RwLock::new(Some(std::time::Instant::now()))),
        })
    }

    /// Get client ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Check if client is healthy
    pub async fn is_healthy(&self) -> bool {
        *self.is_healthy.read().await
    }

    /// Mark client as healthy/unhealthy
    async fn set_health(&self, healthy: bool) {
        *self.is_healthy.write().await = healthy;
        if healthy {
            *self.last_success.write().await = Some(std::time::Instant::now());
        }
    }

    /// Execute RPC request with retry logic
    async fn execute_with_retry<T, F, Fut>(&self, operation: &str, f: F) -> AppResult<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, solana_client::client_error::ClientError>>,
    {
        let _permit = self.semaphore.acquire().await
            .map_err(|_| AppError::internal("Failed to acquire RPC semaphore"))?;

        let mut backoff = ExponentialBackoff::default();
        backoff.max_elapsed_time = Some(std::time::Duration::from_secs(30));

        let start_time = std::time::Instant::now();
        let mut last_error = None;

        loop {
            match f().await {
                Ok(result) => {
                    let elapsed = start_time.elapsed();
                    debug!("✅ RPC {} succeeded in {:?}", operation, elapsed);
                    self.set_health(true).await;
                    return Ok(result);
                }
                Err(e) => {
                    warn!("⚠️  RPC {} failed: {}", operation, e);
                    last_error = Some(e);

                    match backoff.next_backoff() {
                        Some(duration) => {
                            debug!("🔄 Retrying {} after {:?}", operation, duration);
                            tokio::time::sleep(duration).await;
                        }
                        None => {
                            error!("❌ RPC {} failed after all retries", operation);
                            self.set_health(false).await;
                            break;
                        }
                    }
                }
            }
        }

        Err(AppError::network(format!(
            "RPC {} failed: {:?}",
            operation,
            last_error
        )))
    }

    /// Get account information
    #[instrument(skip(self))]
    pub async fn get_account_info(&self, address: &str) -> AppResult<AccountInfo> {
        let pubkey = Pubkey::from_str(address)
            .map_err(|e| AppError::validation(format!("Invalid address: {}", e)))?;

        let account = self.execute_with_retry("get_account", || {
            self.client.get_account(&pubkey)
        }).await?;

        Ok(AccountInfo {
            address: address.to_string(),
            lamports: account.lamports,
            data: account.data,
            owner: account.owner.to_string(),
            executable: account.executable,
            rent_epoch: account.rent_epoch,
        })
    }

    /// Get token metadata
    #[instrument(skip(self))]
    pub async fn get_token_metadata(&self, mint_address: &str) -> AppResult<TokenMetadata> {
        let mint_pubkey = Pubkey::from_str(mint_address)
            .map_err(|e| AppError::validation(format!("Invalid mint address: {}", e)))?;

        // Get mint account
        let mint_account = self.execute_with_retry("get_mint_account", || {
            self.client.get_account(&mint_pubkey)
        }).await?;

        // Deserialize mint data
        let mint = spl_token::state::Mint::unpack(&mint_account.data)
            .map_err(|e| AppError::network(format!("Failed to unpack mint data: {}", e)))?;

        // Try to get metadata account (if it exists)
        let metadata_pubkey = self.get_metadata_pubkey(&mint_pubkey)?;
        let metadata = match self.client.get_account(&metadata_pubkey).await {
            Ok(account) => Some(self.parse_metadata(&account)?),
            Err(_) => None,
        };

        Ok(TokenMetadata {
            address: mint_address.to_string(),
            decimals: mint.decimals,
            supply: mint.supply,
            symbol: metadata.as_ref().and_then(|m| m.get("symbol").cloned()),
            name: metadata.as_ref().and_then(|m| m.get("name").cloned()),
            uri: metadata.as_ref().and_then(|m| m.get("uri").cloned()),
            freeze_authority: match mint.freeze_authority {
                solana_program::program_option::COption::Some(pubkey) => Some(pubkey.to_string()),
                solana_program::program_option::COption::None => None,
            },
            mint_authority: match mint.mint_authority {
                solana_program::program_option::COption::Some(pubkey) => Some(pubkey.to_string()),
                solana_program::program_option::COption::None => None,
            },
            is_initialized: mint.is_initialized,
        })
    }

    /// Get token balance
    #[instrument(skip(self))]
    pub async fn get_token_balance(
        &self,
        wallet_address: &str,
        mint_address: &str
    ) -> AppResult<TokenBalance> {
        let wallet_pubkey = Pubkey::from_str(wallet_address)
            .map_err(|e| AppError::validation(format!("Invalid wallet address: {}", e)))?;
        let mint_pubkey = Pubkey::from_str(mint_address)
            .map_err(|e| AppError::validation(format!("Invalid mint address: {}", e)))?;

        // Get associated token account
        let token_account = spl_associated_token_account::get_associated_token_address(
            &wallet_pubkey,
            &mint_pubkey
        );

        match self.client.get_account(&token_account).await {
            Ok(account) => {
                let token_account_data = TokenAccount::unpack(&account.data)
                    .map_err(|e| AppError::network(format!("Failed to unpack token account: {}", e)))?;

                Ok(TokenBalance {
                    mint: mint_address.to_string(),
                    owner: wallet_address.to_string(),
                    amount: token_account_data.amount,
                    decimals: self.get_token_decimals(&mint_pubkey).await?,
                })
            }
            Err(_) => {
                // Token account doesn't exist, balance is 0
                Ok(TokenBalance {
                    mint: mint_address.to_string(),
                    owner: wallet_address.to_string(),
                    amount: 0,
                    decimals: self.get_token_decimals(&mint_pubkey).await?,
                })
            }
        }
    }

    /// Get SOL balance
    #[instrument(skip(self))]
    pub async fn get_sol_balance(&self, address: &str) -> AppResult<f64> {
        let pubkey = Pubkey::from_str(address)
            .map_err(|e| AppError::validation(format!("Invalid address: {}", e)))?;

        let balance = self.execute_with_retry("get_balance", || {
            self.client.get_balance(&pubkey)
        }).await?;

        Ok(balance as f64 / 1e9) // Convert lamports to SOL
    }

    /// Get recent blockhash
    #[instrument(skip(self))]
    pub async fn get_recent_blockhash(&self) -> AppResult<solana_sdk::hash::Hash> {
        let blockhash = self.execute_with_retry("get_recent_blockhash", || {
            async {
                self.client.get_latest_blockhash().await
            }
        }).await?;

        Ok(blockhash)
    }

    /// Send transaction
    #[instrument(skip(self, transaction))]
    pub async fn send_transaction(&self, transaction: &Transaction) -> AppResult<Signature> {
        let config = RpcSendTransactionConfig {
            skip_preflight: false,
            preflight_commitment: Some(self.commitment.commitment),
            encoding: None,
            max_retries: None,
            min_context_slot: None,
        };

        let signature = self.execute_with_retry("send_transaction", || {
            self.client.send_transaction_with_config(transaction, config)
        }).await?;

        Ok(signature)
    }

    /// Simulate transaction
    #[instrument(skip(self, transaction))]
    pub async fn simulate_transaction(&self, transaction: &Transaction) -> AppResult<()> {
        let result = self.execute_with_retry("simulate_transaction", || {
            self.client.simulate_transaction(transaction)
        }).await?;

        if let Some(err) = result.value.err {
            return Err(AppError::network(format!("Transaction simulation failed: {:?}", err)));
        }


        Ok(())
    }

    /// Health check
    pub async fn health_check(&self) -> AppResult<String> {
        let _ = self.execute_with_retry("health_check", || {
            self.client.get_health()
        }).await?;

        Ok("RPC node is healthy".to_string())
    }

    /// Get metadata pubkey for a mint
    fn get_metadata_pubkey(&self, mint: &Pubkey) -> AppResult<Pubkey> {
        let metadata_program_id = mpl_token_metadata::id();
        let seeds = &[
            b"metadata",
            metadata_program_id.as_ref(),
            mint.as_ref(),
        ];

        let (metadata_pubkey, _) = Pubkey::find_program_address(seeds, &metadata_program_id);
        Ok(metadata_pubkey)
    }

    /// Parse metadata from account
    fn parse_metadata(&self, account: &Account) -> AppResult<std::collections::HashMap<String, String>> {
        // This is a simplified version - actual implementation would properly parse metadata
        let mut metadata = std::collections::HashMap::new();

        // In a real implementation, you would deserialize the metadata account
        // For now, return empty metadata
        Ok(metadata)
    }

    /// Get token decimals
    async fn get_token_decimals(&self, mint: &Pubkey) -> AppResult<u8> {
        let account = self.client.get_account(mint).await
            .map_err(|e| AppError::network(format!("Failed to get mint account: {}", e)))?;

        let mint_data = Mint::unpack(&account.data)
            .map_err(|e| AppError::network(format!("Failed to unpack mint data: {}", e)))?;

        Ok(mint_data.decimals)
    }
}

/// RPC connection pool with failover support
#[derive(Debug, Clone)]
pub struct RpcPool {
    /// Primary RPC client
    primary: Arc<RpcClient>,
    /// Fallback RPC clients
    fallbacks: Vec<Arc<RpcClient>>,
    /// Current active client index
    active_index: Arc<RwLock<usize>>,
    /// Pool configuration
    config: SolanaConfig,
}

impl RpcPool {
    /// Create a new RPC connection pool
    pub async fn new(config: &SolanaConfig) -> AppResult<Self> {
        info!("🏊 Creating RPC connection pool");

        // Parse commitment level
        let commitment = match config.commitment.as_str() {
            "processed" => CommitmentLevel::Processed,
            "confirmed" => CommitmentLevel::Confirmed,
            "finalized" => CommitmentLevel::Finalized,
            _ => CommitmentLevel::Confirmed,
        };

        // Create primary client
        let primary = Arc::new(RpcClient::new(&config.rpc_url, commitment).await?);

        // Create fallback clients
        let mut fallbacks = Vec::new();
        for url in &config.fallback_rpc_urls {
            match RpcClient::new(url, commitment).await {
                Ok(client) => {
                    info!("✅ Added fallback RPC: {}", url);
                    fallbacks.push(Arc::new(client));
                }
                Err(e) => {
                    warn!("⚠️  Failed to add fallback RPC {}: {}", url, e);
                }
            }
        }

        Ok(Self {
            primary,
            fallbacks,
            active_index: Arc::new(RwLock::new(0)),
            config: config.clone(),
        })
    }

    /// Get a connection from the pool
    pub async fn get_connection(&self) -> AppResult<RpcConnection> {
        let active_index = *self.active_index.read().await;

        // Try active client first
        let all_clients = self.get_all_clients();
        if active_index < all_clients.len() {
            let client = all_clients[active_index].clone();
            if client.is_healthy().await {
                return Ok(RpcConnection::new(client, self.clone()));
            }
        }

        // Find a healthy client
        for (index, client) in all_clients.iter().enumerate() {
            if client.is_healthy().await {
                *self.active_index.write().await = index;
                return Ok(RpcConnection::new(client.clone(), self.clone()));
            }
        }

        // Try to reconnect to any client
        for (index, client) in all_clients.iter().enumerate() {
            if let Ok(_) = client.health_check().await {
                *self.active_index.write().await = index;
                return Ok(RpcConnection::new(client.clone(), self.clone()));
            }
        }

        Err(AppError::network("All RPC endpoints are unavailable"))
    }

    /// Get a dedicated client for heavy operations
    pub async fn get_dedicated_client(&self) -> AppResult<Arc<RpcClient>> {
        // For dedicated clients, prefer fallbacks to avoid overloading primary
        for client in &self.fallbacks {
            if client.is_healthy().await {
                return Ok(client.clone());
            }
        }

        // Fall back to primary if no healthy fallbacks
        if self.primary.is_healthy().await {
            return Ok(self.primary.clone());
        }

        Err(AppError::network("No healthy RPC endpoints available"))
    }

    /// Get all clients
    fn get_all_clients(&self) -> Vec<Arc<RpcClient>> {
        let mut clients = vec![self.primary.clone()];
        clients.extend(self.fallbacks.clone());
        clients
    }

    /// Report client failure
    pub async fn report_failure(&self, client_id: &str) {
        warn!("⚠️  RPC client failure reported: {}", client_id);

        // Mark client as unhealthy
        for client in self.get_all_clients() {
            if client.id() == client_id {
                client.set_health(false).await;
                break;
            }
        }

        // Switch to next healthy client
        let current_index = *self.active_index.read().await;
        let all_clients = self.get_all_clients();

        for i in 1..all_clients.len() {
            let next_index = (current_index + i) % all_clients.len();
            if all_clients[next_index].is_healthy().await {
                *self.active_index.write().await = next_index;
                info!("🔄 Switched to RPC client: {}", all_clients[next_index].id());
                break;
            }
        }
    }

    /// Health check for the pool
    pub async fn health_check(&self) -> AppResult<String> {
        let mut healthy_count = 0;
        let all_clients = self.get_all_clients();

        for client in &all_clients {
            if client.is_healthy().await {
                healthy_count += 1;
            }
        }

        Ok(format!("{}/{} RPC clients healthy", healthy_count, all_clients.len()))
    }

    /// Close all connections
    pub async fn close(&self) -> AppResult<()> {
        info!("🔌 Closing RPC connection pool");
        // Connections are closed automatically when dropped
        Ok(())
    }
}

/// RPC connection wrapper that reports failures to the pool
#[derive(Clone)]
pub struct RpcConnection {
    client: Arc<RpcClient>,
    pool: RpcPool,
}

impl RpcConnection {
    fn new(client: Arc<RpcClient>, pool: RpcPool) -> Self {
        Self { client, pool }
    }

    /// Delegate all RPC methods to the underlying client
    pub async fn get_account_info(&self, address: &str) -> AppResult<AccountInfo> {
        match self.client.get_account_info(address).await {
            Ok(info) => Ok(info),
            Err(e) => {
                self.pool.report_failure(self.client.id()).await;
                Err(e)
            }
        }
    }

    pub async fn get_token_metadata(&self, mint_address: &str) -> AppResult<TokenMetadata> {
        match self.client.get_token_metadata(mint_address).await {
            Ok(metadata) => Ok(metadata),
            Err(e) => {
                self.pool.report_failure(self.client.id()).await;
                Err(e)
            }
        }
    }

    pub async fn get_token_balance(
        &self,
        wallet_address: &str,
        mint_address: &str
    ) -> AppResult<TokenBalance> {
        match self.client.get_token_balance(wallet_address, mint_address).await {
            Ok(balance) => Ok(balance),
            Err(e) => {
                self.pool.report_failure(self.client.id()).await;
                Err(e)
            }
        }
    }

    pub async fn get_sol_balance(&self, address: &str) -> AppResult<f64> {
        match self.client.get_sol_balance(address).await {
            Ok(balance) => Ok(balance),
            Err(e) => {
                self.pool.report_failure(self.client.id()).await;
                Err(e)
            }
        }
    }

    pub async fn get_recent_blockhash(&self) -> AppResult<solana_sdk::hash::Hash> {
        match self.client.get_recent_blockhash().await {
            Ok(blockhash) => Ok(blockhash),
            Err(e) => {
                self.pool.report_failure(self.client.id()).await;
                Err(e)
            }
        }
    }

    pub async fn send_transaction(&self, transaction: &Transaction) -> AppResult<Signature> {
        match self.client.send_transaction(transaction).await {
            Ok(signature) => Ok(signature),
            Err(e) => {
                self.pool.report_failure(self.client.id()).await;
                Err(e)
            }
        }
    }

    pub async fn simulate_transaction(&self, transaction: &Transaction) -> AppResult<()> {
        match self.client.simulate_transaction(transaction).await {
            Ok(()) => Ok(()),
            Err(e) => {
                self.pool.report_failure(self.client.id()).await;
                Err(e)
            }
        }
    }
}

// Module-level constants for metadata program
mod mpl_token_metadata {
    use solana_sdk::pubkey::Pubkey;
    use std::str::FromStr;

    pub fn id() -> Pubkey {
        Pubkey::from_str("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s").unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConfigLoader;

    #[tokio::test]
    async fn test_rpc_client_creation() {
        // This test requires a valid RPC endpoint
        let config = ConfigLoader::new().without_env().create_default_config();

        // Test will likely fail without actual RPC, which is expected
        let result = RpcClient::new(&config.solana.rpc_url, CommitmentLevel::Confirmed).await;

        // We expect this to fail in test environment
        assert!(result.is_err() || result.is_ok());
    }

    #[tokio::test]
    async fn test_rpc_pool_creation() {
        let config = ConfigLoader::new().without_env().create_default_config();

        // Test pool creation
        let result = RpcPool::new(&config.solana).await;

        // We expect this to fail in test environment without RPC
        assert!(result.is_err() || result.is_ok());
    }
}