//! Redis caching service implementation
//!
//! This module provides Redis connectivity, caching operations,
//! connection pool management, and advanced Redis features for the Solana Sniper Bot.

use redis::{Client, Commands, ConnectionManager, RedisResult, AsyncCommands};
use redis::aio::{ConnectionManager as AsyncConnectionManager, MultiplexedConnection};
use std::time::Duration;
use tracing::{info, warn, error, debug, instrument};
use tokio::sync::{RwLock, Mutex};
use std::sync::Arc;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

use crate::config::models::RedisConfig;
use crate::core::result::AppResult;
use crate::core::error::AppError;
use crate::application::health::{ComponentHealth, HealthStatus};

/// Redis connection pool type alias
pub type RedisPool = Arc<RwLock<AsyncConnectionManager>>;

/// Redis service with advanced caching capabilities
#[derive(Debug, Clone)]
pub struct RedisService {
    /// Primary async connection manager
    connection_manager: AsyncConnectionManager,
    /// Connection pool for high-throughput operations
    pool: RedisPool,
    /// Redis client for creating new connections
    client: Client,
    /// Service configuration
    config: RedisConfig,
    /// Connection statistics
    stats: Arc<Mutex<RedisStats>>,
    /// Cache metrics
    metrics: Arc<Mutex<CacheMetrics>>,
}

impl RedisService {
    /// Create a new Redis service with connection pool
    #[instrument(skip(config))]
    pub async fn new(config: &RedisConfig) -> AppResult<Self> {
        info!("🔴 Initializing Redis connection pool");

        // Validate configuration
        Self::validate_config(config)?;

        // Create Redis client with connection options
        let client = Self::create_client(config)?;

        // Test initial connection
        let mut test_conn = client.get_multiplexed_async_connection().await
            .map_err(|e| AppError::database(
                format!("Failed to establish Redis connection: {}", e),
                "connection_test"
            ))?;

        // Verify Redis is responsive
        let _: String = test_conn.ping().await
            .map_err(|e| AppError::database(
                format!("Redis ping failed: {}", e),
                "initial_ping"
            ))?;

        // Create async connection manager
        let connection_manager = ConnectionManager::new(client.clone())
            .await
            .map_err(|e| AppError::database(
                format!("Failed to create Redis connection manager: {}", e),
                "connection_manager"
            ))?;

        let pool = Arc::new(RwLock::new(connection_manager.clone()));

        info!("✅ Redis connection pool established");
        info!("📊 Redis configuration: prefix='{}', ttl={}s, max_connections={}",
              config.key_prefix, config.default_ttl_seconds, config.max_connections);

        // Initialize statistics and metrics
        let stats = Arc::new(Mutex::new(RedisStats::new()));
        let metrics = Arc::new(Mutex::new(CacheMetrics::new()));

        Ok(Self {
            connection_manager,
            pool,
            client,
            config: config.clone(),
            stats,
            metrics,
        })
    }

    /// Validate Redis configuration
    fn validate_config(config: &RedisConfig) -> AppResult<()> {
        if config.url.is_empty() {
            return Err(AppError::database(
                "Redis URL is required".to_string(),
                "validation"
            ));
        }

        if config.max_connections == 0 {
            return Err(AppError::database(
                "Max connections must be greater than 0".to_string(),
                "validation"
            ));
        }

        if config.connection_timeout_ms == 0 {
            return Err(AppError::database(
                "Connection timeout must be greater than 0".to_string(),
                "validation"
            ));
        }

        Ok(())
    }

    /// Create Redis client with optimized settings
    fn create_client(config: &RedisConfig) -> AppResult<Client> {
        let mut client_builder = redis::Client::open(config.url.as_str())
            .map_err(|e| AppError::database(
                format!("Failed to create Redis client: {}", e),
                "client_creation"
            ))?;

        // Configure connection timeout
        let connection_info = client_builder.get_connection_info();
        debug!("🔗 Redis connection info: {}:{}",
               connection_info.addr,
               connection_info.redis.database);

        Ok(client_builder)
    }

    /// Get the connection pool
    pub fn pool(&self) -> RedisPool {
        self.pool.clone()
    }

    /// Get Redis client for creating new connections
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Get a new multiplexed connection
    pub async fn get_multiplexed_connection(&self) -> AppResult<MultiplexedConnection> {
        self.client.get_multiplexed_async_connection()
            .await
            .map_err(|e| AppError::database(
                format!("Failed to get multiplexed connection: {}", e),
                "multiplexed_connection"
            ))
    }

    /// Close all connections
    pub async fn close(&self) -> AppResult<()> {
        info!("🔌 Closing Redis connections");

        // Update statistics
        {
            let mut stats = self.stats.lock().await;
            stats.connection_closed_at = Some(Utc::now());
        }

        // Connection manager handles cleanup automatically
        info!("✅ Redis connections closed");
        Ok(())
    }

    /// Execute a health check
    #[instrument(skip(self))]
    pub async fn health_check(&self) -> ComponentHealth {
        let mut component = ComponentHealth::new("redis".to_string(), false);
        let start_time = std::time::Instant::now();

        match self.execute_health_check().await {
            Ok(info) => {
                let response_time = start_time.elapsed().as_millis() as u64;
                component.mark_healthy(
                    Some(format!("Redis healthy: {}", info)),
                    Some(response_time)
                );
            }
            Err(e) => {
                component.mark_unhealthy(format!("Redis health check failed: {}", e));
            }
        }

        component
    }

    /// Execute the actual health check with detailed information
    async fn execute_health_check(&self) -> AppResult<String> {
        let mut conn = self.pool.write().await;

        // Test basic connectivity
        let ping_result: String = conn.ping().await
            .map_err(|e| AppError::database(
                format!("Redis PING failed: {}", e),
                "health_check"
            ))?;

        if ping_result != "PONG" {
            return Err(AppError::database(
                format!("Redis PING returned unexpected response: {}", ping_result),
                "health_check"
            ));
        }

        // Get Redis info for additional health data
        let info: String = redis::cmd("INFO")
            .arg("server")
            .query_async(&mut *conn)
            .await
            .map_err(|e| AppError::database(
                format!("Failed to get Redis info: {}", e),
                "health_check"
            ))?;

        // Parse version and uptime from info
        let version = Self::extract_info_field(&info, "redis_version")
            .unwrap_or_else(|| "unknown".to_string());
        let uptime = Self::extract_info_field(&info, "uptime_in_seconds")
            .unwrap_or_else(|| "0".to_string());

        Ok(format!("Redis v{}, uptime: {}s", version, uptime))
    }

    /// Extract field from Redis INFO output
    fn extract_info_field(info: &str, field: &str) -> Option<String> {
        for line in info.lines() {
            if line.starts_with(field) {
                if let Some((_, value)) = line.split_once(':') {
                    return Some(value.trim().to_string());
                }
            }
        }
        None
    }

    /// Set a key-value pair with optional TTL
    #[instrument(skip(self, value))]
    pub async fn set<V>(&self, key: &str, value: V, ttl: Option<Duration>) -> AppResult<()>
    where
        V: redis::ToRedisArgs + Send + Sync,
    {
        let full_key = self.build_key(key);
        let mut conn = self.pool.write().await;
        let start_time = std::time::Instant::now();

        let result = if let Some(ttl) = ttl {
            conn.set_ex(&full_key, value, ttl.as_secs() as usize).await
        } else {
            conn.set(&full_key, value).await
        };

        let operation_time = start_time.elapsed();

        result.map_err(|e| AppError::database(
            format!("Redis SET failed for key '{}': {}", full_key, e),
            "set"
        ))?;

        // Update metrics
        self.update_metrics("SET", operation_time, true).await;

        debug!("📝 Redis SET: {} (TTL: {:?}, took: {}ms)",
               full_key, ttl, operation_time.as_millis());
        Ok(())
    }

    /// Get a value by key
    #[instrument(skip(self))]
    pub async fn get<V>(&self, key: &str) -> AppResult<Option<V>>
    where
        V: redis::FromRedisValue,
    {
        let full_key = self.build_key(key);
        let mut conn = self.pool.write().await;
        let start_time = std::time::Instant::now();

        let result: RedisResult<V> = conn.get(&full_key).await;
        let operation_time = start_time.elapsed();

        match result {
            Ok(value) => {
                self.update_metrics("GET_HIT", operation_time, true).await;
                debug!("📖 Redis GET: {} = found ({}ms)", full_key, operation_time.as_millis());
                Ok(Some(value))
            }
            Err(redis::RedisError { kind: redis::ErrorKind::TypeError, .. }) => {
                self.update_metrics("GET_MISS", operation_time, true).await;
                debug!("📖 Redis GET: {} = not found ({}ms)", full_key, operation_time.as_millis());
                Ok(None)
            }
            Err(e) => {
                self.update_metrics("GET_ERROR", operation_time, false).await;
                error!("❌ Redis GET failed for key '{}': {}", full_key, e);
                Err(AppError::database(
                    format!("Redis GET failed for key '{}': {}", full_key, e),
                    "get"
                ))
            }
        }
    }

    /// Delete a key
    #[instrument(skip(self))]
    pub async fn delete(&self, key: &str) -> AppResult<bool> {
        let full_key = self.build_key(key);
        let mut conn = self.pool.write().await;
        let start_time = std::time::Instant::now();

        let deleted: i32 = conn.del(&full_key).await
            .map_err(|e| AppError::database(
                format!("Redis DEL failed for key '{}': {}", full_key, e),
                "delete"
            ))?;

        let operation_time = start_time.elapsed();
        let was_deleted = deleted > 0;

        self.update_metrics("DEL", operation_time, true).await;
        debug!("🗑️  Redis DEL: {} = {} ({}ms)",
               full_key, was_deleted, operation_time.as_millis());

        Ok(was_deleted)
    }

    /// Check if a key exists
    #[instrument(skip(self))]
    pub async fn exists(&self, key: &str) -> AppResult<bool> {
        let full_key = self.build_key(key);
        let mut conn = self.pool.write().await;
        let start_time = std::time::Instant::now();

        let exists: bool = conn.exists(&full_key).await
            .map_err(|e| AppError::database(
                format!("Redis EXISTS failed for key '{}': {}", full_key, e),
                "exists"
            ))?;

        let operation_time = start_time.elapsed();
        self.update_metrics("EXISTS", operation_time, true).await;

        debug!("🔍 Redis EXISTS: {} = {} ({}ms)",
               full_key, exists, operation_time.as_millis());
        Ok(exists)
    }

    /// Set expiration for a key
    #[instrument(skip(self))]
    pub async fn expire(&self, key: &str, ttl: Duration) -> AppResult<bool> {
        let full_key = self.build_key(key);
        let mut conn = self.pool.write().await;
        let start_time = std::time::Instant::now();

        let set: bool = conn.expire(&full_key, ttl.as_secs() as usize).await
            .map_err(|e| AppError::database(
                format!("Redis EXPIRE failed for key '{}': {}", full_key, e),
                "expire"
            ))?;

        let operation_time = start_time.elapsed();
        self.update_metrics("EXPIRE", operation_time, true).await;

        debug!("⏰ Redis EXPIRE: {} = {} (TTL: {:?}, {}ms)",
               full_key, set, ttl, operation_time.as_millis());
        Ok(set)
    }

    /// Get time to live for a key
    #[instrument(skip(self))]
    pub async fn ttl(&self, key: &str) -> AppResult<i64> {
        let full_key = self.build_key(key);
        let mut conn = self.pool.write().await;

        let ttl: i64 = conn.ttl(&full_key).await
            .map_err(|e| AppError::database(
                format!("Redis TTL failed for key '{}': {}", full_key, e),
                "ttl"
            ))?;

        debug!("⏱️  Redis TTL: {} = {}s", full_key, ttl);
        Ok(ttl)
    }

    /// Increment a numeric value
    #[instrument(skip(self))]
    pub async fn increment(&self, key: &str, amount: i64) -> AppResult<i64> {
        let full_key = self.build_key(key);
        let mut conn = self.pool.write().await;
        let start_time = std::time::Instant::now();

        let new_value: i64 = if amount == 1 {
            conn.incr(&full_key, 1).await
        } else {
            conn.incr(&full_key, amount).await
        }
            .map_err(|e| AppError::database(
                format!("Redis INCREMENT failed for key '{}': {}", full_key, e),
                "increment"
            ))?;

        let operation_time = start_time.elapsed();
        self.update_metrics("INCR", operation_time, true).await;

        debug!("📈 Redis INCREMENT: {} by {} = {} ({}ms)",
               full_key, amount, new_value, operation_time.as_millis());
        Ok(new_value)
    }

    /// Decrement a numeric value
    #[instrument(skip(self))]
    pub async fn decrement(&self, key: &str, amount: i64) -> AppResult<i64> {
        let full_key = self.build_key(key);
        let mut conn = self.pool.write().await;
        let start_time = std::time::Instant::now();

        let new_value: i64 = conn.decr(&full_key, amount).await
            .map_err(|e| AppError::database(
                format!("Redis DECREMENT failed for key '{}': {}", full_key, e),
                "decrement"
            ))?;

        let operation_time = start_time.elapsed();
        self.update_metrics("DECR", operation_time, true).await;

        debug!("📉 Redis DECREMENT: {} by {} = {} ({}ms)",
               full_key, amount, new_value, operation_time.as_millis());
        Ok(new_value)
    }

    /// Add item to a list (left push)
    #[instrument(skip(self, value))]
    pub async fn list_push<V>(&self, key: &str, value: V) -> AppResult<i64>
    where
        V: redis::ToRedisArgs + Send + Sync,
    {
        let full_key = self.build_key(key);
        let mut conn = self.pool.write().await;
        let start_time = std::time::Instant::now();

        let list_length: i64 = conn.lpush(&full_key, value).await
            .map_err(|e| AppError::database(
                format!("Redis LPUSH failed for key '{}': {}", full_key, e),
                "list_push"
            ))?;

        let operation_time = start_time.elapsed();
        self.update_metrics("LPUSH", operation_time, true).await;

        debug!("📝 Redis LPUSH: {} = length {} ({}ms)",
               full_key, list_length, operation_time.as_millis());
        Ok(list_length)
    }

    /// Remove and return item from list (right pop)
    #[instrument(skip(self))]
    pub async fn list_pop<V>(&self, key: &str) -> AppResult<Option<V>>
    where
        V: redis::FromRedisValue,
    {
        let full_key = self.build_key(key);
        let mut conn = self.pool.write().await;
        let start_time = std::time::Instant::now();

        let result: RedisResult<V> = conn.rpop(&full_key, None).await;
        let operation_time = start_time.elapsed();

        match result {
            Ok(value) => {
                self.update_metrics("RPOP_HIT", operation_time, true).await;
                debug!("📤 Redis RPOP: {} = found ({}ms)", full_key, operation_time.as_millis());
                Ok(Some(value))
            }
            Err(redis::RedisError { kind: redis::ErrorKind::TypeError, .. }) => {
                self.update_metrics("RPOP_MISS", operation_time, true).await;
                debug!("📤 Redis RPOP: {} = empty ({}ms)", full_key, operation_time.as_millis());
                Ok(None)
            }
            Err(e) => {
                self.update_metrics("RPOP_ERROR", operation_time, false).await;
                Err(AppError::database(
                    format!("Redis RPOP failed for key '{}': {}", full_key, e),
                    "list_pop"
                ))
            }
        }
    }

    /// Get items from a list (range)
    #[instrument(skip(self))]
    pub async fn list_range<V>(&self, key: &str, start: isize, stop: isize) -> AppResult<Vec<V>>
    where
        V: redis::FromRedisValue,
    {
        let full_key = self.build_key(key);
        let mut conn = self.pool.write().await;
        let start_time = std::time::Instant::now();

        let items: Vec<V> = conn.lrange(&full_key, start, stop).await
            .map_err(|e| AppError::database(
                format!("Redis LRANGE failed for key '{}': {}", full_key, e),
                "list_range"
            ))?;

        let operation_time = start_time.elapsed();
        self.update_metrics("LRANGE", operation_time, true).await;

        debug!("📖 Redis LRANGE: {} [{}:{}] = {} items ({}ms)",
               full_key, start, stop, items.len(), operation_time.as_millis());
        Ok(items)
    }

    /// Get list length
    #[instrument(skip(self))]
    pub async fn list_length(&self, key: &str) -> AppResult<i64> {
        let full_key = self.build_key(key);
        let mut conn = self.pool.write().await;

        let length: i64 = conn.llen(&full_key).await
            .map_err(|e| AppError::database(
                format!("Redis LLEN failed for key '{}': {}", full_key, e),
                "list_length"
            ))?;

        debug!("📏 Redis LLEN: {} = {}", full_key, length);
        Ok(length)
    }

    /// Add member to a set
    #[instrument(skip(self, member))]
    pub async fn set_add<M>(&self, key: &str, member: M) -> AppResult<bool>
    where
        M: redis::ToRedisArgs + Send + Sync,
    {
        let full_key = self.build_key(key);
        let mut conn = self.pool.write().await;
        let start_time = std::time::Instant::now();

        let added: i32 = conn.sadd(&full_key, member).await
            .map_err(|e| AppError::database(
                format!("Redis SADD failed for key '{}': {}", full_key, e),
                "set_add"
            ))?;

        let operation_time = start_time.elapsed();
        let was_added = added > 0;

        self.update_metrics("SADD", operation_time, true).await;
        debug!("📝 Redis SADD: {} = {} ({}ms)",
               full_key, was_added, operation_time.as_millis());
        Ok(was_added)
    }

    /// Remove member from a set
    #[instrument(skip(self, member))]
    pub async fn set_remove<M>(&self, key: &str, member: M) -> AppResult<bool>
    where
        M: redis::ToRedisArgs + Send + Sync,
    {
        let full_key = self.build_key(key);
        let mut conn = self.pool.write().await;

        let removed: i32 = conn.srem(&full_key, member).await
            .map_err(|e| AppError::database(
                format!("Redis SREM failed for key '{}': {}", full_key, e),
                "set_remove"
            ))?;

        let was_removed = removed > 0;
        debug!("🗑️  Redis SREM: {} = {}", full_key, was_removed);
        Ok(was_removed)
    }

    /// Check if member exists in set
    #[instrument(skip(self, member))]
    pub async fn set_is_member<M>(&self, key: &str, member: M) -> AppResult<bool>
    where
        M: redis::ToRedisArgs + Send + Sync,
    {
        let full_key = self.build_key(key);
        let mut conn = self.pool.write().await;

        let is_member: bool = conn.sismember(&full_key, member).await
            .map_err(|e| AppError::database(
                format!("Redis SISMEMBER failed for key '{}': {}", full_key, e),
                "set_is_member"
            ))?;

        debug!("🔍 Redis SISMEMBER: {} = {}", full_key, is_member);
        Ok(is_member)
    }

    /// Get all members of a set
    #[instrument(skip(self))]
    pub async fn set_members<M>(&self, key: &str) -> AppResult<Vec<M>>
    where
        M: redis::FromRedisValue,
    {
        let full_key = self.build_key(key);
        let mut conn = self.pool.write().await;

        let members: Vec<M> = conn.smembers(&full_key).await
            .map_err(|e| AppError::database(
                format!("Redis SMEMBERS failed for key '{}': {}", full_key, e),
                "set_members"
            ))?;

        debug!("📖 Redis SMEMBERS: {} = {} members", full_key, members.len());
        Ok(members)
    }

    /// Get set cardinality (number of members)
    #[instrument(skip(self))]
    pub async fn set_cardinality(&self, key: &str) -> AppResult<i64> {
        let full_key = self.build_key(key);
        let mut conn = self.pool.write().await;

        let cardinality: i64 = conn.scard(&full_key).await
            .map_err(|e| AppError::database(
                format!("Redis SCARD failed for key '{}': {}", full_key, e),
                "set_cardinality"
            ))?;

        debug!("🔢 Redis SCARD: {} = {}", full_key, cardinality);
        Ok(cardinality)
    }

    /// Set hash field
    #[instrument(skip(self, value))]
    pub async fn hash_set<V>(&self, key: &str, field: &str, value: V) -> AppResult<bool>
    where
        V: redis::ToRedisArgs + Send + Sync,
    {
        let full_key = self.build_key(key);
        let mut conn = self.pool.write().await;
        let start_time = std::time::Instant::now();

        let is_new: bool = conn.hset(&full_key, field, value).await
            .map_err(|e| AppError::database(
                format!("Redis HSET failed for key '{}' field '{}': {}", full_key, field, e),
                "hash_set"
            ))?;

        let operation_time = start_time.elapsed();
        self.update_metrics("HSET", operation_time, true).await;

        debug!("📝 Redis HSET: {}[{}] = {} ({}ms)",
               full_key, field, is_new, operation_time.as_millis());
        Ok(is_new)
    }

    /// Get hash field
    #[instrument(skip(self))]
    pub async fn hash_get<V>(&self, key: &str, field: &str) -> AppResult<Option<V>>
    where
        V: redis::FromRedisValue,
    {
        let full_key = self.build_key(key);
        let mut conn = self.pool.write().await;
        let start_time = std::time::Instant::now();

        let result: RedisResult<V> = conn.hget(&full_key, field).await;
        let operation_time = start_time.elapsed();

        match result {
            Ok(value) => {
                self.update_metrics("HGET_HIT", operation_time, true).await;
                debug!("📖 Redis HGET: {}[{}] = found ({}ms)",
                       full_key, field, operation_time.as_millis());
                Ok(Some(value))
            }
            Err(redis::RedisError { kind: redis::ErrorKind::TypeError, .. }) => {
                self.update_metrics("HGET_MISS", operation_time, true).await;
                debug!("📖 Redis HGET: {}[{}] = not found ({}ms)",
                       full_key, field, operation_time.as_millis());
                Ok(None)
            }
            Err(e) => {
                self.update_metrics("HGET_ERROR", operation_time, false).await;
                error!("❌ Redis HGET failed for key '{}' field '{}': {}", full_key, field, e);
                Err(AppError::database(
                    format!("Redis HGET failed for key '{}' field '{}': {}", full_key, field, e),
                    "hash_get"
                ))
            }
        }
    }

    /// Delete hash field
    #[instrument(skip(self))]
    pub async fn hash_delete(&self, key: &str, field: &str) -> AppResult<bool> {
        let full_key = self.build_key(key);
        let mut conn = self.pool.write().await;

        let deleted: i32 = conn.hdel(&full_key, field).await
            .map_err(|e| AppError::database(
                format!("Redis HDEL failed for key '{}' field '{}': {}", full_key, field, e),
                "hash_delete"
            ))?;

        let was_deleted = deleted > 0;
        debug!("🗑️  Redis HDEL: {}[{}] = {}", full_key, field, was_deleted);
        Ok(was_deleted)
    }

    /// Get all hash fields and values
    #[instrument(skip(self))]
    pub async fn hash_get_all(&self, key: &str) -> AppResult<HashMap<String, String>> {
        let full_key = self.build_key(key);
        let mut conn = self.pool.write().await;

        let hash: HashMap<String, String> = conn.hgetall(&full_key).await
            .map_err(|e| AppError::database(
                format!("Redis HGETALL failed for key '{}': {}", full_key, e),
                "hash_get_all"
            ))?;

        debug!("📖 Redis HGETALL: {} = {} fields", full_key, hash.len());
        Ok(hash)
    }

    /// Get hash field count
    #[instrument(skip(self))]
    pub async fn hash_length(&self, key: &str) -> AppResult<i64> {
        let full_key = self.build_key(key);
        let mut conn = self.pool.write().await;

        let length: i64 = conn.hlen(&full_key).await
            .map_err(|e| AppError::database(
                format!("Redis HLEN failed for key '{}': {}", full_key, e),
                "hash_length"
            ))?;

        debug!("📏 Redis HLEN: {} = {}", full_key, length);
        Ok(length)
    }

    /// Execute multiple commands in a pipeline
    #[instrument(skip(self, commands))]
    pub async fn pipeline(&self, commands: Vec<RedisCommand>) -> AppResult<Vec<RedisResult<redis::Value>>> {
        let mut conn = self.pool.write().await;
        let start_time = std::time::Instant::now();

        let mut pipe = redis::pipe();

        for command in &commands {
            match command {
                RedisCommand::Set { key, value, ttl } => {
                    let full_key = self.build_key(key);
                    if let Some(ttl) = ttl {
                        pipe.set_ex(&full_key, value, ttl.as_secs() as usize);
                    } else {
                        pipe.set(&full_key, value);
                    }
                }
                RedisCommand::Get { key } => {
                    let full_key = self.build_key(key);
                    pipe.get(&full_key);
                }
                RedisCommand::Delete { key } => {
                    let full_key = self.build_key(key);
                    pipe.del(&full_key);
                }
                RedisCommand::Increment { key, amount } => {
                    let full_key = self.build_key(key);
                    pipe.incr(&full_key, *amount);
                }
            }
        }

        let results: Vec<RedisResult<redis::Value>> = pipe.query_async(&mut *conn).await
            .map_err(|e| AppError::database(
                format!("Redis pipeline failed: {}", e),
                "pipeline"
            ))?;

        let operation_time = start_time.elapsed();
        self.update_metrics("PIPELINE", operation_time, true).await;

        debug!("🔄 Redis PIPELINE: {} commands executed ({}ms)",
               commands.len(), operation_time.as_millis());

        Ok(results)
    }

    /// Cache a serializable value with JSON encoding
    #[instrument(skip(self, value))]
    pub async fn cache_json<T>(&self, key: &str, value: &T, ttl: Option<Duration>) -> AppResult<()>
    where
        T: Serialize,
    {
        let json_value = serde_json::to_string(value)
            .map_err(|e| AppError::database(
                format!("Failed to serialize value for caching: {}", e),
                "cache_json"
            ))?;

        self.set(key, json_value, ttl).await
    }

    /// Retrieve and deserialize a cached JSON value
    #[instrument(skip(self))]
    pub async fn get_cached_json<T>(&self, key: &str) -> AppResult<Option<T>>
    where
        T: for<'de> Deserialize<'de>,
    {
        let json_value: Option<String> = self.get(key).await?;

        match json_value {
            Some(json_str) => {
                let value = serde_json::from_str(&json_str)
                    .map_err(|e| AppError::database(
                        format!("Failed to deserialize cached value: {}", e),
                        "get_cached_json"
                    ))?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    /// Clear all keys matching a pattern
    #[instrument(skip(self))]
    pub async fn clear_pattern(&self, pattern: &str) -> AppResult<u32> {
        let full_pattern = self.build_key(pattern);
        let mut conn = self.pool.write().await;

        // Use SCAN instead of KEYS for better performance
        let mut cursor = 0;
        let mut total_deleted = 0u32;

        loop {
            let (new_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(&full_pattern)
                .arg("COUNT")
                .arg(100)
                .query_async(&mut *conn)
                .await
                .map_err(|e| AppError::database(
                    format!("Redis SCAN failed for pattern '{}': {}", full_pattern, e),
                    "clear_pattern"
                ))?;

            if !keys.is_empty() {
                let deleted: i32 = conn.del(&keys).await
                    .map_err(|e| AppError::database(
                        format!("Redis DEL failed for pattern '{}': {}", full_pattern, e),
                        "clear_pattern"
                    ))?;
                total_deleted += deleted as u32;
            }

            cursor = new_cursor;
            if cursor == 0 {
                break;
            }
        }

        debug!("🗑️  Redis CLEAR: Deleted {} keys for pattern '{}'", total_deleted, full_pattern);
        Ok(total_deleted)
    }

    /// Get Redis statistics
    #[instrument(skip(self))]
    pub async fn get_statistics(&self) -> AppResult<RedisStatistics> {
        debug!("📊 Collecting Redis statistics");

        let mut conn = self.pool.write().await;

        // Get Redis info
        let info: String = redis::cmd("INFO")
            .arg("all")
            .query_async(&mut *conn)
            .await
            .map_err(|e| AppError::database(
                format!("Failed to get Redis info: {}", e),
                "statistics"
            ))?;

        // Parse the info string
        let stats = self.parse_info_string(&info);

        // Get our internal metrics
        let metrics = self.metrics.lock().await.clone();

        Ok(RedisStatistics {
            connected_clients: stats.get("connected_clients")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
            used_memory_bytes: stats.get("used_memory")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
            used_memory_human: stats.get("used_memory_human")
                .cloned()
                .unwrap_or_else(|| "N/A".to_string()),
            total_commands_processed: stats.get("total_commands_processed")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
            instantaneous_ops_per_sec: stats.get("instantaneous_ops_per_sec")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
            keyspace_hits: stats.get("keyspace_hits")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
            keyspace_misses: stats.get("keyspace_misses")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
            cache_hit_ratio: {
                let hits: f64 = stats.get("keyspace_hits")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0.0);
                let misses: f64 = stats.get("keyspace_misses")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(0.0);
                if hits + misses > 0.0 {
                    (hits / (hits + misses)) * 100.0
                } else {
                    0.0
                }
            },
            uptime_in_seconds: stats.get("uptime_in_seconds")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
            our_metrics: metrics,
        })
    }

    /// Update internal metrics
    async fn update_metrics(&self, operation: &str, duration: Duration, success: bool) {
        let mut metrics = self.metrics.lock().await;
        metrics.total_operations += 1;
        metrics.total_duration += duration;

        if success {
            metrics.successful_operations += 1;
        } else {
            metrics.failed_operations += 1;
        }

        // Update operation-specific metrics
        let operation_metrics = metrics.operation_metrics
            .entry(operation.to_string())
            .or_insert(OperationMetrics::new());

        operation_metrics.count += 1;
        operation_metrics.total_duration += duration;
        operation_metrics.min_duration = operation_metrics.min_duration
            .map(|min| min.min(duration))
            .or(Some(duration));
        operation_metrics.max_duration = operation_metrics.max_duration
            .map(|max| max.max(duration))
            .or(Some(duration));

        // Update cache hit/miss metrics
        match operation {
            "GET_HIT" | "HGET_HIT" | "RPOP_HIT" => metrics.cache_hits += 1,
            "GET_MISS" | "HGET_MISS" | "RPOP_MISS" => metrics.cache_misses += 1,
            _ => {}
        }
    }

    /// Build full key with prefix
    fn build_key(&self, key: &str) -> String {
        format!("{}{}", self.config.key_prefix, key)
    }

    /// Parse Redis INFO string into key-value pairs
    fn parse_info_string(&self, info: &str) -> HashMap<String, String> {
        let mut stats = HashMap::new();

        for line in info.lines() {
            if line.starts_with('#') || line.is_empty() {
                continue;
            }

            if let Some((key, value)) = line.split_once(':') {
                stats.insert(key.trim().to_string(), value.trim().to_string());
            }
        }

        stats
    }

    /// Get cache metrics
    pub async fn get_cache_metrics(&self) -> CacheMetrics {
        self.metrics.lock().await.clone()
    }

    /// Reset cache metrics
    pub async fn reset_metrics(&self) {
        let mut metrics = self.metrics.lock().await;
        *metrics = CacheMetrics::new();
        info!("📊 Redis cache metrics reset");
    }

    /// Perform maintenance operations
    #[instrument(skip(self))]
    pub async fn maintenance(&self) -> AppResult<MaintenanceReport> {
        info!("🔧 Starting Redis maintenance");

        let mut conn = self.pool.write().await;
        let start_time = std::time::Instant::now();

        // Get database info
        let info: String = redis::cmd("INFO")
            .arg("keyspace")
            .query_async(&mut *conn)
            .await
            .map_err(|e| AppError::database(
                format!("Failed to get keyspace info: {}", e),
                "maintenance"
            ))?;

        // Count keys with our prefix
        let pattern = format!("{}*", self.config.key_prefix);
        let mut cursor = 0;
        let mut total_keys = 0;
        let mut expired_keys = 0;

        loop {
            let (new_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(&pattern)
                .arg("COUNT")
                .arg(100)
                .query_async(&mut *conn)
                .await
                .map_err(|e| AppError::database(
                    format!("SCAN failed during maintenance: {}", e),
                    "maintenance"
                ))?;

            total_keys += keys.len();

            // Check for expired keys
            for key in keys {
                let ttl: i64 = conn.ttl(&key).await.unwrap_or(-1);
                if ttl == -2 {
                    expired_keys += 1;
                }
            }

            cursor = new_cursor;
            if cursor == 0 {
                break;
            }
        }

        let maintenance_duration = start_time.elapsed();

        let report = MaintenanceReport {
            total_keys,
            expired_keys,
            maintenance_duration,
            performed_at: Utc::now(),
        };

        info!("✅ Redis maintenance completed: {} keys scanned, {} expired ({}ms)",
              total_keys, expired_keys, maintenance_duration.as_millis());

        Ok(report)
    }
}

/// Redis command for pipeline operations
#[derive(Debug, Clone)]
pub enum RedisCommand {
    Set { key: String, value: String, ttl: Option<Duration> },
    Get { key: String },
    Delete { key: String },
    Increment { key: String, amount: i64 },
}

/// Redis statistics
#[derive(Debug, Clone)]
pub struct RedisStatistics {
    /// Number of connected clients
    pub connected_clients: i32,
    /// Used memory in bytes
    pub used_memory_bytes: u64,
    /// Used memory (human readable)
    pub used_memory_human: String,
    /// Total commands processed
    pub total_commands_processed: u64,
    /// Operations per second
    pub instantaneous_ops_per_sec: i32,
    /// Cache hits
    pub keyspace_hits: u64,
    /// Cache misses
    pub keyspace_misses: u64,
    /// Cache hit ratio percentage
    pub cache_hit_ratio: f64,
    /// Uptime in seconds
    pub uptime_in_seconds: u64,
    /// Our application-specific metrics
    pub our_metrics: CacheMetrics,
}

/// Internal Redis connection statistics
#[derive(Debug, Clone)]
struct RedisStats {
    connection_established_at: DateTime<Utc>,
    connection_closed_at: Option<DateTime<Utc>>,
    total_reconnections: u32,
    last_reconnection_at: Option<DateTime<Utc>>,
}

impl RedisStats {
    fn new() -> Self {
        Self {
            connection_established_at: Utc::now(),
            connection_closed_at: None,
            total_reconnections: 0,
            last_reconnection_at: None,
        }
    }
}

/// Cache performance metrics
#[derive(Debug, Clone)]
pub struct CacheMetrics {
    /// Total cache operations
    pub total_operations: u64,
    /// Successful operations
    pub successful_operations: u64,
    /// Failed operations
    pub failed_operations: u64,
    /// Cache hits
    pub cache_hits: u64,
    /// Cache misses
    pub cache_misses: u64,
    /// Total operation duration
    pub total_duration: Duration,
    /// Per-operation metrics
    pub operation_metrics: HashMap<String, OperationMetrics>,
    /// Metrics collection started at
    pub started_at: DateTime<Utc>,
}

impl CacheMetrics {
    fn new() -> Self {
        Self {
            total_operations: 0,
            successful_operations: 0,
            failed_operations: 0,
            cache_hits: 0,
            cache_misses: 0,
            total_duration: Duration::ZERO,
            operation_metrics: HashMap::new(),
            started_at: Utc::now(),
        }
    }

    /// Calculate cache hit ratio
    pub fn cache_hit_ratio(&self) -> f64 {
        let total_cache_ops = self.cache_hits + self.cache_misses;
        if total_cache_ops > 0 {
            (self.cache_hits as f64 / total_cache_ops as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Calculate success ratio
    pub fn success_ratio(&self) -> f64 {
        if self.total_operations > 0 {
            (self.successful_operations as f64 / self.total_operations as f64) * 100.0
        } else {
            0.0
        }
    }

    /// Calculate average operation duration
    pub fn average_duration(&self) -> Duration {
        if self.total_operations > 0 {
            self.total_duration / self.total_operations as u32
        } else {
            Duration::ZERO
        }
    }
}

/// Per-operation metrics
#[derive(Debug, Clone)]
pub struct OperationMetrics {
    pub count: u64,
    pub total_duration: Duration,
    pub min_duration: Option<Duration>,
    pub max_duration: Option<Duration>,
}

impl OperationMetrics {
    fn new() -> Self {
        Self {
            count: 0,
            total_duration: Duration::ZERO,
            min_duration: None,
            max_duration: None,
        }
    }

    /// Calculate average duration for this operation
    pub fn average_duration(&self) -> Duration {
        if self.count > 0 {
            self.total_duration / self.count as u32
        } else {
            Duration::ZERO
        }
    }
}

/// Maintenance report
#[derive(Debug, Clone)]
pub struct MaintenanceReport {
    pub total_keys: usize,
    pub expired_keys: usize,
    pub maintenance_duration: Duration,
    pub performed_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConfigLoader;

    #[tokio::test]
    async fn test_redis_service_creation() {
        let config = ConfigLoader::new().without_env().create_default_config();

        // Skip test if no Redis URL is configured
        if config.redis.url.is_empty() {
            return;
        }

        let result = RedisService::new(&config.redis).await;

        match result {
            Ok(service) => {
                let health = service.health_check().await;
                assert_eq!(health.name, "redis");

                // Test basic operations
                let _ = service.set("test_key", "test_value", None).await;
                let value: Option<String> = service.get("test_key").await.unwrap_or(None);
                assert_eq!(value, Some("test_value".to_string()));

                // Test JSON caching
                #[derive(Serialize, Deserialize, PartialEq, Debug)]
                struct TestData {
                    id: u32,
                    name: String,
                }

                let test_data = TestData {
                    id: 1,
                    name: "test".to_string(),
                };

                let _ = service.cache_json("test_json", &test_data, None).await;
                let cached_data: Option<TestData> = service.get_cached_json("test_json").await.unwrap_or(None);
                assert_eq!(cached_data, Some(test_data));

                // Clean up
                let _ = service.delete("test_key").await;
                let _ = service.delete("test_json").await;
                let _ = service.close().await;
            }
            Err(_) => {
                // Expected in test environment without Redis
            }
        }
    }

    #[tokio::test]
    async fn test_redis_key_building() {
        let config = crate::config::models::RedisConfig {
            url: "redis://localhost".to_string(),
            max_connections: 20,
            connection_timeout_ms: 3000,
            command_timeout_ms: 1000,
            default_ttl_seconds: 3600,
            key_prefix: "test:".to_string(),
            enable_debug_logging: false,
            enable_clustering: false,
            enable_persistence: false,
            persistence_interval_seconds: 300,
        };

        // Create a mock service to test key building
        if let Ok(service) = RedisService::new(&config).await {
            let key = service.build_key("mykey");
            assert_eq!(key, "test:mykey");
        }
    }

    #[tokio::test]
    async fn test_redis_metrics() {
        let config = ConfigLoader::new().without_env().create_default_config();

        if config.redis.url.is_empty() {
            return;
        }

        if let Ok(service) = RedisService::new(&config.redis).await {
            // Perform some operations
            let _ = service.set("metrics_test", "value", None).await;
            let _: Option<String> = service.get("metrics_test").await.unwrap_or(None);
            let _ = service.get::<String>("non_existent_key").await;

            // Check metrics
            let metrics = service.get_cache_metrics().await;
            assert!(metrics.total_operations > 0);
            assert!(metrics.cache_hits > 0);
            assert!(metrics.cache_misses > 0);

            // Clean up
            let _ = service.delete("metrics_test").await;
        }
    }

    #[tokio::test]
    async fn test_redis_lists() {
        let config = ConfigLoader::new().without_env().create_default_config();

        if config.redis.url.is_empty() {
            return;
        }

        if let Ok(service) = RedisService::new(&config.redis).await {
            let list_key = "test_list";

            // Test list operations
            let length = service.list_push(list_key, "item1").await.unwrap();
            assert_eq!(length, 1);

            let length = service.list_push(list_key, "item2").await.unwrap();
            assert_eq!(length, 2);

            let items: Vec<String> = service.list_range(list_key, 0, -1).await.unwrap();
            assert_eq!(items.len(), 2);

            let popped: Option<String> = service.list_pop(list_key).await.unwrap();
            assert_eq!(popped, Some("item1".to_string()));

            // Clean up
            let _ = service.delete(list_key).await;
        }
    }

    #[tokio::test]
    async fn test_redis_sets() {
        let config = ConfigLoader::new().without_env().create_default_config();

        if config.redis.url.is_empty() {
            return;
        }

        if let Ok(service) = RedisService::new(&config.redis).await {
            let set_key = "test_set";

            // Test set operations
            let added = service.set_add(set_key, "member1").await.unwrap();
            assert!(added);

            let added = service.set_add(set_key, "member2").await.unwrap();
            assert!(added);

            let added = service.set_add(set_key, "member1").await.unwrap();
            assert!(!added); // Already exists

            let is_member = service.set_is_member(set_key, "member1").await.unwrap();
            assert!(is_member);

            let cardinality = service.set_cardinality(set_key).await.unwrap();
            assert_eq!(cardinality, 2);

            let members: Vec<String> = service.set_members(set_key).await.unwrap();
            assert_eq!(members.len(), 2);

            // Clean up
            let _ = service.delete(set_key).await;
        }
    }

    #[tokio::test]
    async fn test_redis_hashes() {
        let config = ConfigLoader::new().without_env().create_default_config();

        if config.redis.url.is_empty() {
            return;
        }

        if let Ok(service) = RedisService::new(&config.redis).await {
            let hash_key = "test_hash";

            // Test hash operations
            let is_new = service.hash_set(hash_key, "field1", "value1").await.unwrap();
            assert!(is_new);

            let value: Option<String> = service.hash_get(hash_key, "field1").await.unwrap();
            assert_eq!(value, Some("value1".to_string()));

            let is_new = service.hash_set(hash_key, "field2", "value2").await.unwrap();
            assert!(is_new);

            let length = service.hash_length(hash_key).await.unwrap();
            assert_eq!(length, 2);

            let all_fields = service.hash_get_all(hash_key).await.unwrap();
            assert_eq!(all_fields.len(), 2);

            let deleted = service.hash_delete(hash_key, "field1").await.unwrap();
            assert!(deleted);

            // Clean up
            let _ = service.delete(hash_key).await;
        }
    }
}