//! Real-time blockchain event monitoring
//!
//! This module provides WebSocket and webhook-based event listening
//! for real-time token creation and liquidity pool events.

use std::sync::Arc;
use tokio::sync::{broadcast, RwLock, Mutex};
use tracing::{info, warn, error, debug, instrument};
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use futures_util::StreamExt;

use crate::config::models::ScannerConfig;
use crate::core::result::AppResult;
use crate::core::error::AppError;
use crate::core::types::{TokenAddress, Timestamp};
use crate::services::solana::{SolanaService, HeliusWebsocket};

/// Event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    /// New token mint created
    TokenMint,
    /// New liquidity pool created
    LiquidityPool,
    /// Token metadata updated
    MetadataUpdate,
    /// Large liquidity addition
    LiquidityAdded,
    /// Token account created
    TokenAccount,
}

/// Token event data
#[derive(Debug, Clone)]
pub struct TokenEvent {
    /// Event type
    pub event_type: EventType,

    /// Token address
    pub token_address: TokenAddress,

    /// Event source
    pub source: String,

    /// Event timestamp
    pub timestamp: Timestamp,

    /// Detection latency in milliseconds
    pub latency_ms: u64,

    /// Additional event data
    pub data: serde_json::Value,
}

/// Event listener state
#[derive(Debug, Clone)]
struct ListenerState {
    /// Is listener running
    is_running: bool,

    /// Total events received
    total_events: u64,

    /// Events by type
    events_by_type: std::collections::HashMap<String, u64>,

    /// Last event timestamp
    last_event: Option<Timestamp>,

    /// Connection status
    connection_status: ConnectionStatus,
}

/// Connection status
#[derive(Debug, Clone)]
enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
}

/// Event listener for real-time token detection
#[derive(Debug)]
pub struct EventListener {
    /// Configuration
    config: Arc<ScannerConfig>,

    /// Solana service
    solana: Arc<SolanaService>,

    /// Event broadcaster
    event_sender: broadcast::Sender<TokenEvent>,

    /// Listener state
    state: Arc<RwLock<ListenerState>>,

    /// Reconnection attempts
    reconnect_attempts: Arc<Mutex<u32>>,

    /// WebSocket connections
    websockets: Arc<RwLock<Vec<HeliusWebsocket>>>,
}

impl EventListener {
    /// Create a new event listener
    #[instrument(skip_all)]
    pub async fn new(
        config: Arc<ScannerConfig>,
        solana: Arc<SolanaService>,
    ) -> AppResult<Self> {
        info!("📡 Initializing event listener");

        let (event_sender, _) = broadcast::channel(10000);

        let state = Arc::new(RwLock::new(ListenerState {
            is_running: false,
            total_events: 0,
            events_by_type: std::collections::HashMap::new(),
            last_event: None,
            connection_status: ConnectionStatus::Disconnected,
        }));

        Ok(Self {
            config,
            solana,
            event_sender,
            state,
            reconnect_attempts: Arc::new(Mutex::new(0)),
            websockets: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Start listening for events
    #[instrument(skip(self))]
    pub async fn start(&self) -> AppResult<()> {
        info!("🚀 Starting event listener");

        // Update state
        {
            let mut state = self.state.write().await;
            if state.is_running {
                return Err(AppError::internal("Event listener already running"));
            }
            state.is_running = true;
            state.connection_status = ConnectionStatus::Connecting;
        }

        // Start WebSocket connections
        if self.config.enable_real_time_scanning {
            self.start_websocket_listeners().await?;
        }

        // Start webhook server if configured
        if let Some(ref webhook_url) = self.solana.helius_webhook_url() {
            self.start_webhook_server(webhook_url).await?;
        }

        info!("✅ Event listener started");
        Ok(())
    }

    /// Stop listening for events
    #[instrument(skip(self))]
    pub async fn stop(&self) -> AppResult<()> {
        info!("🛑 Stopping event listener");

        // Update state
        {
            let mut state = self.state.write().await;
            state.is_running = false;
            state.connection_status = ConnectionStatus::Disconnected;
        }

        // Close WebSocket connections
        {
            let mut websockets = self.websockets.write().await;
            for ws in websockets.drain(..) {
                ws.close().await;
            }
        }

        info!("✅ Event listener stopped");
        Ok(())
    }

    /// Start WebSocket listeners
    async fn start_websocket_listeners(&self) -> AppResult<()> {
        // Connect to Helius WebSocket for program events
        let ws_url = self.solana.helius_websocket_url()?;

        // Programs to monitor
        let programs_to_monitor = vec![
            "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA", // Token Program
            "11111111111111111111111111111112", // System Program (for new accounts)
            "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8", // Raydium AMM
            "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc", // Orca Whirlpool
            "Eo7WjKq67rjJQSZxS6z3YkapzY3eMj6Xy8X5EQVn5UaB", // Meteora
        ];

        for program_id in programs_to_monitor {
            self.connect_program_websocket(program_id).await?;
        }

        // Update connection status
        {
            let mut state = self.state.write().await;
            state.connection_status = ConnectionStatus::Connected;
        }

        Ok(())
    }

    /// Connect to program-specific WebSocket
    async fn connect_program_websocket(&self, program_id: &str) -> AppResult<()> {
        let solana = self.solana.clone();
        let event_sender = self.event_sender.clone();
        let state = self.state.clone();
        let reconnect_attempts = self.reconnect_attempts.clone();
        let program_id = program_id.to_string();

        tokio::spawn(async move {
            loop {
                match Self::handle_program_websocket(
                    &solana,
                    &program_id,
                    &event_sender,
                    &state,
                ).await {
                    Ok(_) => {
                        warn!("WebSocket connection closed for program {}", program_id);
                    }
                    Err(e) => {
                        error!("WebSocket error for program {}: {}", program_id, e);
                    }
                }

                // Check if we should reconnect
                {
                    let state = state.read().await;
                    if !state.is_running {
                        break;
                    }
                }

                // Exponential backoff for reconnection
                let attempts = {
                    let mut attempts = reconnect_attempts.lock().await;
                    *attempts += 1;
                    *attempts
                };

                let delay = Duration::from_secs((2_u64).pow(attempts.min(6)));
                warn!("Reconnecting to {} in {:?} (attempt {})", program_id, delay, attempts);
                tokio::time::sleep(delay).await;
            }
        });

        Ok(())
    }

    /// Handle program WebSocket connection
    async fn handle_program_websocket(
        solana: &SolanaService,
        program_id: &str,
        event_sender: &broadcast::Sender<TokenEvent>,
        state: &Arc<RwLock<ListenerState>>,
    ) -> AppResult<()> {
        debug!("Connecting to WebSocket for program {}", program_id);

        let mut ws_stream = solana.subscribe_program_events(program_id).await?;

        while let Some(msg) = ws_stream.next().await {
            match msg {
                Ok(event_data) => {
                    let event_time = Instant::now();

                    // Parse event
                    match Self::parse_program_event(program_id, event_data) {
                        Ok(Some(token_event)) => {
                            let latency = event_time.elapsed().as_millis() as u64;

                            // Update state
                            {
                                let mut state = state.write().await;
                                state.total_events += 1;
                                state.last_event = Some(Timestamp::now());

                                let event_type_str = format!("{:?}", token_event.event_type);
                                *state.events_by_type.entry(event_type_str).or_insert(0) += 1;
                            }

                            // Create event with latency
                            let mut event = token_event;
                            event.latency_ms = latency;

                            // Broadcast event
                            let _ = event_sender.send(event);
                        }
                        Ok(None) => {
                            // Event not relevant
                        }
                        Err(e) => {
                            debug!("Failed to parse event: {}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("WebSocket message error: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Parse program event into token event
    fn parse_program_event(
        program_id: &str,
        event_data: serde_json::Value,
    ) -> AppResult<Option<TokenEvent>> {
        // Extract instruction data
        let instruction = event_data.get("instruction")
            .ok_or_else(|| AppError::internal("Missing instruction in event"))?;

        match program_id {
            // Token Program
            "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" => {
                Self::parse_token_program_event(instruction)
            }
            // Raydium AMM
            "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8" => {
                Self::parse_raydium_event(instruction)
            }
            // Other DEX programs
            _ => {
                Self::parse_generic_dex_event(program_id, instruction)
            }
        }
    }

    /// Parse Token Program event
    fn parse_token_program_event(instruction: &serde_json::Value) -> AppResult<Option<TokenEvent>> {
        let instruction_type = instruction.get("type")
            .and_then(|t| t.as_str())
            .ok_or_else(|| AppError::internal("Missing instruction type"))?;

        match instruction_type {
            "initializeMint" | "initializeMint2" => {
                let mint_address = instruction.get("mint")
                    .and_then(|m| m.as_str())
                    .ok_or_else(|| AppError::internal("Missing mint address"))?;

                Ok(Some(TokenEvent {
                    event_type: EventType::TokenMint,
                    token_address: TokenAddress::new_unchecked(mint_address.to_string()),
                    source: "token_program".to_string(),
                    timestamp: Timestamp::now(),
                    latency_ms: 0,
                    data: instruction.clone(),
                }))
            }
            _ => Ok(None),
        }
    }

    /// Parse Raydium event
    fn parse_raydium_event(instruction: &serde_json::Value) -> AppResult<Option<TokenEvent>> {
        let instruction_type = instruction.get("type")
            .and_then(|t| t.as_str())
            .ok_or_else(|| AppError::internal("Missing instruction type"))?;

        match instruction_type {
            "initialize" | "initialize2" => {
                // New liquidity pool created
                let token_a = instruction.get("tokenA")
                    .and_then(|t| t.as_str())
                    .ok_or_else(|| AppError::internal("Missing token A"))?;

                let token_b = instruction.get("tokenB")
                    .and_then(|t| t.as_str())
                    .ok_or_else(|| AppError::internal("Missing token B"))?;

                // Check if one of the tokens is SOL
                let sol_mint = "So11111111111111111111111111111111111111112";
                let new_token = if token_a == sol_mint {
                    token_b
                } else if token_b == sol_mint {
                    token_a
                } else {
                    return Ok(None); // Not a SOL pair
                };

                Ok(Some(TokenEvent {
                    event_type: EventType::LiquidityPool,
                    token_address: TokenAddress::new_unchecked(new_token.to_string()),
                    source: "raydium".to_string(),
                    timestamp: Timestamp::now(),
                    latency_ms: 0,
                    data: instruction.clone(),
                }))
            }
            "deposit" | "depositAllTokenTypes" => {
                // Liquidity added
                let amount = instruction.get("amount")
                    .and_then(|a| a.as_u64())
                    .unwrap_or(0);

                // Only interested in large liquidity additions
                if amount > 1_000_000_000 { // 1 SOL worth
                    let pool_id = instruction.get("poolId")
                        .and_then(|p| p.as_str())
                        .ok_or_else(|| AppError::internal("Missing pool ID"))?;

                    // Would need to look up token from pool ID
                    // For now, we'll skip this
                    return Ok(None);
                }

                Ok(None)
            }
            _ => Ok(None),
        }
    }

    /// Parse generic DEX event
    fn parse_generic_dex_event(
        program_id: &str,
        instruction: &serde_json::Value,
    ) -> AppResult<Option<TokenEvent>> {
        // Generic parsing for other DEX programs
        // This would be expanded based on specific DEX requirements

        debug!("Generic DEX event from {}: {:?}", program_id, instruction);
        Ok(None)
    }

    /// Start webhook server
    async fn start_webhook_server(&self, webhook_url: &str) -> AppResult<()> {
        info!("🌐 Starting webhook server at {}", webhook_url);

        // This would implement an HTTP server to receive webhooks
        // For now, we'll use a placeholder

        let event_sender = self.event_sender.clone();
        let state = self.state.clone();

        tokio::spawn(async move {
            // Webhook server implementation would go here
            // Using axum or warp to create HTTP endpoints

            warn!("Webhook server not yet implemented");
        });

        Ok(())
    }

    /// Subscribe to token events
    pub async fn subscribe(&self) -> AppResult<broadcast::Receiver<TokenEvent>> {
        Ok(self.event_sender.subscribe())
    }

    /// Get listener statistics
    pub async fn get_stats(&self) -> ListenerStats {
        let state = self.state.read().await;

        ListenerStats {
            is_running: state.is_running,
            total_events: state.total_events,
            events_by_type: state.events_by_type.clone(),
            last_event: state.last_event,
            connection_status: format!("{:?}", state.connection_status),
        }
    }
}

/// Listener statistics
#[derive(Debug, Clone, Serialize)]
pub struct ListenerStats {
    pub is_running: bool,
    pub total_events: u64,
    pub events_by_type: std::collections::HashMap<String, u64>,
    pub last_event: Option<Timestamp>,
    pub connection_status: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_serialization() {
        let event_type = EventType::TokenMint;
        let serialized = serde_json::to_string(&event_type).unwrap();
        assert!(serialized.contains("TokenMint"));
    }
}