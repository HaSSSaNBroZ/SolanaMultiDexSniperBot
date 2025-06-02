//! Database infrastructure module
//!
//! This module provides database connectivity and management services
//! for PostgreSQL and Redis.

pub mod postgres;
pub mod redis;

// Re-export types
pub use postgres::{PostgresService, PostgresPool};
pub use redis::{RedisService, RedisPool};

use crate::config::AppConfig;
use crate::core::result::AppResult;
use crate::application::health::ComponentHealth;
use std::collections::HashMap;

/// Database service coordinator
#[derive(Debug, Clone)]
pub struct DatabaseService {
    /// PostgreSQL service
    pub postgres: PostgresService,
    /// Redis service
    pub redis: RedisService,
}

impl DatabaseService {
    /// Create a new database service
    pub async fn new(config: &AppConfig) -> AppResult<Self> {
        tracing::info!("🗄️  Initializing database services");

        // Initialize PostgreSQL
        let postgres = PostgresService::new(&config.database).await?;

        // Initialize Redis
        let redis = RedisService::new(&config.redis).await?;

        tracing::info!("✅ Database services initialized");

        Ok(Self {
            postgres,
            redis,
        })
    }

    /// Close all database connections
    pub async fn close(&self) -> AppResult<()> {
        tracing::info!("🔌 Closing database connections");

        // Close PostgreSQL connections
        self.postgres.close().await?;

        // Close Redis connections
        self.redis.close().await?;

        tracing::info!("✅ Database connections closed");
        Ok(())
    }

    /// Run database migrations
    pub async fn migrate(&self) -> AppResult<()> {
        tracing::info!("🔄 Running database migrations");
        self.postgres.migrate().await?;
        tracing::info!("✅ Database migrations completed");
        Ok(())
    }

    /// Health check for all database services
    pub async fn health_check(&self) -> HashMap<String, ComponentHealth> {
        let mut health_status = HashMap::new();

        // Check PostgreSQL health
        health_status.insert("postgres".to_string(), self.postgres.health_check().await);

        // Check Redis health
        health_status.insert("redis".to_string(), self.redis.health_check().await);

        health_status
    }

    /// Get database statistics
    pub async fn get_statistics(&self) -> AppResult<DatabaseStatistics> {
        let postgres_stats = self.postgres.get_statistics().await?;
        let redis_stats = self.redis.get_statistics().await?;

        Ok(DatabaseStatistics {
            postgres: postgres_stats,
            redis: redis_stats,
        })
    }
}

/// Database statistics
#[derive(Debug, Clone)]
pub struct DatabaseStatistics {
    /// PostgreSQL statistics
    pub postgres: postgres::PostgresStatistics,
    /// Redis statistics
    pub redis: redis::RedisStatistics,
}