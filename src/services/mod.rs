//! Services layer module
//!
//! This module contains all business services including blockchain integration,
//! trading execution, risk management, and external API integrations.

pub mod solana;

// Re-export commonly used types
pub use solana::{SolanaService, HeliusClient, TokenMetadata, RpcClient};

use crate::config::AppConfig;
use crate::core::result::AppResult;
use std::sync::Arc;

/// Services collection for dependency injection
#[derive(Clone)]
pub struct ServiceContainer {
    /// Solana blockchain service
    pub solana: Arc<solana::SolanaService>,
}

impl ServiceContainer {
    /// Initialize all services
    pub async fn initialize(config: &AppConfig) -> AppResult<Self> {
        tracing::info!("🚀 Initializing service container");

        // Initialize Solana service
        let solana = Arc::new(solana::SolanaService::new(config).await?);

        tracing::info!("✅ Service container initialized successfully");

        Ok(Self { solana })
    }

    /// Graceful shutdown of all services
    pub async fn shutdown(&self) -> AppResult<()> {
        tracing::info!("🛑 Shutting down services");

        // Shutdown Solana connections
        self.solana.shutdown().await?;

        tracing::info!("✅ Services shut down successfully");
        Ok(())
    }

    /// Health check for all services
    pub async fn health_check(&self) -> std::collections::HashMap<String, crate::application::health::ComponentHealth> {
        let mut health_status = std::collections::HashMap::new();

        // Check Solana service health
        health_status.extend(self.solana.health_check().await);

        health_status
    }
}