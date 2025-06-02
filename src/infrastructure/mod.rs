//! Infrastructure layer module
//!
//! This module contains all infrastructure-related services including
//! database connections, caching, monitoring, and security services.

pub mod database;
pub mod monitoring;
pub mod security;

// Re-export commonly used types
pub use database::{DatabaseService, PostgresPool, RedisPool};
pub use monitoring::{MetricsService, TracingService, HealthTracker};
pub use security::{EncryptionService, AuthenticationService};

/// Infrastructure service collection
#[derive(Debug, Clone)]
pub struct InfrastructureServices {
    /// Database service
    pub database: DatabaseService,
    /// Metrics service
    pub metrics: MetricsService,
    /// Tracing service
    pub tracing: TracingService,
    /// Encryption service
    pub encryption: EncryptionService,
    /// Authentication service
    pub authentication: AuthenticationService,
}

impl InfrastructureServices {
    /// Initialize all infrastructure services
    pub async fn initialize(config: &crate::config::AppConfig) -> crate::core::result::AppResult<Self> {
        tracing::info!("🏗️  Initializing infrastructure services");

        // Initialize database connections
        let database = DatabaseService::new(config).await?;

        // Initialize monitoring services
        let metrics = MetricsService::new(config)?;
        let tracing_service = TracingService::new(config)?;

        // Initialize security services
        let encryption = EncryptionService::new(config)?;
        let authentication = AuthenticationService::new(config)?;

        tracing::info!("✅ Infrastructure services initialized successfully");

        Ok(Self {
            database,
            metrics,
            tracing: tracing_service,
            encryption,
            authentication,
        })
    }

    /// Graceful shutdown of all services
    pub async fn shutdown(&self) -> crate::core::result::AppResult<()> {
        tracing::info!("🛑 Shutting down infrastructure services");

        // Close database connections
        self.database.close().await?;

        // Flush metrics
        self.metrics.flush().await?;

        tracing::info!("✅ Infrastructure services shut down successfully");
        Ok(())
    }

    /// Health check for all infrastructure services
    pub async fn health_check(&self) -> std::collections::HashMap<String, crate::application::health::ComponentHealth> {
        let mut health_status = std::collections::HashMap::new();

        // Check database health
        health_status.extend(self.database.health_check().await);

        // Check metrics health
        health_status.insert("metrics".to_string(), self.metrics.health_check().await);

        health_status
    }
}