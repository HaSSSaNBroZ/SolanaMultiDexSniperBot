//! Main application structure and lifecycle management
//!
//! This module contains the core Application struct that coordinates all services
//! and manages the application lifecycle from startup to shutdown.

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug, instrument};

use crate::config::AppConfig;
use crate::core::result::AppResult;
use crate::core::error::AppError;
use super::health::{HealthService, HealthStatus};

/// Main application state and coordinator
#[derive(Debug)]
pub struct Application {
    /// Application configuration
    config: Arc<AppConfig>,

    /// Health service for monitoring
    health_service: Arc<HealthService>,

    /// Application state
    state: Arc<RwLock<ApplicationState>>,
}

/// Application runtime state
#[derive(Debug, Clone)]
pub struct ApplicationState {
    /// Whether the application is running
    pub is_running: bool,

    /// Whether the application is shutting down
    pub is_shutting_down: bool,

    /// Start timestamp
    pub started_at: chrono::DateTime<chrono::Utc>,

    /// Last health check timestamp
    pub last_health_check: Option<chrono::DateTime<chrono::Utc>>,

    /// Current health status
    pub health_status: HealthStatus,
}

impl Default for ApplicationState {
    fn default() -> Self {
        Self {
            is_running: false,
            is_shutting_down: false,
            started_at: chrono::Utc::now(),
            last_health_check: None,
            health_status: HealthStatus::Starting,
        }
    }
}

impl Application {
    /// Build a new application instance with the given configuration
    #[instrument(skip(config))]
    pub async fn build(config: AppConfig) -> AppResult<Self> {
        info!("🏗️  Building application instance");

        // Validate configuration before proceeding
        let validation_result = config.validate()?;
        if !validation_result.is_valid {
            return Err(AppError::config(format!(
                "Configuration validation failed: {:?}",
                validation_result.errors
            )));
        }

        if !validation_result.warnings.is_empty() {
            for warning in &validation_result.warnings {
                warn!("⚠️  Configuration warning: {}", warning);
            }
        }

        let config = Arc::new(config);

        // Initialize health service
        let health_service = Arc::new(HealthService::new(config.clone()));

        // Initialize application state
        let state = Arc::new(RwLock::new(ApplicationState::default()));

        let app = Self {
            config,
            health_service,
            state,
        };

        info!("✅ Application instance built successfully");
        Ok(app)
    }

    /// Run the application main loop
    #[instrument(skip(self))]
    pub async fn run(self) -> AppResult<()> {
        info!("🚀 Starting Solana Sniper Bot application");

        // Update state to running
        {
            let mut state = self.state.write().await;
            state.is_running = true;
            state.health_status = HealthStatus::Healthy;
        }

        // Start health service
        let health_service = self.health_service.clone();
        let health_task = tokio::spawn(async move {
            if let Err(e) = health_service.start().await {
                error!("Health service failed: {}", e);
            }
        });

        // Start core services based on configuration
        let mut service_handles = Vec::new();

        // Start metrics server if enabled
        if self.config.analytics.enable_metrics {
            info!("📊 Starting metrics server on port {}", self.config.analytics.metrics_port);
            let metrics_handle = self.start_metrics_server().await?;
            service_handles.push(metrics_handle);
        }

        // Start health check server if enabled
        if self.config.monitoring.enable_health_checks {
            info!("🔍 Starting health check server on port {}", self.config.monitoring.health_check_port);
            let health_handle = self.start_health_server().await?;
            service_handles.push(health_handle);
        }

        info!("✅ All services started successfully");
        info!("🎯 Solana Sniper Bot is now running in {} mode", self.config.trading.scenario_mode);

        // Main application loop
        let main_loop = async {
            loop {
                // Check if we should shut down
                {
                    let state = self.state.read().await;
                    if state.is_shutting_down {
                        break;
                    }
                }

                // Perform periodic health checks
                if let Err(e) = self.perform_health_check().await {
                    warn!("Health check failed: {}", e);
                }

                // Sleep for a short interval
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        };

        // Wait for main loop or health service to complete
        tokio::select! {
            _ = main_loop => {
                info!("Main application loop completed");
            }
            result = health_task => {
                match result {
                    Ok(_) => info!("Health service completed"),
                    Err(e) => error!("Health service task failed: {}", e),
                }
            }
        }

        // Graceful shutdown
        self.shutdown().await?;

        info!("👋 Solana Sniper Bot application stopped");
        Ok(())
    }

    /// Start the metrics server
    async fn start_metrics_server(&self) -> AppResult<tokio::task::JoinHandle<()>> {
        let port = self.config.analytics.metrics_port;

        let handle = tokio::spawn(async move {
            info!("📊 Metrics server would start on port {} (Phase 3 implementation)", port);
            // TODO: Implement actual metrics server in Phase 3

            // For now, just log that it would be running
            loop {
                debug!("Metrics server heartbeat");
                tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
            }
        });

        Ok(handle)
    }

    /// Start the health check server
    async fn start_health_server(&self) -> AppResult<tokio::task::JoinHandle<()>> {
        let port = self.config.monitoring.health_check_port;
        let health_service = self.health_service.clone();

        let handle = tokio::spawn(async move {
            info!("🔍 Health check server would start on port {} (Phase 3 implementation)", port);
            // TODO: Implement actual health check HTTP server in Phase 3

            // For now, just log health status periodically
            loop {
                let status = health_service.get_overall_health().await;
                debug!("Health check: {:?}", status);
                tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            }
        });

        Ok(handle)
    }

    /// Perform a health check on all components
    async fn perform_health_check(&self) -> AppResult<()> {
        let health_status = self.health_service.get_overall_health().await;

        // Update state with health status
        {
            let mut state = self.state.write().await;
            state.last_health_check = Some(chrono::Utc::now());
            state.health_status = health_status;
        }

        match health_status {
            HealthStatus::Healthy => {
                debug!("✅ All systems healthy");
            }
            HealthStatus::Degraded => {
                warn!("⚠️  System degraded - some components unhealthy");
            }
            HealthStatus::Unhealthy => {
                error!("❌ System unhealthy - critical components failed");
            }
            HealthStatus::Starting => {
                debug!("🔄 System starting up");
            }
        }

        Ok(())
    }

    /// Initiate graceful shutdown
    #[instrument(skip(self))]
    pub async fn shutdown(&self) -> AppResult<()> {
        info!("🛑 Initiating graceful shutdown");

        // Mark as shutting down
        {
            let mut state = self.state.write().await;
            state.is_shutting_down = true;
            state.is_running = false;
        }

        // Stop health service
        if let Err(e) = self.health_service.stop().await {
            warn!("Failed to stop health service cleanly: {}", e);
        }

        info!("✅ Graceful shutdown completed");
        Ok(())
    }

    /// Get current application state
    pub async fn get_state(&self) -> ApplicationState {
        self.state.read().await.clone()
    }

    /// Get application configuration
    pub fn get_config(&self) -> &AppConfig {
        &self.config
    }

    /// Check if application is running
    pub async fn is_running(&self) -> bool {
        self.state.read().await.is_running
    }

    /// Check if application is shutting down
    pub async fn is_shutting_down(&self) -> bool {
        self.state.read().await.is_shutting_down
    }

    /// Get current health status
    pub async fn get_health_status(&self) -> HealthStatus {
        self.state.read().await.health_status
    }

    /// Get application uptime
    pub async fn get_uptime(&self) -> chrono::Duration {
        let state = self.state.read().await;
        chrono::Utc::now() - state.started_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConfigLoader;

    #[tokio::test]
    async fn test_application_build() {
        // Create a minimal test configuration
        let config = ConfigLoader::new().without_env().create_default_config();

        let app_result = Application::build(config).await;
        assert!(app_result.is_ok());

        let app = app_result.unwrap();
        assert!(!app.is_running().await);
        assert!(!app.is_shutting_down().await);
    }

    #[tokio::test]
    async fn test_application_state() {
        let config = ConfigLoader::new().without_env().create_default_config();
        let app = Application::build(config).await.unwrap();

        let state = app.get_state().await;
        assert!(!state.is_running);
        assert!(!state.is_shutting_down);
        assert!(matches!(state.health_status, HealthStatus::Starting));
    }

    #[tokio::test]
    async fn test_application_configuration_access() {
        let config = ConfigLoader::new().without_env().create_default_config();
        let expected_env = config.environment.name.clone();

        let app = Application::build(config).await.unwrap();

        assert_eq!(app.get_config().environment.name, expected_env);
    }

    #[tokio::test]
    async fn test_application_shutdown() {
        let config = ConfigLoader::new().without_env().create_default_config();
        let app = Application::build(config).await.unwrap();

        // Test shutdown
        let shutdown_result = app.shutdown().await;
        assert!(shutdown_result.is_ok());

        assert!(app.is_shutting_down().await);
        assert!(!app.is_running().await);
    }
}