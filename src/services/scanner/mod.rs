//! Token scanning and detection service module
//!
//! This module provides real-time token discovery, metadata extraction,
//! and intelligent filtering for the Solana blockchain.

pub mod detector;
pub mod event_listener;
pub mod filters;
pub mod token_parser;

use std::sync::Arc;
use tokio::sync::{broadcast, RwLock, Mutex};
use tracing::{info, warn, error, debug, instrument};
use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::config::models::ScannerConfig;
use crate::core::result::AppResult;
use crate::core::error::AppError;
use crate::core::types::{TokenAddress, Timestamp};
use crate::infrastructure::database::DatabaseService;
use crate::services::solana::SolanaService;

pub use detector::{TokenDetector, DetectedToken};
pub use event_listener::{EventListener, TokenEvent, EventType};
pub use filters::{TokenFilter, FilterCriteria, FilterResult};
pub use token_parser::{TokenParser, TokenMetadata, ParsedToken};

/// Scanner service coordinator
#[derive(Debug)]
pub struct ScannerService {
    /// Configuration
    config: Arc<ScannerConfig>,

    /// Token detector
    detector: Arc<TokenDetector>,

    /// Event listener
    event_listener: Arc<EventListener>,

    /// Token parser
    parser: Arc<TokenParser>,

    /// Token filter
    filter: Arc<TokenFilter>,

    /// Database service
    database: Arc<DatabaseService>,

    /// Solana service
    solana: Arc<SolanaService>,

    /// Broadcast channel for new tokens
    token_broadcaster: broadcast::Sender<DetectedToken>,

    /// Scanner state
    state: Arc<RwLock<ScannerState>>,

    /// Performance metrics
    metrics: Arc<Mutex<ScannerMetrics>>,
}

/// Scanner state
#[derive(Debug, Clone)]
pub struct ScannerState {
    /// Is scanner running
    pub is_running: bool,

    /// Last scan timestamp
    pub last_scan: Option<Timestamp>,

    /// Total tokens detected
    pub total_detected: u64,

    /// Tokens passed filters
    pub total_passed: u64,

    /// Current scan interval
    pub scan_interval_ms: u64,
}

/// Scanner performance metrics
#[derive(Debug, Clone)]
pub struct ScannerMetrics {
    /// Total scans performed
    pub total_scans: u64,

    /// Average scan duration
    pub avg_scan_duration_ms: u64,

    /// Fastest scan
    pub min_scan_duration_ms: u64,

    /// Slowest scan
    pub max_scan_duration_ms: u64,

    /// Detection latency samples
    pub detection_latencies: Vec<u64>,

    /// Filter pass rate
    pub filter_pass_rate: f64,
}

impl ScannerService {
    /// Create a new scanner service
    #[instrument(skip_all)]
    pub async fn new(
        config: Arc<ScannerConfig>,
        database: Arc<DatabaseService>,
        solana: Arc<SolanaService>,
    ) -> AppResult<Self> {
        info!("🔍 Initializing token scanner service");

        // Create broadcast channel for new tokens
        let (token_broadcaster, _) = broadcast::channel(1000);

        // Initialize components
        let detector = Arc::new(TokenDetector::new(config.clone(), solana.clone()).await?);
        let event_listener = Arc::new(EventListener::new(config.clone(), solana.clone()).await?);
        let parser = Arc::new(TokenParser::new(solana.clone()).await?);
        let filter = Arc::new(TokenFilter::new(config.clone(), database.clone()).await?);

        // Initialize state
        let state = Arc::new(RwLock::new(ScannerState {
            is_running: false,
            last_scan: None,
            total_detected: 0,
            total_passed: 0,
            scan_interval_ms: config.scan_interval_ms,
        }));

        // Initialize metrics
        let metrics = Arc::new(Mutex::new(ScannerMetrics {
            total_scans: 0,
            avg_scan_duration_ms: 0,
            min_scan_duration_ms: u64::MAX,
            max_scan_duration_ms: 0,
            detection_latencies: Vec::new(),
            filter_pass_rate: 0.0,
        }));

        info!("✅ Token scanner service initialized");

        Ok(Self {
            config,
            detector,
            event_listener,
            parser,
            filter,
            database,
            solana,
            token_broadcaster,
            state,
            metrics,
        })
    }

    /// Start the scanner service
    #[instrument(skip(self))]
    pub async fn start(&self) -> AppResult<()> {
        info!("🚀 Starting token scanner service");

        // Update state
        {
            let mut state = self.state.write().await;
            if state.is_running {
                return Err(AppError::internal("Scanner already running"));
            }
            state.is_running = true;
        }

        // Start event listener
        self.start_event_listener().await?;

        // Start periodic scanner
        if self.config.enable_real_time_scanning {
            self.start_periodic_scanner().await?;
        }

        info!("✅ Token scanner service started");
        Ok(())
    }

    /// Stop the scanner service
    #[instrument(skip(self))]
    pub async fn stop(&self) -> AppResult<()> {
        info!("🛑 Stopping token scanner service");

        // Update state
        {
            let mut state = self.state.write().await;
            state.is_running = false;
        }

        // Stop components
        self.event_listener.stop().await?;

        info!("✅ Token scanner service stopped");
        Ok(())
    }

    /// Start event listener task
    async fn start_event_listener(&self) -> AppResult<()> {
        let event_listener = self.event_listener.clone();
        let parser = self.parser.clone();
        let filter = self.filter.clone();
        let broadcaster = self.token_broadcaster.clone();
        let state = self.state.clone();
        let metrics = self.metrics.clone();

        // Subscribe to events
        let mut event_receiver = event_listener.subscribe().await?;

        tokio::spawn(async move {
            info!("📡 Event listener task started");

            while let Ok(event) = event_receiver.recv().await {
                let start = Instant::now();

                match Self::process_token_event(
                    event,
                    &parser,
                    &filter,
                    &broadcaster,
                    &state,
                ).await {
                    Ok(passed) => {
                        let latency = start.elapsed().as_millis() as u64;

                        // Update metrics
                        let mut metrics = metrics.lock().await;
                        metrics.detection_latencies.push(latency);
                        if metrics.detection_latencies.len() > 1000 {
                            metrics.detection_latencies.remove(0);
                        }

                        if passed {
                            debug!("✅ Token passed filters ({}ms latency)", latency);
                        }
                    }
                    Err(e) => {
                        error!("Failed to process token event: {}", e);
                    }
                }
            }

            warn!("Event listener task ended");
        });

        // Start the listener
        event_listener.start().await?;

        Ok(())
    }

    /// Start periodic scanner task
    async fn start_periodic_scanner(&self) -> AppResult<()> {
        let detector = self.detector.clone();
        let parser = self.parser.clone();
        let filter = self.filter.clone();
        let broadcaster = self.token_broadcaster.clone();
        let state = self.state.clone();
        let metrics = self.metrics.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            info!("🔄 Periodic scanner task started");

            let mut interval = tokio::time::interval(Duration::from_millis(config.scan_interval_ms));

            loop {
                interval.tick().await;

                // Check if still running
                {
                    let state = state.read().await;
                    if !state.is_running {
                        break;
                    }
                }

                let start = Instant::now();

                // Perform scan
                match Self::perform_scan(
                    &detector,
                    &parser,
                    &filter,
                    &broadcaster,
                    &state,
                    &config,
                ).await {
                    Ok(tokens_found) => {
                        let duration = start.elapsed().as_millis() as u64;

                        // Update metrics
                        let mut metrics = metrics.lock().await;
                        metrics.total_scans += 1;
                        metrics.min_scan_duration_ms = metrics.min_scan_duration_ms.min(duration);
                        metrics.max_scan_duration_ms = metrics.max_scan_duration_ms.max(duration);

                        // Update average
                        let total_duration = metrics.avg_scan_duration_ms * (metrics.total_scans - 1) + duration;
                        metrics.avg_scan_duration_ms = total_duration / metrics.total_scans;

                        debug!("🔍 Scan completed: {} tokens found in {}ms", tokens_found, duration);
                    }
                    Err(e) => {
                        error!("Scan failed: {}", e);
                    }
                }
            }

            warn!("Periodic scanner task ended");
        });

        Ok(())
    }

    /// Process a token event
    async fn process_token_event(
        event: TokenEvent,
        parser: &Arc<TokenParser>,
        filter: &Arc<TokenFilter>,
        broadcaster: &broadcast::Sender<DetectedToken>,
        state: &Arc<RwLock<ScannerState>>,
    ) -> AppResult<bool> {
        debug!("Processing token event: {:?}", event.event_type);

        // Parse token metadata
        let parsed_token = parser.parse_token(&event.token_address).await?;

        // Apply filters
        let filter_result = filter.apply_filters(&parsed_token).await?;

        // Update state
        {
            let mut state = state.write().await;
            state.total_detected += 1;

            if filter_result.passed {
                state.total_passed += 1;
            }
        }

        if filter_result.passed {
            // Create detected token
            let detected_token = DetectedToken {
                address: event.token_address,
                metadata: parsed_token.metadata,
                filter_result,
                detected_at: Timestamp::now(),
                event_source: event.source,
                detection_latency_ms: event.latency_ms,
            };

            // Broadcast to subscribers
            let _ = broadcaster.send(detected_token);

            Ok(true)
        } else {
            debug!("Token filtered out: {:?}", filter_result.rejection_reasons);
            Ok(false)
        }
    }

    /// Perform a periodic scan
    async fn perform_scan(
        detector: &Arc<TokenDetector>,
        parser: &Arc<TokenParser>,
        filter: &Arc<TokenFilter>,
        broadcaster: &broadcast::Sender<DetectedToken>,
        state: &Arc<RwLock<ScannerState>>,
        config: &ScannerConfig,
    ) -> AppResult<u32> {
        // Detect new tokens
        let new_tokens = detector.detect_new_tokens(config.max_tokens_per_scan).await?;
        let mut passed_count = 0;

        for token_address in new_tokens {
            // Parse metadata
            match parser.parse_token(&token_address).await {
                Ok(parsed_token) => {
                    // Apply filters
                    match filter.apply_filters(&parsed_token).await {
                        Ok(filter_result) => {
                            if filter_result.passed {
                                passed_count += 1;

                                // Create detected token
                                let detected_token = DetectedToken {
                                    address: token_address,
                                    metadata: parsed_token.metadata,
                                    filter_result,
                                    detected_at: Timestamp::now(),
                                    event_source: "periodic_scan".to_string(),
                                    detection_latency_ms: 0,
                                };

                                // Broadcast
                                let _ = broadcaster.send(detected_token);
                            }
                        }
                        Err(e) => {
                            warn!("Filter error for token {}: {}", token_address, e);
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to parse token {}: {}", token_address, e);
                }
            }
        }

        // Update state
        {
            let mut state = state.write().await;
            state.last_scan = Some(Timestamp::now());
            state.total_detected += new_tokens.len() as u64;
            state.total_passed += passed_count as u64;
        }

        Ok(passed_count)
    }

    /// Subscribe to new token events
    pub fn subscribe(&self) -> broadcast::Receiver<DetectedToken> {
        self.token_broadcaster.subscribe()
    }

    /// Get scanner state
    pub async fn get_state(&self) -> ScannerState {
        self.state.read().await.clone()
    }

    /// Get scanner metrics
    pub async fn get_metrics(&self) -> ScannerMetrics {
        let metrics = self.metrics.lock().await;
        let mut result = metrics.clone();

        // Calculate filter pass rate
        let state = self.state.read().await;
        if state.total_detected > 0 {
            result.filter_pass_rate = state.total_passed as f64 / state.total_detected as f64;
        }

        result
    }

    /// Update scan interval
    pub async fn update_scan_interval(&self, interval_ms: u64) -> AppResult<()> {
        if interval_ms < 100 {
            return Err(AppError::validation("Scan interval must be at least 100ms"));
        }

        let mut state = self.state.write().await;
        state.scan_interval_ms = interval_ms;

        info!("Updated scan interval to {}ms", interval_ms);
        Ok(())
    }

    /// Manually trigger a scan
    #[instrument(skip(self))]
    pub async fn trigger_scan(&self) -> AppResult<u32> {
        info!("🔍 Manually triggering token scan");

        let start = Instant::now();

        let tokens_found = Self::perform_scan(
            &self.detector,
            &self.parser,
            &self.filter,
            &self.token_broadcaster,
            &self.state,
            &self.config,
        ).await?;

        let duration = start.elapsed();
        info!("✅ Manual scan completed: {} tokens found in {:?}", tokens_found, duration);

        Ok(tokens_found)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConfigLoader;

    #[tokio::test]
    async fn test_scanner_service_creation() {
        let config = ConfigLoader::new().without_env().create_default_config();
        let solana_service = Arc::new(SolanaService::new(&config.solana, &config.helius).await.unwrap());
        let database_service = Arc::new(DatabaseService::new(&config).await.unwrap());

        let scanner = ScannerService::new(
            Arc::new(config.scanner),
            database_service,
            solana_service,
        ).await;

        assert!(scanner.is_ok());
    }
}