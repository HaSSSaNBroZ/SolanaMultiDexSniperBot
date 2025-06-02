//! Health monitoring service
//!
//! This module provides comprehensive health monitoring for all application components,
//! including database connections, external services, and internal system health.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn, error, instrument};
use chrono::{DateTime, Utc};

use crate::config::AppConfig;
use crate::core::result::AppResult;
use crate::core::error::AppError;

/// Overall system health status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    /// All components are healthy
    Healthy,
    /// Some non-critical components are unhealthy
    Degraded,
    /// Critical components are unhealthy
    Unhealthy,
    /// System is starting up
    Starting,
}

/// Health status of an individual component
#[derive(Debug, Clone)]
pub struct ComponentHealth {
    /// Component name
    pub name: String,

    /// Current health status
    pub status: HealthStatus,

    /// Health check message
    pub message: Option<String>,

    /// Last successful health check
    pub last_success: Option<DateTime<Utc>>,

    /// Last health check attempt
    pub last_check: DateTime<Utc>,

    /// Number of consecutive failures
    pub consecutive_failures: u32,

    /// Whether this component is critical for overall system health
    pub is_critical: bool,

    /// Response time for last health check (in milliseconds)
    pub response_time_ms: Option<u64>,
}

impl ComponentHealth {
    /// Create a new component health status
    pub fn new(name: String, is_critical: bool) -> Self {
        Self {
            name,
            status: HealthStatus::Starting,
            message: None,
            last_success: None,
            last_check: Utc::now(),
            consecutive_failures: 0,
            is_critical,
            response_time_ms: None,
        }
    }

    /// Mark component as healthy
    pub fn mark_healthy(&mut self, message: Option<String>, response_time_ms: Option<u64>) {
        self.status = HealthStatus::Healthy;
        self.message = message;
        self.last_success = Some(Utc::now());
        self.last_check = Utc::now();
        self.consecutive_failures = 0;
        self.response_time_ms = response_time_ms;
    }

    /// Mark component as unhealthy
    pub fn mark_unhealthy(&mut self, message: String) {
        self.status = HealthStatus::Unhealthy;
        self.message = Some(message);
        self.last_check = Utc::now();
        self.consecutive_failures += 1;
        self.response_time_ms = None;
    }

    /// Mark component as degraded
    pub fn mark_degraded(&mut self, message: String, response_time_ms: Option<u64>) {
        self.status = HealthStatus::Degraded;
        self.message = Some(message);
        self.last_check = Utc::now();
        self.response_time_ms = response_time_ms;
    }
}

/// Health monitoring service
#[derive(Debug)]
pub struct HealthService {
    /// Application configuration
    config: Arc<AppConfig>,

    /// Component health statuses
    components: Arc<RwLock<HashMap<String, ComponentHealth>>>,

    /// Whether the service is running
    is_running: Arc<RwLock<bool>>,
}

impl HealthService {
    /// Create a new health service
    pub fn new(config: Arc<AppConfig>) -> Self {
        let mut components = HashMap::new();

        // Initialize all monitored components
        components.insert("config".to_string(), ComponentHealth::new("config".to_string(), true));
        components.insert("database".to_string(), ComponentHealth::new("database".to_string(), true));
        components.insert("redis".to_string(), ComponentHealth::new("redis".to_string(), false));
        components.insert("solana_rpc".to_string(), ComponentHealth::new("solana_rpc".to_string(), true));
        components.insert("helius_api".to_string(), ComponentHealth::new("helius_api".to_string(), false));
        components.insert("birdeye_api".to_string(), ComponentHealth::new("birdeye_api".to_string(), false));
        components.insert("telegram_bot".to_string(), ComponentHealth::new("telegram_bot".to_string(), false));
        components.insert("metrics".to_string(), ComponentHealth::new("metrics".to_string(), false));

        Self {
            config,
            components: Arc::new(RwLock::new(components)),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start the health monitoring service
    #[instrument(skip(self))]
    pub async fn start(&self) -> AppResult<()> {
        debug!("🔍 Starting health monitoring service");

        {
            let mut running = self.is_running.write().await;
            *running = true;
        }

        // Perform initial health checks
        self.check_all_components().await?;

        // Start periodic health check loop
        let components = self.components.clone();
        let config = self.config.clone();
        let is_running = self.is_running.clone();

        tokio::spawn(async move {
            let interval = std::time::Duration::from_secs(config.monitoring.health_check_interval_seconds);
            let mut interval_timer = tokio::time::interval(interval);

            loop {
                interval_timer.tick().await;

                {
                    let running = is_running.read().await;
                    if !*running {
                        break;
                    }
                }

                // Perform health checks
                if let Err(e) = Self::perform_periodic_checks(&components, &config).await {
                    error!("Failed to perform health checks: {}", e);
                }
            }

            debug!("Health monitoring service stopped");
        });

        debug!("✅ Health monitoring service started");
        Ok(())
    }

    /// Stop the health monitoring service
    pub async fn stop(&self) -> AppResult<()> {
        debug!("🛑 Stopping health monitoring service");

        {
            let mut running = self.is_running.write().await;
            *running = false;
        }

        debug!("✅ Health monitoring service stopped");
        Ok(())
    }

    /// Get the overall system health status
    pub async fn get_overall_health(&self) -> HealthStatus {
        let components = self.components.read().await;

        let mut has_critical_failure = false;
        let mut has_degraded = false;
        let mut all_starting = true;

        for component in components.values() {
            match component.status {
                HealthStatus::Unhealthy if component.is_critical => {
                    has_critical_failure = true;
                }
                HealthStatus::Unhealthy | HealthStatus::Degraded => {
                    has_degraded = true;
                }
                HealthStatus::Healthy => {
                    all_starting = false;
                }
                HealthStatus::Starting => {
                    // Keep all_starting true
                }
            }
        }

        if has_critical_failure {
            HealthStatus::Unhealthy
        } else if has_degraded {
            HealthStatus::Degraded
        } else if all_starting {
            HealthStatus::Starting
        } else {
            HealthStatus::Healthy
        }
    }

    /// Get health status for a specific component
    pub async fn get_component_health(&self, component_name: &str) -> Option<ComponentHealth> {
        let components = self.components.read().await;
        components.get(component_name).cloned()
    }

    /// Get health status for all components
    pub async fn get_all_component_health(&self) -> HashMap<String, ComponentHealth> {
        let components = self.components.read().await;
        components.clone()
    }

    /// Check all components health
    async fn check_all_components(&self) -> AppResult<()> {
        debug!("🔍 Performing health checks on all components");

        // Check configuration health
        self.check_config_health().await;

        // Check database health (placeholder for Phase 3)
        self.check_database_health().await;

        // Check Redis health (placeholder for Phase 3)
        self.check_redis_health().await;

        // Check Solana RPC health
        self.check_solana_rpc_health().await;

        // Check external APIs health
        self.check_helius_api_health().await;
        self.check_birdeye_api_health().await;

        // Check Telegram bot health (placeholder for Phase 9)
        self.check_telegram_bot_health().await;

        // Check metrics system health
        self.check_metrics_health().await;

        debug!("✅ Health checks completed");
        Ok(())
    }

    /// Perform periodic health checks
    async fn perform_periodic_checks(
        components: &Arc<RwLock<HashMap<String, ComponentHealth>>>,
        config: &AppConfig,
    ) -> AppResult<()> {
        debug!("🔄 Performing periodic health checks");

        // Create a temporary health service for periodic checks
        let temp_service = HealthService {
            config: Arc::new(config.clone()),
            components: components.clone(),
            is_running: Arc::new(RwLock::new(true)),
        };

        temp_service.check_all_components().await?;
        Ok(())
    }

    /// Check configuration health
    async fn check_config_health(&self) {
        let start_time = std::time::Instant::now();

        let validation_result = self.config.validate();
        let response_time = start_time.elapsed().as_millis() as u64;

        let mut components = self.components.write().await;
        if let Some(component) = components.get_mut("config") {
            match validation_result {
                Ok(result) if result.is_valid => {
                    component.mark_healthy(
                        Some("Configuration is valid".to_string()),
                        Some(response_time),
                    );
                }
                Ok(result) => {
                    let message = format!("Configuration has warnings: {:?}", result.warnings);
                    component.mark_degraded(message, Some(response_time));
                }
                Err(e) => {
                    component.mark_unhealthy(format!("Configuration validation failed: {}", e));
                }
            }
        }
    }

    /// Check database health (placeholder)
    async fn check_database_health(&self) {
        let mut components = self.components.write().await;
        if let Some(component) = components.get_mut("database") {
            if self.config.database.url.is_empty() {
                component.mark_unhealthy("Database URL not configured".to_string());
            } else {
                // TODO: Implement actual database health check in Phase 3
                component.mark_healthy(
                    Some("Database connection check pending (Phase 3)".to_string()),
                    Some(1),
                );
            }
        }
    }

    /// Check Redis health (placeholder)
    async fn check_redis_health(&self) {
        let mut components = self.components.write().await;
        if let Some(component) = components.get_mut("redis") {
            if self.config.redis.url.is_empty() {
                component.mark_unhealthy("Redis URL not configured".to_string());
            } else {
                // TODO: Implement actual Redis health check in Phase 3
                component.mark_healthy(
                    Some("Redis connection check pending (Phase 3)".to_string()),
                    Some(1),
                );
            }
        }
    }

    /// Check Solana RPC health
    async fn check_solana_rpc_health(&self) {
        let start_time = std::time::Instant::now();

        // TODO: Implement actual Solana RPC health check in Phase 4
        // For now, just validate the URL format
        let url_validation = crate::utils::validation::validate_url(&self.config.solana.rpc_url);
        let response_time = start_time.elapsed().as_millis() as u64;

        let mut components = self.components.write().await;
        if let Some(component) = components.get_mut("solana_rpc") {
            match url_validation {
                Ok(_) => {
                    component.mark_healthy(
                        Some("Solana RPC URL is valid (Phase 4: actual connection check)".to_string()),
                        Some(response_time),
                    );
                }
                Err(e) => {
                    component.mark_unhealthy(format!("Invalid Solana RPC URL: {}", e));
                }
            }
        }
    }

    /// Check Helius API health
    async fn check_helius_api_health(&self) {
        let mut components = self.components.write().await;
        if let Some(component) = components.get_mut("helius_api") {
            if self.config.helius.api_key.is_empty() {
                component.mark_degraded(
                    "Helius API key not configured - limited functionality".to_string(),
                    None,
                );
            } else {
                // TODO: Implement actual Helius API health check in Phase 4
                component.mark_healthy(
                    Some("Helius API check pending (Phase 4)".to_string()),
                    Some(1),
                );
            }
        }
    }

    /// Check Birdeye API health
    async fn check_birdeye_api_health(&self) {
        let mut components = self.components.write().await;
        if let Some(component) = components.get_mut("birdeye_api") {
            if self.config.birdeye.api_key.is_empty() {
                component.mark_degraded(
                    "Birdeye API key not configured - limited functionality".to_string(),
                    None,
                );
            } else {
                // TODO: Implement actual Birdeye API health check in Phase 4
                component.mark_healthy(
                    Some("Birdeye API check pending (Phase 4)".to_string()),
                    Some(1),
                );
            }
        }
    }

    /// Check Telegram bot health (placeholder)
    async fn check_telegram_bot_health(&self) {
        let mut components = self.components.write().await;
        if let Some(component) = components.get_mut("telegram_bot") {
            if self.config.telegram.bot_token.is_empty() {
                component.mark_unhealthy("Telegram bot token not configured".to_string());
            } else {
                // TODO: Implement actual Telegram bot health check in Phase 9
                component.mark_healthy(
                    Some("Telegram bot check pending (Phase 9)".to_string()),
                    Some(1),
                );
            }
        }
    }

    /// Check metrics system health
    async fn check_metrics_health(&self) {
        let mut components = self.components.write().await;
        if let Some(component) = components.get_mut("metrics") {
            if self.config.analytics.enable_metrics {
                // TODO: Implement actual metrics system health check in Phase 3
                component.mark_healthy(
                    Some("Metrics system check pending (Phase 3)".to_string()),
                    Some(1),
                );
            } else {
                component.mark_degraded(
                    "Metrics collection disabled".to_string(),
                    None,
                );
            }
        }
    }

    /// Get health summary as a formatted string
    pub async fn get_health_summary(&self) -> String {
        let overall_status = self.get_overall_health().await;
        let components = self.components.read().await;

        let mut summary = format!("Overall Status: {:?}\n\nComponents:\n", overall_status);

        for (name, component) in components.iter() {
            let status_emoji = match component.status {
                HealthStatus::Healthy => "✅",
                HealthStatus::Degraded => "⚠️",
                HealthStatus::Unhealthy => "❌",
                HealthStatus::Starting => "🔄",
            };

            let criticality = if component.is_critical { " (Critical)" } else { "" };

            summary.push_str(&format!(
                "{} {}{}: {:?}",
                status_emoji,
                name,
                criticality,
                component.status
            ));

            if let Some(ref message) = component.message {
                summary.push_str(&format!(" - {}", message));
            }

            if let Some(response_time) = component.response_time_ms {
                summary.push_str(&format!(" ({}ms)", response_time));
            }

            summary.push('\n');
        }

        summary
    }

    /// Check if the service is running
    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "Healthy"),
            HealthStatus::Degraded => write!(f, "Degraded"),
            HealthStatus::Unhealthy => write!(f, "Unhealthy"),
            HealthStatus::Starting => write!(f, "Starting"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConfigLoader;

    #[tokio::test]
    async fn test_health_service_creation() {
        let config = Arc::new(ConfigLoader::new().without_env().create_default_config());
        let health_service = HealthService::new(config);

        assert!(!health_service.is_running().await);

        let components = health_service.get_all_component_health().await;
        assert!(!components.is_empty());
        assert!(components.contains_key("config"));
        assert!(components.contains_key("database"));
    }

    #[tokio::test]
    async fn test_component_health_lifecycle() {
        let mut component = ComponentHealth::new("test".to_string(), true);

        assert!(matches!(component.status, HealthStatus::Starting));
        assert_eq!(component.consecutive_failures, 0);

        component.mark_healthy(Some("All good".to_string()), Some(100));
        assert!(matches!(component.status, HealthStatus::Healthy));
        assert_eq!(component.consecutive_failures, 0);
        assert!(component.last_success.is_some());

        component.mark_unhealthy("Something failed".to_string());
        assert!(matches!(component.status, HealthStatus::Unhealthy));
        assert_eq!(component.consecutive_failures, 1);

        component.mark_unhealthy("Still failing".to_string());
        assert_eq!(component.consecutive_failures, 2);
    }

    #[tokio::test]
    async fn test_overall_health_calculation() {
        let config = Arc::new(ConfigLoader::new().without_env().create_default_config());
        let health_service = HealthService::new(config);

        // Initially should be starting
        let status = health_service.get_overall_health().await;
        assert!(matches!(status, HealthStatus::Starting));

        // Mark a critical component as unhealthy
        {
            let mut components = health_service.components.write().await;
            if let Some(component) = components.get_mut("database") {
                component.mark_unhealthy("Database down".to_string());
            }
        }

        let status = health_service.get_overall_health().await;
        assert!(matches!(status, HealthStatus::Unhealthy));
    }

    #[tokio::test]
    async fn test_config_health_check() {
        let config = Arc::new(ConfigLoader::new().without_env().create_default_config());
        let health_service = HealthService::new(config);

        health_service.check_config_health().await;

        let config_health = health_service.get_component_health("config").await;
        assert!(config_health.is_some());

        let component = config_health.unwrap();
        // Should be healthy since we're using a valid default config
        assert!(matches!(component.status, HealthStatus::Healthy));
    }

    #[tokio::test]
    async fn test_health_summary_generation() {
        let config = Arc::new(ConfigLoader::new().without_env().create_default_config());
        let health_service = HealthService::new(config);

        health_service.check_all_components().await;

        let summary = health_service.get_health_summary().await;
        assert!(!summary.is_empty());
        assert!(summary.contains("Overall Status:"));
        assert!(summary.contains("Components:"));
    }
}