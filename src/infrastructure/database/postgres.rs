//! PostgreSQL database service implementation
//!
//! This module provides PostgreSQL connectivity, query execution,
//! migration management, and connection pooling.

use sqlx::{PgPool, Row, Executor, migrate::MigrateDatabase};
use sqlx::postgres::{PgPoolOptions, PgConnectOptions};
use std::time::Duration;
use tracing::{info, warn, error, debug, instrument};
use anyhow::Context;

use crate::config::models::DatabaseConfig;
use crate::core::result::AppResult;
use crate::core::error::AppError;
use crate::application::health::{ComponentHealth, HealthStatus};

/// PostgreSQL connection pool type alias
pub type PostgresPool = PgPool;

/// PostgreSQL service
#[derive(Debug, Clone)]
pub struct PostgresService {
    /// Connection pool
    pool: PgPool,
    /// Database configuration
    config: DatabaseConfig,
}

impl PostgresService {
    /// Create a new PostgreSQL service with connection pool
    #[instrument(skip(config))]
    pub async fn new(config: &DatabaseConfig) -> AppResult<Self> {
        info!("🐘 Initializing PostgreSQL connection pool");

        // Validate configuration
        if config.url.is_empty() {
            return Err(AppError::database(
                "PostgreSQL URL is required".to_string(),
                "connection"
            ));
        }

        // Parse connection options
        let connect_options = config.url.parse::<PgConnectOptions>()
            .context("Failed to parse PostgreSQL connection string")?
            .log_statements(tracing::log::LevelFilter::Debug)
            .log_slow_statements(
                tracing::log::LevelFilter::Warn,
                Duration::from_millis(config.slow_query_threshold_ms)
            );

        // Create connection pool
        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .acquire_timeout(Duration::from_millis(config.connection_timeout_ms))
            .idle_timeout(Duration::from_millis(config.idle_timeout_ms))
            .max_lifetime(Duration::from_secs(1800)) // 30 minutes
            .test_before_acquire(true)
            .connect_with(connect_options)
            .await
            .map_err(|e| AppError::database(
                format!("Failed to create PostgreSQL connection pool: {}", e),
                "connection_pool"
            ))?;

        info!("✅ PostgreSQL connection pool established");
        info!("📊 Pool configuration: max={}, min={}",
              config.max_connections, config.min_connections);

        // Run migrations if enabled
        let service = Self {
            pool,
            config: config.clone(),
        };

        if config.auto_migrate {
            service.migrate().await?;
        }

        Ok(service)
    }

    /// Get the connection pool
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Run database migrations
    #[instrument(skip(self))]
    pub async fn migrate(&self) -> AppResult<()> {
        info!("🔄 Running PostgreSQL migrations");

        let migration_results = sqlx::migrate!(&self.config.migration_path)
            .run(&self.pool)
            .await
            .map_err(|e| AppError::database(
                format!("Migration failed: {}", e),
                "migration"
            ))?;

        info!("✅ PostgreSQL migrations completed successfully");
        debug!("Migration results: {:?}", migration_results);

        Ok(())
    }

    /// Close all connections in the pool
    pub async fn close(&self) -> AppResult<()> {
        info!("🔌 Closing PostgreSQL connection pool");
        self.pool.close().await;
        info!("✅ PostgreSQL connection pool closed");
        Ok(())
    }

    /// Execute a health check query
    #[instrument(skip(self))]
    pub async fn health_check(&self) -> ComponentHealth {
        let mut component = ComponentHealth::new("postgres".to_string(), true);
        let start_time = std::time::Instant::now();

        match self.execute_health_check_query().await {
            Ok(_) => {
                let response_time = start_time.elapsed().as_millis() as u64;
                component.mark_healthy(
                    Some("Database connection healthy".to_string()),
                    Some(response_time)
                );
            }
            Err(e) => {
                component.mark_unhealthy(format!("Database health check failed: {}", e));
            }
        }

        component
    }

    /// Execute the actual health check query
    async fn execute_health_check_query(&self) -> AppResult<()> {
        let result = sqlx::query("SELECT 1 as health_check")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::database(
                format!("Health check query failed: {}", e),
                "health_check"
            ))?;

        let health_value: i32 = result.try_get("health_check")
            .map_err(|e| AppError::database(
                format!("Failed to parse health check result: {}", e),
                "health_check"
            ))?;

        if health_value != 1 {
            return Err(AppError::database(
                "Health check returned unexpected value".to_string(),
                "health_check"
            ));
        }

        Ok(())
    }

    /// Get database statistics
    #[instrument(skip(self))]
    pub async fn get_statistics(&self) -> AppResult<PostgresStatistics> {
        debug!("📊 Collecting PostgreSQL statistics");

        // Get connection pool statistics
        let pool_size = self.pool.size();
        let idle_connections = self.pool.num_idle();

        // Get database size
        let db_size_result = sqlx::query("SELECT pg_size_pretty(pg_database_size(current_database())) as size")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::database(
                format!("Failed to get database size: {}", e),
                "statistics"
            ))?;

        let database_size: String = db_size_result.try_get("size")
            .map_err(|e| AppError::database(
                format!("Failed to parse database size: {}", e),
                "statistics"
            ))?;

        // Get table statistics
        let table_stats = self.get_table_statistics().await?;

        // Get activity statistics
        let activity_stats = self.get_activity_statistics().await?;

        Ok(PostgresStatistics {
            pool_size,
            idle_connections,
            active_connections: pool_size - idle_connections,
            database_size,
            table_statistics: table_stats,
            activity_statistics: activity_stats,
        })
    }

    /// Get table-level statistics
    async fn get_table_statistics(&self) -> AppResult<Vec<TableStatistics>> {
        let rows = sqlx::query(r#"
            SELECT
                schemaname,
                tablename,
                n_tup_ins as inserts,
                n_tup_upd as updates,
                n_tup_del as deletes,
                n_live_tup as live_tuples,
                n_dead_tup as dead_tuples,
                last_vacuum,
                last_autovacuum,
                last_analyze,
                last_autoanalyze
            FROM pg_stat_user_tables
            ORDER BY n_live_tup DESC
            LIMIT 20
        "#)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::database(
                format!("Failed to get table statistics: {}", e),
                "statistics"
            ))?;

        let mut stats = Vec::new();
        for row in rows {
            stats.push(TableStatistics {
                schema_name: row.try_get("schemaname")?,
                table_name: row.try_get("tablename")?,
                inserts: row.try_get("inserts").unwrap_or(0),
                updates: row.try_get("updates").unwrap_or(0),
                deletes: row.try_get("deletes").unwrap_or(0),
                live_tuples: row.try_get("live_tuples").unwrap_or(0),
                dead_tuples: row.try_get("dead_tuples").unwrap_or(0),
            });
        }

        Ok(stats)
    }

    /// Get database activity statistics
    async fn get_activity_statistics(&self) -> AppResult<ActivityStatistics> {
        let row = sqlx::query(r#"
            SELECT
                sum(numbackends) as active_connections,
                sum(xact_commit) as transactions_committed,
                sum(xact_rollback) as transactions_rolled_back,
                sum(blks_read) as blocks_read,
                sum(blks_hit) as blocks_hit,
                sum(tup_returned) as tuples_returned,
                sum(tup_fetched) as tuples_fetched,
                sum(tup_inserted) as tuples_inserted,
                sum(tup_updated) as tuples_updated,
                sum(tup_deleted) as tuples_deleted
            FROM pg_stat_database
            WHERE datname = current_database()
        "#)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::database(
                format!("Failed to get activity statistics: {}", e),
                "statistics"
            ))?;

        Ok(ActivityStatistics {
            active_connections: row.try_get("active_connections").unwrap_or(0),
            transactions_committed: row.try_get("transactions_committed").unwrap_or(0),
            transactions_rolled_back: row.try_get("transactions_rolled_back").unwrap_or(0),
            blocks_read: row.try_get("blocks_read").unwrap_or(0),
            blocks_hit: row.try_get("blocks_hit").unwrap_or(0),
            cache_hit_ratio: {
                let read: i64 = row.try_get("blocks_read").unwrap_or(0);
                let hit: i64 = row.try_get("blocks_hit").unwrap_or(0);
                if read + hit > 0 {
                    (hit as f64 / (read + hit) as f64) * 100.0
                } else {
                    0.0
                }
            },
            tuples_returned: row.try_get("tuples_returned").unwrap_or(0),
            tuples_fetched: row.try_get("tuples_fetched").unwrap_or(0),
            tuples_inserted: row.try_get("tuples_inserted").unwrap_or(0),
            tuples_updated: row.try_get("tuples_updated").unwrap_or(0),
            tuples_deleted: row.try_get("tuples_deleted").unwrap_or(0),
        })
    }

    /// Execute a query with logging and metrics
    pub async fn execute_query<'e, E>(&self, query: &str, executor: E) -> AppResult<sqlx::postgres::PgQueryResult>
    where
        E: Executor<'e, Database = sqlx::Postgres>,
    {
        let start_time = std::time::Instant::now();

        let result = sqlx::query(query)
            .execute(executor)
            .await
            .map_err(|e| AppError::database(
                format!("Query execution failed: {}", e),
                "query_execution"
            ))?;

        let execution_time = start_time.elapsed();

        if execution_time.as_millis() > self.config.slow_query_threshold_ms as u128 {
            warn!("🐌 Slow query detected: {}ms - {}",
                  execution_time.as_millis(),
                  query.chars().take(100).collect::<String>());
        }

        debug!("📊 Query executed in {}ms", execution_time.as_millis());

        Ok(result)
    }

    /// Start a database transaction
    pub async fn begin_transaction(&self) -> AppResult<sqlx::Transaction<'_, sqlx::Postgres>> {
        self.pool.begin()
            .await
            .map_err(|e| AppError::database(
                format!("Failed to start transaction: {}", e),
                "transaction"
            ))
    }
}

/// PostgreSQL statistics
#[derive(Debug, Clone)]
pub struct PostgresStatistics {
    /// Current pool size
    pub pool_size: u32,
    /// Number of idle connections
    pub idle_connections: u32,
    /// Number of active connections
    pub active_connections: u32,
    /// Database size (human readable)
    pub database_size: String,
    /// Table-level statistics
    pub table_statistics: Vec<TableStatistics>,
    /// Activity statistics
    pub activity_statistics: ActivityStatistics,
}

/// Table-level statistics
#[derive(Debug, Clone)]
pub struct TableStatistics {
    /// Schema name
    pub schema_name: String,
    /// Table name
    pub table_name: String,
    /// Number of inserts
    pub inserts: i64,
    /// Number of updates
    pub updates: i64,
    /// Number of deletes
    pub deletes: i64,
    /// Number of live tuples
    pub live_tuples: i64,
    /// Number of dead tuples
    pub dead_tuples: i64,
}

/// Database activity statistics
#[derive(Debug, Clone)]
pub struct ActivityStatistics {
    /// Number of active connections
    pub active_connections: i32,
    /// Committed transactions
    pub transactions_committed: i64,
    /// Rolled back transactions
    pub transactions_rolled_back: i64,
    /// Blocks read from disk
    pub blocks_read: i64,
    /// Blocks hit in cache
    pub blocks_hit: i64,
    /// Cache hit ratio percentage
    pub cache_hit_ratio: f64,
    /// Tuples returned
    pub tuples_returned: i64,
    /// Tuples fetched
    pub tuples_fetched: i64,
    /// Tuples inserted
    pub tuples_inserted: i64,
    /// Tuples updated
    pub tuples_updated: i64,
    /// Tuples deleted
    pub tuples_deleted: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConfigLoader;

    #[tokio::test]
    async fn test_postgres_service_creation() {
        let config = ConfigLoader::new().without_env().create_default_config();

        // Skip test if no database URL is configured
        if config.database.url.is_empty() {
            return;
        }

        let result = PostgresService::new(&config.database).await;

        match result {
            Ok(service) => {
                let health = service.health_check().await;
                assert_eq!(health.name, "postgres");

                // Clean up
                let _ = service.close().await;
            }
            Err(_) => {
                // Expected in test environment without database
            }
        }
    }
}