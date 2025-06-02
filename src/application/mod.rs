//! Application layer module
//!
//! This module contains the main application structure and coordination logic
//! for the Solana Sniper Bot. It handles application lifecycle, dependency injection,
//! and service orchestration.

pub mod app;
pub mod health;

// Re-export main application type
pub use app::Application;
pub use health::{HealthService, HealthStatus, ComponentHealth};

/// Application result type alias
pub type AppResult<T> = crate::core::result::AppResult<T>;