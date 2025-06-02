//! Metrics collection and reporting service
//!
//! This module provides Prometheus metrics collection, custom metrics,
//! and performance tracking capabilities.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn, error, instrument};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::config::AppConfig;
use crate::core::result::AppResult;
use crate::core::error::AppError;
use crate::application::health::{ComponentHealth, HealthStatus};

/// Metrics service for collecting and exposing application metrics
#[derive(Debug, Clone)]
pub struct MetricsService {
    /// Metrics collector
    collector: Arc<MetricsCollector>,
    /// Configuration
    config: Arc<AppConfig>,
    /// Service start time
    start_time: SystemTime,
}

impl MetricsService {
    /// Create a new metrics service
    pub fn new(config: &AppConfig) -> AppResult<Self> {
        info!("📊 Initializing metrics service");

        let collector = Arc::new(MetricsCollector::new());
        let start_time = SystemTime::now();

        // Initialize default metrics
        collector.register_counter("sniper_bot_requests_total", "Total number of requests");
        collector.register_counter("sniper_bot_trades_total", "Total number of trades executed");
        collector.register_counter("sniper_bot_errors_total", "Total number of errors");

        collector.register_histogram("sniper_bot_request_duration_seconds", "Request duration in seconds");
        collector.register_histogram("sniper_bot_trade_execution_duration_seconds", "Trade execution duration in seconds");
        collector.register_histogram("sniper_bot_token_detection_duration_seconds", "Token detection duration in seconds");

        collector.register_gauge("sniper_bot_active_sessions", "Number of active trading sessions");
        collector.register_gauge("sniper_bot_open_positions", "Number of open positions");
        collector.register_gauge("sniper_bot_wallet_balance_sol", "Wallet balance in SOL");
        collector.register_gauge("sniper_bot_uptime_seconds", "Application uptime in seconds");

        info!("✅ Metrics service initialized");

        Ok(Self {
            collector,
            config: Arc::new(config.clone()),
            start_time,
        })
    }

    /// Get the metrics collector
    pub fn collector(&self) -> Arc<MetricsCollector> {
        self.collector.clone()
    }

    /// Increment a counter metric
    pub fn increment_counter(&self, name: &str, labels: Option<HashMap<String, String>>) {
        self.collector.increment_counter(name, labels);
    }

    /// Add value to histogram
    pub fn record_histogram(&self, name: &str, value: f64, labels: Option<HashMap<String, String>>) {
        self.collector.record_histogram(name, value, labels);
    }

    /// Set gauge value
    pub fn set_gauge(&self, name: &str, value: f64, labels: Option<HashMap<String, String>>) {
        self.collector.set_gauge(name, value, labels);
    }

    /// Record a timer measurement
    pub fn record_timer(&self, name: &str, duration: Duration, labels: Option<HashMap<String, String>>) {
        self.record_histogram(name, duration.as_secs_f64(), labels);
    }

    /// Start a timer
    pub fn start_timer(&self, name: &str) -> Timer {
        Timer::new(name.to_string(), self.collector.clone())
    }

    /// Record trade metrics
    #[instrument(skip(self))]
    pub fn record_trade(&self, success: bool, execution_time: Duration, dex: &str, pnl_sol: f64) {
        let mut labels = HashMap::new();
        labels.insert("dex".to_string(), dex.to_string());
        labels.insert("success".to_string(), success.to_string());

        self.increment_counter("sniper_bot_trades_total", Some(labels.clone()));
        self.record_timer("sniper_bot_trade_execution_duration_seconds", execution_time, Some(labels.clone()));

        if success {
            labels.insert("type".to_string(), "pnl".to_string());
            self.set_gauge("sniper_bot_trade_pnl_sol", pnl_sol, Some(labels));
        } else {
            self.increment_counter("sniper_bot_trade_failures_total", Some(labels));
        }

        debug!("📊 Recorded trade metrics: success={}, execution_time={}ms, dex={}, pnl={}",
              success, execution_time.as_millis(), dex, pnl_sol);
    }

    /// Record token detection metrics
    #[instrument(skip(self))]
    pub fn record_token_detection(&self, detection_time: Duration, tokens_found: u32, source: &str) {
        let mut labels = HashMap::new();
        labels.insert("source".to_string(), source.to_string());

        self.record_timer("sniper_bot_token_detection_duration_seconds", detection_time, Some(labels.clone()));
        self.set_gauge("sniper_bot_tokens_detected_total", tokens_found as f64, Some(labels));

        debug!("📊 Recorded token detection metrics: time={}ms, found={}, source={}",
              detection_time.as_millis(), tokens_found, source);
    }

    /// Record error metrics
    #[instrument(skip(self))]
    pub fn record_error(&self, error_type: &str, component: &str) {
        let mut labels = HashMap::new();
        labels.insert("error_type".to_string(), error_type.to_string());
        labels.insert("component".to_string(), component.to_string());

        self.increment_counter("sniper_bot_errors_total", Some(labels));

        debug!("📊 Recorded error metrics: type={}, component={}", error_type, component);
    }

    /// Update system metrics
    #[instrument(skip(self))]
    pub async fn update_system_metrics(&self) {
        // Update uptime
        if let Ok(elapsed) = self.start_time.elapsed() {
            self.set_gauge("sniper_bot_uptime_seconds", elapsed.as_secs() as f64, None);
        }

        // Update memory usage (simplified - in production would use system metrics)
        if let Ok(memory_info) = self.get_memory_usage() {
            self.set_gauge("sniper_bot_memory_usage_bytes", memory_info.used_bytes as f64, None);
            self.set_gauge("sniper_bot_memory_usage_percent", memory_info.usage_percent, None);
        }

        debug!("📊 Updated system metrics");
    }

    /// Get memory usage information
    fn get_memory_usage(&self) -> AppResult<MemoryInfo> {
        // Simplified memory tracking - in production would use proper system metrics
        // This is a placeholder implementation
        Ok(MemoryInfo {
            used_bytes: 0,
            total_bytes: 0,
            usage_percent: 0.0,
        })
    }

    /// Export metrics in Prometheus format
    pub async fn export_prometheus(&self) -> String {
        self.collector.export_prometheus().await
    }

    /// Get metrics summary
    pub async fn get_summary(&self) -> MetricsSummary {
        self.collector.get_summary().await
    }

    /// Health check for metrics service
    pub async fn health_check(&self) -> ComponentHealth {
        let mut component = ComponentHealth::new("metrics".to_string(), false);
        let start_time = Instant::now();

        // Simple health check - verify we can collect metrics
        let summary = self.get_summary().await;
        let response_time = start_time.elapsed().as_millis() as u64;

        if summary.total_metrics > 0 {
            component.mark_healthy(
                Some(format!("Metrics service healthy, {} metrics collected", summary.total_metrics)),
                Some(response_time)
            );
        } else {
            component.mark_degraded(
                "Metrics service degraded - no metrics collected".to_string(),
                Some(response_time)
            );
        }

        component
    }

    /// Flush metrics (prepare for shutdown)
    pub async fn flush(&self) -> AppResult<()> {
        info!("📊 Flushing metrics");
        // In a real implementation, this would flush metrics to external systems
        Ok(())
    }
}

/// Metrics collector implementation
#[derive(Debug)]
pub struct MetricsCollector {
    /// Counter metrics
    counters: Arc<RwLock<HashMap<String, CounterMetric>>>,
    /// Histogram metrics
    histograms: Arc<RwLock<HashMap<String, HistogramMetric>>>,
    /// Gauge metrics
    gauges: Arc<RwLock<HashMap<String, GaugeMetric>>>,
    /// Metric metadata
    metadata: Arc<RwLock<HashMap<String, MetricMetadata>>>,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            counters: Arc::new(RwLock::new(HashMap::new())),
            histograms: Arc::new(RwLock::new(HashMap::new())),
            gauges: Arc::new(RwLock::new(HashMap::new())),
            metadata: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a counter metric
    pub fn register_counter(&self, name: &str, description: &str) {
        let rt = tokio::runtime::Handle::current();
        rt.spawn({
            let name = name.to_string();
            let description = description.to_string();
            let metadata = self.metadata.clone();
            let counters = self.counters.clone();

            async move {
                let mut metadata_map = metadata.write().await;
                metadata_map.insert(name.clone(), MetricMetadata {
                    metric_type: MetricType::Counter,
                    description,
                });

                let mut counters_map = counters.write().await;
                counters_map.insert(name, CounterMetric::new());
            }
        });
    }

    /// Register a histogram metric
    pub fn register_histogram(&self, name: &str, description: &str) {
        let rt = tokio::runtime::Handle::current();
        rt.spawn({
            let name = name.to_string();
            let description = description.to_string();
            let metadata = self.metadata.clone();
            let histograms = self.histograms.clone();

            async move {
                let mut metadata_map = metadata.write().await;
                metadata_map.insert(name.clone(), MetricMetadata {
                    metric_type: MetricType::Histogram,
                    description,
                });

                let mut histograms_map = histograms.write().await;
                histograms_map.insert(name, HistogramMetric::new());
            }
        });
    }

    /// Register a gauge metric
    pub fn register_gauge(&self, name: &str, description: &str) {
        let rt = tokio::runtime::Handle::current();
        rt.spawn({
            let name = name.to_string();
            let description = description.to_string();
            let metadata = self.metadata.clone();
            let gauges = self.gauges.clone();

            async move {
                let mut metadata_map = metadata.write().await;
                metadata_map.insert(name.clone(), MetricMetadata {
                    metric_type: MetricType::Gauge,
                    description,
                });

                let mut gauges_map = gauges.write().await;
                gauges_map.insert(name, GaugeMetric::new());
            }
        });
    }

    /// Increment a counter
    pub fn increment_counter(&self, name: &str, labels: Option<HashMap<String, String>>) {
        let rt = tokio::runtime::Handle::current();
        rt.spawn({
            let name = name.to_string();
            let counters = self.counters.clone();

            async move {
                let mut counters_map = counters.write().await;
                if let Some(counter) = counters_map.get_mut(&name) {
                    counter.increment(labels);
                }
            }
        });
    }

    /// Record histogram value
    pub fn record_histogram(&self, name: &str, value: f64, labels: Option<HashMap<String, String>>) {
        let rt = tokio::runtime::Handle::current();
        rt.spawn({
            let name = name.to_string();
            let histograms = self.histograms.clone();

            async move {
                let mut histograms_map = histograms.write().await;
                if let Some(histogram) = histograms_map.get_mut(&name) {
                    histogram.record(value, labels);
                }
            }
        });
    }

    /// Set gauge value
    pub fn set_gauge(&self, name: &str, value: f64, labels: Option<HashMap<String, String>>) {
        let rt = tokio::runtime::Handle::current();
        rt.spawn({
            let name = name.to_string();
            let gauges = self.gauges.clone();

            async move {
                let mut gauges_map = gauges.write().await;
                if let Some(gauge) = gauges_map.get_mut(&name) {
                    gauge.set(value, labels);
                }
            }
        });
    }

    /// Export metrics in Prometheus format
    pub async fn export_prometheus(&self) -> String {
        let mut output = String::new();

        // Export counters
        let counters = self.counters.read().await;
        let metadata = self.metadata.read().await;

        for (name, counter) in counters.iter() {
            if let Some(meta) = metadata.get(name) {
                output.push_str(&format!("# HELP {} {}\n", name, meta.description));
                output.push_str(&format!("# TYPE {} counter\n", name));
                output.push_str(&counter.to_prometheus_string(name));
            }
        }

        // Export histograms
        let histograms = self.histograms.read().await;
        for (name, histogram) in histograms.iter() {
            if let Some(meta) = metadata.get(name) {
                output.push_str(&format!("# HELP {} {}\n", name, meta.description));
                output.push_str(&format!("# TYPE {} histogram\n", name));
                output.push_str(&histogram.to_prometheus_string(name));
            }
        }

        // Export gauges
        let gauges = self.gauges.read().await;
        for (name, gauge) in gauges.iter() {
            if let Some(meta) = metadata.get(name) {
                output.push_str(&format!("# HELP {} {}\n", name, meta.description));
                output.push_str(&format!("# TYPE {} gauge\n", name));
                output.push_str(&gauge.to_prometheus_string(name));
            }
        }

        output
    }

    /// Get metrics summary
    pub async fn get_summary(&self) -> MetricsSummary {
        let counters = self.counters.read().await;
        let histograms = self.histograms.read().await;
        let gauges = self.gauges.read().await;

        MetricsSummary {
            total_metrics: counters.len() + histograms.len() + gauges.len(),
            counters: counters.len(),
            histograms: histograms.len(),
            gauges: gauges.len(),
        }
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Timer for measuring execution time
pub struct Timer {
    name: String,
    start_time: Instant,
    collector: Arc<MetricsCollector>,
}

impl Timer {
    fn new(name: String, collector: Arc<MetricsCollector>) -> Self {
        Self {
            name,
            start_time: Instant::now(),
            collector,
        }
    }

    /// Stop the timer and record the measurement
    pub fn stop(self) {
        let duration = self.start_time.elapsed();
        self.collector.record_histogram(&self.name, duration.as_secs_f64(), None);
    }

    /// Stop the timer with labels and record the measurement
    pub fn stop_with_labels(self, labels: HashMap<String, String>) {
        let duration = self.start_time.elapsed();
        self.collector.record_histogram(&self.name, duration.as_secs_f64(), Some(labels));
    }
}

/// Counter metric implementation
#[derive(Debug)]
struct CounterMetric {
    values: HashMap<String, f64>,
}

impl CounterMetric {
    fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    fn increment(&mut self, labels: Option<HashMap<String, String>>) {
        let key = Self::labels_to_key(labels);
        *self.values.entry(key).or_insert(0.0) += 1.0;
    }

    fn to_prometheus_string(&self, name: &str) -> String {
        let mut output = String::new();
        for (labels_key, value) in &self.values {
            if labels_key.is_empty() {
                output.push_str(&format!("{} {}\n", name, value));
            } else {
                output.push_str(&format!("{}{} {}\n", name, labels_key, value));
            }
        }
        output
    }

    fn labels_to_key(labels: Option<HashMap<String, String>>) -> String {
        match labels {
            Some(labels) if !labels.is_empty() => {
                let mut pairs: Vec<_> = labels.iter().collect();
                pairs.sort_by_key(|(k, _)| *k);
                let label_str = pairs
                    .iter()
                    .map(|(k, v)| format!("{}=\"{}\"", k, v))
                    .collect::<Vec<_>>()
                    .join(",");
                format!("{{{}}}", label_str)
            }
            _ => String::new(),
        }
    }
}

/// Histogram metric implementation
#[derive(Debug)]
struct HistogramMetric {
    buckets: HashMap<String, Vec<f64>>,
}

impl HistogramMetric {
    fn new() -> Self {
        Self {
            buckets: HashMap::new(),
        }
    }

    fn record(&mut self, value: f64, labels: Option<HashMap<String, String>>) {
        let key = CounterMetric::labels_to_key(labels);
        self.buckets.entry(key).or_insert_with(Vec::new).push(value);
    }

    fn to_prometheus_string(&self, name: &str) -> String {
        let mut output = String::new();
        for (labels_key, values) in &self.buckets {
            let count = values.len();
            let sum: f64 = values.iter().sum();

            if labels_key.is_empty() {
                output.push_str(&format!("{}_count {}\n", name, count));
                output.push_str(&format!("{}_sum {}\n", name, sum));
            } else {
                output.push_str(&format!("{}_count{} {}\n", name, labels_key, count));
                output.push_str(&format!("{}_sum{} {}\n", name, labels_key, sum));
            }
        }
        output
    }
}

/// Gauge metric implementation
#[derive(Debug)]
struct GaugeMetric {
    values: HashMap<String, f64>,
}

impl GaugeMetric {
    fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    fn set(&mut self, value: f64, labels: Option<HashMap<String, String>>) {
        let key = CounterMetric::labels_to_key(labels);
        self.values.insert(key, value);
    }

    fn to_prometheus_string(&self, name: &str) -> String {
        let mut output = String::new();
        for (labels_key, value) in &self.values {
            if labels_key.is_empty() {
                output.push_str(&format!("{} {}\n", name, value));
            } else {
                output.push_str(&format!("{}{} {}\n", name, labels_key, value));
            }
        }
        output
    }
}

/// Metric metadata
#[derive(Debug, Clone)]
struct MetricMetadata {
    metric_type: MetricType,
    description: String,
}

/// Metric types
#[derive(Debug, Clone)]
enum MetricType {
    Counter,
    Histogram,
    Gauge,
}

/// Memory usage information
#[derive(Debug)]
struct MemoryInfo {
    used_bytes: u64,
    total_bytes: u64,
    usage_percent: f64,
}

/// Metrics summary
#[derive(Debug, Clone)]
pub struct MetricsSummary {
    pub total_metrics: usize,
    pub counters: usize,
    pub histograms: usize,
    pub gauges: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConfigLoader;

    #[tokio::test]
    async fn test_metrics_service_creation() {
        let config = ConfigLoader::new().without_env().create_default_config();
        let metrics_service = MetricsService::new(&config);

        assert!(metrics_service.is_ok());
        let service = metrics_service.unwrap();

        // Test basic operations
        service.increment_counter("test_counter", None);
        service.record_histogram("test_histogram", 1.0, None);
        service.set_gauge("test_gauge", 42.0, None);

        let summary = service.get_summary().await;
        assert!(summary.total_metrics > 0);
    }

    #[tokio::test]
    async fn test_timer() {
        let config = ConfigLoader::new().without_env().create_default_config();
        let metrics_service = MetricsService::new(&config).unwrap();

        let timer = metrics_service.start_timer("test_timer");
        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
        timer.stop();

        // Timer should have been recorded
        let prometheus = metrics_service.export_prometheus().await;
        assert!(prometheus.contains("test_timer"));
    }

    #[test]
    fn test_counter_metric() {
        let mut counter = CounterMetric::new();
        counter.increment(None);
        counter.increment(None);

        let prometheus = counter.to_prometheus_string("test_counter");
        assert!(prometheus.contains("test_counter 2"));
    }
}