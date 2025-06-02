//! Distributed tracing service implementation
//!
//! This module provides OpenTelemetry-compatible distributed tracing
//! with support for spans, events, and context propagation.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};
use tracing::{debug, info, warn, error, instrument, Span};
use tracing_subscriber::{Layer, Registry, EnvFilter};
use std::time::{Duration, Instant};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::config::AppConfig;
use crate::core::result::AppResult;
use crate::core::error::AppError;
use crate::application::health::{ComponentHealth, HealthStatus};

/// Distributed tracing service
#[derive(Debug, Clone)]
pub struct TracingService {
    /// Active spans tracking
    active_spans: Arc<RwLock<HashMap<String, ActiveSpan>>>,
    /// Completed spans buffer
    completed_spans: Arc<RwLock<Vec<CompletedSpan>>>,
    /// Service configuration
    config: Arc<AppConfig>,
    /// Tracing statistics
    stats: Arc<Mutex<TracingStats>>,
    /// Root span ID
    root_span_id: Arc<RwLock<Option<String>>>,
}

impl TracingService {
    /// Create a new tracing service
    pub fn new(config: &AppConfig) -> AppResult<Self> {
        info!("📍 Initializing tracing service");

        let service = Self {
            active_spans: Arc::new(RwLock::new(HashMap::new())),
            completed_spans: Arc::new(RwLock::new(Vec::new())),
            config: Arc::new(config.clone()),
            stats: Arc::new(Mutex::new(TracingStats::new())),
            root_span_id: Arc::new(RwLock::new(None)),
        };

        info!("✅ Tracing service initialized");
        Ok(service)
    }

    /// Initialize root span for the application
    pub async fn initialize_root_span(&self) -> AppResult<()> {
        let root_span = self.create_span("sniper_bot_root");
        root_span.set_attribute("version", env!("CARGO_PKG_VERSION"));
        root_span.set_attribute("environment", &self.config.environment.name);
        root_span.set_attribute("scenario_mode", &self.config.trading.scenario_mode.to_string());

        let span_id = root_span.id.clone();
        {
            let mut root_id = self.root_span_id.write().await;
            *root_id = Some(span_id);
        }

        Ok(())
    }

    /// Create a new trace span
    pub fn create_span(&self, name: &str) -> TraceSpan {
        self.create_child_span(name, None)
    }

    /// Create a child span
    pub fn create_child_span(&self, name: &str, parent_id: Option<String>) -> TraceSpan {
        let span_id = Uuid::new_v4().to_string();
        let parent_span_id = parent_id.or_else(|| {
            if let Ok(root_id) = self.root_span_id.try_read() {
                root_id.clone()
            } else {
                None
            }
        });

        let active_span = ActiveSpan {
            id: span_id.clone(),
            name: name.to_string(),
            parent_id: parent_span_id.clone(),
            start_time: Instant::now(),
            started_at: Utc::now(),
            attributes: HashMap::new(),
            events: Vec::new(),
            status: SpanStatus::InProgress,
        };

        // Store in active spans
        let rt = tokio::runtime::Handle::current();
        let active_spans = self.active_spans.clone();
        let span_id_clone = span_id.clone();
        rt.spawn(async move {
            let mut spans = active_spans.write().await;
            spans.insert(span_id_clone, active_span);
        });

        // Update statistics
        let stats = self.stats.clone();
        rt.spawn(async move {
            let mut s = stats.lock().await;
            s.spans_created += 1;
            s.spans_active += 1;
        });

        TraceSpan::new(span_id, self.clone())
    }

    /// Complete a span
    async fn complete_span(&self, span_id: &str, status: SpanStatus, error_message: Option<String>) {
        let active_span = {
            let mut spans = self.active_spans.write().await;
            spans.remove(span_id)
        };

        if let Some(mut span) = active_span {
            let duration = span.start_time.elapsed();
            span.status = status;

            let completed = CompletedSpan {
                id: span.id,
                name: span.name,
                parent_id: span.parent_id,
                started_at: span.started_at,
                completed_at: Utc::now(),
                duration,
                attributes: span.attributes,
                events: span.events,
                status: span.status,
                error_message,
            };

            // Store completed span
            {
                let mut completed_spans = self.completed_spans.write().await;
                completed_spans.push(completed.clone());

                // Keep only recent spans (last 1000)
                if completed_spans.len() > 1000 {
                    completed_spans.drain(0..100);
                }
            }

            // Update statistics
            {
                let mut stats = self.stats.lock().await;
                stats.spans_completed += 1;
                stats.spans_active = stats.spans_active.saturating_sub(1);

                match status {
                    SpanStatus::Success => stats.spans_successful += 1,
                    SpanStatus::Error => stats.spans_failed += 1,
                    SpanStatus::Cancelled => stats.spans_cancelled += 1,
                    _ => {}
                }
            }

            // Log span completion
            match status {
                SpanStatus::Success => {
                    debug!("📍 Span completed: {} ({}ms)", completed.name, duration.as_millis());
                }
                SpanStatus::Error => {
                    warn!("📍 Span failed: {} ({}ms) - {:?}",
                          completed.name, duration.as_millis(), error_message);
                }
                _ => {}
            }
        }
    }

    /// Add event to a span
    async fn add_span_event(&self, span_id: &str, event: SpanEvent) {
        let mut spans = self.active_spans.write().await;
        if let Some(span) = spans.get_mut(span_id) {
            span.events.push(event);
        }

        let mut stats = self.stats.lock().await;
        stats.events_recorded += 1;
    }

    /// Set span attribute
    async fn set_span_attribute(&self, span_id: &str, key: String, value: String) {
        let mut spans = self.active_spans.write().await;
        if let Some(span) = spans.get_mut(span_id) {
            span.attributes.insert(key, value);
        }
    }

    /// Get tracing statistics
    pub async fn get_statistics(&self) -> TracingStats {
        self.stats.lock().await.clone()
    }

    /// Get active spans
    pub async fn get_active_spans(&self) -> Vec<SpanInfo> {
        let spans = self.active_spans.read().await;
        spans.values().map(|s| SpanInfo {
            id: s.id.clone(),
            name: s.name.clone(),
            parent_id: s.parent_id.clone(),
            duration: s.start_time.elapsed(),
            attributes: s.attributes.clone(),
        }).collect()
    }

    /// Get completed spans
    pub async fn get_completed_spans(&self, limit: Option<usize>) -> Vec<CompletedSpan> {
        let spans = self.completed_spans.read().await;
        let limit = limit.unwrap_or(100);

        spans.iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    /// Export spans in OpenTelemetry format
    pub async fn export_spans(&self) -> AppResult<Vec<ExportedSpan>> {
        let completed = self.completed_spans.read().await;

        let exported: Vec<ExportedSpan> = completed.iter()
            .map(|span| ExportedSpan {
                trace_id: Uuid::new_v4().to_string(), // In production, use proper trace ID
                span_id: span.id.clone(),
                parent_span_id: span.parent_id.clone(),
                operation_name: span.name.clone(),
                start_time: span.started_at,
                end_time: span.completed_at,
                duration_ms: span.duration.as_millis() as u64,
                attributes: span.attributes.clone(),
                events: span.events.iter().map(|e| ExportedEvent {
                    name: e.name.clone(),
                    timestamp: e.timestamp,
                    attributes: e.attributes.clone(),
                }).collect(),
                status: match span.status {
                    SpanStatus::Success => "OK",
                    SpanStatus::Error => "ERROR",
                    SpanStatus::Cancelled => "CANCELLED",
                    _ => "UNKNOWN",
                }.to_string(),
                error_message: span.error_message.clone(),
            })
            .collect();

        Ok(exported)
    }

    /// Health check for tracing service
    pub async fn health_check(&self) -> ComponentHealth {
        let mut component = ComponentHealth::new("tracing".to_string(), false);
        let start_time = Instant::now();

        let stats = self.get_statistics().await;
        let active_spans = self.active_spans.read().await.len();
        let response_time = start_time.elapsed().as_millis() as u64;

        // Check for memory leaks (too many active spans)
        if active_spans > 1000 {
            component.mark_degraded(
                format!("Too many active spans: {}", active_spans),
                Some(response_time)
            );
        } else {
            component.mark_healthy(
                Some(format!("Tracing healthy: {} active spans", active_spans)),
                Some(response_time)
            );
        }

        component
    }

    /// Flush all pending traces
    pub async fn flush(&self) -> AppResult<()> {
        info!("📍 Flushing traces");

        // Complete any remaining active spans
        let active_span_ids: Vec<String> = {
            let spans = self.active_spans.read().await;
            spans.keys().cloned().collect()
        };

        for span_id in active_span_ids {
            self.complete_span(&span_id, SpanStatus::Cancelled, Some("Service shutdown".to_string())).await;
        }

        // In production, would export to external system here
        let exported = self.export_spans().await?;
        debug!("📍 Exported {} spans", exported.len());

        Ok(())
    }
}

/// Trace span handle
#[derive(Debug)]
pub struct TraceSpan {
    id: String,
    service: TracingService,
    completed: Arc<Mutex<bool>>,
}

impl TraceSpan {
    fn new(id: String, service: TracingService) -> Self {
        Self {
            id,
            service,
            completed: Arc::new(Mutex::new(false)),
        }
    }

    /// Get span ID
    pub fn span_id(&self) -> &str {
        &self.id
    }

    /// Add an event to the span
    pub fn add_event(&self, name: &str, attributes: Option<HashMap<String, String>>) {
        let event = SpanEvent {
            name: name.to_string(),
            timestamp: Utc::now(),
            attributes: attributes.unwrap_or_default(),
        };

        let service = self.service.clone();
        let span_id = self.id.clone();
        tokio::spawn(async move {
            service.add_span_event(&span_id, event).await;
        });
    }

    /// Set an attribute on the span
    pub fn set_attribute(&self, key: &str, value: &str) {
        let service = self.service.clone();
        let span_id = self.id.clone();
        let key = key.to_string();
        let value = value.to_string();

        tokio::spawn(async move {
            service.set_span_attribute(&span_id, key, value).await;
        });
    }

    /// Complete the span successfully
    pub fn success(self) {
        self.complete(SpanStatus::Success, None);
    }

    /// Complete the span with error
    pub fn error(self, message: &str) {
        self.complete(SpanStatus::Error, Some(message.to_string()));
    }

    /// Complete the span
    fn complete(self, status: SpanStatus, error_message: Option<String>) {
        let service = self.service.clone();
        let span_id = self.id.clone();
        let completed = self.completed.clone();

        tokio::spawn(async move {
            let mut is_completed = completed.lock().await;
            if !*is_completed {
                *is_completed = true;
                service.complete_span(&span_id, status, error_message).await;
            }
        });
    }

    /// Create a child span
    pub fn child(&self, name: &str) -> TraceSpan {
        self.service.create_child_span(name, Some(self.id.clone()))
    }
}

impl Drop for TraceSpan {
    fn drop(&mut self) {
        // Auto-complete if not already completed
        let completed = self.completed.clone();
        let service = self.service.clone();
        let span_id = self.id.clone();

        tokio::spawn(async move {
            let mut is_completed = completed.lock().await;
            if !*is_completed {
                *is_completed = true;
                service.complete_span(&span_id, SpanStatus::Cancelled, None).await;
            }
        });
    }
}

/// Trace context for propagation
#[derive(Debug, Clone)]
pub struct TraceContext {
    /// Trace ID
    pub trace_id: String,
    /// Parent span ID
    pub parent_span_id: String,
    /// Baggage items
    pub baggage: HashMap<String, String>,
}

impl TraceContext {
    /// Create from HTTP headers
    pub fn from_headers(headers: &http::HeaderMap) -> Option<Self> {
        // Implement W3C Trace Context parsing
        // This is a simplified version
        let trace_parent = headers.get("traceparent")?;
        let trace_parent_str = trace_parent.to_str().ok()?;

        let parts: Vec<&str> = trace_parent_str.split('-').collect();
        if parts.len() >= 3 {
            Some(Self {
                trace_id: parts[1].to_string(),
                parent_span_id: parts[2].to_string(),
                baggage: HashMap::new(),
            })
        } else {
            None
        }
    }

    /// Convert to HTTP headers
    pub fn to_headers(&self) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        headers.insert(
            "traceparent".to_string(),
            format!("00-{}-{}-01", self.trace_id, self.parent_span_id)
        );
        headers
    }
}

/// Active span information
#[derive(Debug, Clone)]
struct ActiveSpan {
    id: String,
    name: String,
    parent_id: Option<String>,
    start_time: Instant,
    started_at: DateTime<Utc>,
    attributes: HashMap<String, String>,
    events: Vec<SpanEvent>,
    status: SpanStatus,
}

/// Completed span information
#[derive(Debug, Clone)]
struct CompletedSpan {
    id: String,
    name: String,
    parent_id: Option<String>,
    started_at: DateTime<Utc>,
    completed_at: DateTime<Utc>,
    duration: Duration,
    attributes: HashMap<String, String>,
    events: Vec<SpanEvent>,
    status: SpanStatus,
    error_message: Option<String>,
}

/// Span event
#[derive(Debug, Clone)]
struct SpanEvent {
    name: String,
    timestamp: DateTime<Utc>,
    attributes: HashMap<String, String>,
}

/// Span status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SpanStatus {
    InProgress,
    Success,
    Error,
    Cancelled,
}

/// Tracing statistics
#[derive(Debug, Clone)]
pub struct TracingStats {
    pub spans_created: u64,
    pub spans_completed: u64,
    pub spans_active: u64,
    pub spans_successful: u64,
    pub spans_failed: u64,
    pub spans_cancelled: u64,
    pub events_recorded: u64,
}

impl TracingStats {
    fn new() -> Self {
        Self {
            spans_created: 0,
            spans_completed: 0,
            spans_active: 0,
            spans_successful: 0,
            spans_failed: 0,
            spans_cancelled: 0,
            events_recorded: 0,
        }
    }
}

/// Span information for active spans
#[derive(Debug, Clone)]
pub struct SpanInfo {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub duration: Duration,
    pub attributes: HashMap<String, String>,
}

/// Exported span format (OpenTelemetry compatible)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExportedSpan {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub operation_name: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub duration_ms: u64,
    pub attributes: HashMap<String, String>,
    pub events: Vec<ExportedEvent>,
    pub status: String,
    pub error_message: Option<String>,
}

/// Exported event format
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExportedEvent {
    pub name: String,
    pub timestamp: DateTime<Utc>,
    pub attributes: HashMap<String, String>,
}

/// Tracing configuration extensions
pub struct TracingConfig {
    /// Enable distributed tracing
    pub enabled: bool,
    /// Export endpoint (OTLP)
    pub export_endpoint: Option<String>,
    /// Sampling rate (0.0 to 1.0)
    pub sampling_rate: f64,
    /// Max span retention
    pub max_span_retention: usize,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            export_endpoint: None,
            sampling_rate: 1.0,
            max_span_retention: 10000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConfigLoader;

    #[tokio::test]
    async fn test_tracing_service_creation() {
        let config = ConfigLoader::new().without_env().create_default_config();
        let result = TracingService::new(&config);

        assert!(result.is_ok());
        let service = result.unwrap();

        // Initialize root span
        assert!(service.initialize_root_span().await.is_ok());

        // Check health
        let health = service.health_check().await;
        assert_eq!(health.name, "tracing");
        assert!(matches!(health.status, HealthStatus::Healthy));
    }

    #[tokio::test]
    async fn test_span_lifecycle() {
        let config = ConfigLoader::new().without_env().create_default_config();
        let service = TracingService::new(&config).unwrap();

        // Create a span
        let span = service.create_span("test_operation");
        let span_id = span.span_id().to_string();

        // Add events and attributes
        span.add_event("start_processing", None);
        span.set_attribute("items_count", "100");

        // Create child span
        let child_span = span.child("sub_operation");
        child_span.set_attribute("type", "validation");
        child_span.success();

        // Complete parent span
        span.success();

        // Wait for async operations
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Check statistics
        let stats = service.get_statistics().await;
        assert!(stats.spans_created >= 2);
        assert!(stats.spans_successful >= 2);
        assert!(stats.events_recorded >= 1);
    }

    #[tokio::test]
    async fn test_span_error_handling() {
        let config = ConfigLoader::new().without_env().create_default_config();
        let service = TracingService::new(&config).unwrap();

        // Create span that fails
        let span = service.create_span("failing_operation");
        span.error("Test error occurred");

        // Wait for async operations
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Check statistics
        let stats = service.get_statistics().await;
        assert!(stats.spans_failed >= 1);
    }

    #[tokio::test]
    async fn test_span_auto_complete() {
        let config = ConfigLoader::new().without_env().create_default_config();
        let service = TracingService::new(&config).unwrap();

        {
            let _span = service.create_span("auto_complete_test");
            // Span should auto-complete when dropped
        }

        // Wait for async operations
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Check that span was completed
        let stats = service.get_statistics().await;
        assert!(stats.spans_cancelled >= 1);
    }

    #[tokio::test]
    async fn test_export_spans() {
        let config = ConfigLoader::new().without_env().create_default_config();
        let service = TracingService::new(&config).unwrap();

        // Create and complete some spans
        for i in 0..5 {
            let span = service.create_span(&format!("test_span_{}", i));
            span.set_attribute("index", &i.to_string());
            span.success();
        }

        // Wait for completion
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Export spans
        let exported = service.export_spans().await.unwrap();
        assert!(!exported.is_empty());
        assert!(exported.len() >= 5);
    }

    #[test]
    fn test_trace_context() {
        use http::HeaderMap;
        use http::header::HeaderValue;

        // Test parsing from headers
        let mut headers = HeaderMap::new();
        headers.insert(
            "traceparent",
            HeaderValue::from_static("00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01")
        );

        let context = TraceContext::from_headers(&headers);
        assert!(context.is_some());

        let ctx = context.unwrap();
        assert_eq!(ctx.trace_id, "4bf92f3577b34da6a3ce929d0e0e4736");
        assert_eq!(ctx.parent_span_id, "00f067aa0ba902b7");

        // Test converting to headers
        let headers = ctx.to_headers();
        assert!(headers.contains_key("traceparent"));
    }
}