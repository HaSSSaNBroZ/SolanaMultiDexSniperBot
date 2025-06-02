//! Monitoring infrastructure module
//!
//! This module provides comprehensive monitoring capabilities including
//! metrics collection, distributed tracing, and performance monitoring.

pub mod metrics;
pub mod tracing;

// Re-export main types
pub use metrics::{MetricsService, MetricsCollector, Timer, MetricsSummary};
pub use tracing::{TracingService, TraceSpan, TraceContext};

use crate::config::AppConfig;
use crate::core::result::AppResult;
use crate::application::health::ComponentHealth;
use std::sync::Arc;

/// Health tracker for monitoring service health
#[derive(Debug, Clone)]
pub struct HealthTracker {
    /// Metrics service
    metrics: Arc<MetricsService>,
    /// Tracing service
    tracing: Arc<TracingService>,
}

impl HealthTracker {
    /// Create a new health tracker
    pub fn new(metrics: Arc<MetricsService>, tracing: Arc<TracingService>) -> Self {
        Self { metrics, tracing }
    }

    /// Perform comprehensive health check
    pub async fn check_health(&self) -> ComponentHealth {
        let mut component = ComponentHealth::new("monitoring".to_string(), false);
        let start_time = std::time::Instant::now();

        // Check metrics health
        let metrics_health = self.metrics.health_check().await;
        let tracing_health = self.tracing.health_check().await;

        let response_time = start_time.elapsed().as_millis() as u64;

        // Determine overall health
        match (metrics_health.status, tracing_health.status) {
            (crate::application::health::HealthStatus::Healthy,
                crate::application::health::HealthStatus::Healthy) => {
                component.mark_healthy(
                    Some("All monitoring systems operational".to_string()),
                    Some(response_time)
                );
            }
            _ => {
                component.mark_degraded(
                    "Some monitoring components degraded".to_string(),
                    Some(response_time)
                );
            }
        }

        component
    }

    /// Get monitoring statistics
    pub async fn get_statistics(&self) -> MonitoringStatistics {
        let metrics_summary = self.metrics.get_summary().await;
        let tracing_stats = self.tracing.get_statistics().await;

        MonitoringStatistics {
            metrics: metrics_summary,
            spans_created: tracing_stats.spans_created,
            spans_completed: tracing_stats.spans_completed,
            spans_active: tracing_stats.spans_active,
            events_recorded: tracing_stats.events_recorded,
        }
    }
}

/// Combined monitoring statistics
#[derive(Debug, Clone)]
pub struct MonitoringStatistics {
    /// Metrics summary
    pub metrics: MetricsSummary,
    /// Total spans created
    pub spans_created: u64,
    /// Total spans completed
    pub spans_completed: u64,
    /// Currently active spans
    pub spans_active: u64,
    /// Total events recorded
    pub events_recorded: u64,
}

/// Monitoring service coordinator
#[derive(Debug, Clone)]
pub struct MonitoringService {
    /// Metrics collection service
    pub metrics: Arc<MetricsService>,
    /// Distributed tracing service
    pub tracing: Arc<TracingService>,
    /// Health tracking
    pub health_tracker: Arc<HealthTracker>,
}

impl MonitoringService {
    /// Initialize monitoring services
    pub fn new(config: &AppConfig) -> AppResult<Self> {
        tracing::info!("📊 Initializing monitoring services");

        // Initialize metrics
        let metrics = Arc::new(MetricsService::new(config)?);

        // Initialize tracing
        let tracing_service = Arc::new(TracingService::new(config)?);

        // Create health tracker
        let health_tracker = Arc::new(HealthTracker::new(
            metrics.clone(),
            tracing_service.clone()
        ));

        tracing::info!("✅ Monitoring services initialized");

        Ok(Self {
            metrics,
            tracing: tracing_service,
            health_tracker,
        })
    }

    /// Start monitoring services
    pub async fn start(&self) -> AppResult<()> {
        tracing::info!("🚀 Starting monitoring services");

        // Start metrics collection
        self.metrics.update_system_metrics().await;

        // Initialize tracing spans
        self.tracing.initialize_root_span().await?;

        tracing::info!("✅ Monitoring services started");
        Ok(())
    }

    /// Stop monitoring services
    pub async fn stop(&self) -> AppResult<()> {
        tracing::info!("🛑 Stopping monitoring services");

        // Flush metrics
        self.metrics.flush().await?;

        // Flush traces
        self.tracing.flush().await?;

        tracing::info!("✅ Monitoring services stopped");
        Ok(())
    }

    /// Get overall health status
    pub async fn health_check(&self) -> ComponentHealth {
        self.health_tracker.check_health().await
    }

    /// Get monitoring statistics
    pub async fn get_statistics(&self) -> MonitoringStatistics {
        self.health_tracker.get_statistics().await
    }

    /// Create a performance monitor for a specific operation
    pub fn monitor_operation(&self, operation_name: &str) -> OperationMonitor {
        OperationMonitor::new(
            operation_name.to_string(),
            self.metrics.clone(),
            self.tracing.clone()
        )
    }
}

/// Operation performance monitor
pub struct OperationMonitor {
    operation_name: String,
    metrics: Arc<MetricsService>,
    tracing: Arc<TracingService>,
    span: Option<TraceSpan>,
    start_time: std::time::Instant,
}

impl OperationMonitor {
    fn new(operation_name: String, metrics: Arc<MetricsService>, tracing: Arc<TracingService>) -> Self {
        let span = tracing.create_span(&operation_name);
        Self {
            operation_name,
            metrics,
            tracing,
            span: Some(span),
            start_time: std::time::Instant::now(),
        }
    }

    /// Record successful completion
    pub fn success(mut self) {
        let duration = self.start_time.elapsed();

        // Record metrics
        let mut labels = std::collections::HashMap::new();
        labels.insert("operation".to_string(), self.operation_name.clone());
        labels.insert("status".to_string(), "success".to_string());

        self.metrics.record_timer("operation_duration_seconds", duration, Some(labels.clone()));
        self.metrics.increment_counter("operation_total", Some(labels));

        // Complete span
        if let Some(span) = self.span.take() {
            span.success();
        }
    }

    /// Record failure
    pub fn failure(mut self, error: &str) {
        let duration = self.start_time.elapsed();

        // Record metrics
        let mut labels = std::collections::HashMap::new();
        labels.insert("operation".to_string(), self.operation_name.clone());
        labels.insert("status".to_string(), "failure".to_string());

        self.metrics.record_timer("operation_duration_seconds", duration, Some(labels.clone()));
        self.metrics.increment_counter("operation_total", Some(labels.clone()));
        self.metrics.increment_counter("operation_failures_total", Some(labels));

        // Complete span with error
        if let Some(span) = self.span.take() {
            span.error(error);
        }
    }

    /// Add an event to the operation
    pub fn add_event(&self, event_name: &str, attributes: Option<std::collections::HashMap<String, String>>) {
        if let Some(ref span) = self.span {
            span.add_event(event_name, attributes);
        }
    }

    /// Set an attribute on the operation
    pub fn set_attribute(&self, key: &str, value: &str) {
        if let Some(ref span) = self.span {
            span.set_attribute(key, value);
        }
    }
}

impl Drop for OperationMonitor {
    fn drop(&mut self) {
        // If not explicitly completed, mark as cancelled
        if self.span.is_some() {
            let duration = self.start_time.elapsed();

            let mut labels = std::collections::HashMap::new();
            labels.insert("operation".to_string(), self.operation_name.clone());
            labels.insert("status".to_string(), "cancelled".to_string());

            self.metrics.record_timer("operation_duration_seconds", duration, Some(labels.clone()));
            self.metrics.increment_counter("operation_total", Some(labels));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConfigLoader;

    #[tokio::test]
    async fn test_monitoring_service_creation() {
        let config = ConfigLoader::new().without_env().create_default_config();
        let result = MonitoringService::new(&config);

        assert!(result.is_ok());
        let service = result.unwrap();

        // Test health check
        let health = service.health_check().await;
        assert_eq!(health.name, "monitoring");
    }

    #[tokio::test]
    async fn test_operation_monitor() {
        let config = ConfigLoader::new().without_env().create_default_config();
        let service = MonitoringService::new(&config).unwrap();

        // Test successful operation
        {
            let monitor = service.monitor_operation("test_operation");
            monitor.add_event("processing", None);
            monitor.set_attribute("items", "100");
            monitor.success();
        }

        // Test failed operation
        {
            let monitor = service.monitor_operation("test_failed_operation");
            monitor.failure("Test error");
        }

        // Get statistics
        let stats = service.get_statistics().await;
        assert!(stats.metrics.total_metrics > 0);
    }
}