//! Result type definitions and utilities for the application
//!
//! This module provides convenient result type aliases and utility functions
//! for working with results throughout the Solana Sniper Bot application.

use crate::core::error::AppError;

/// Application result type alias
///
/// This is the primary result type used throughout the application.
/// It wraps the standard `Result<T, E>` with our custom `AppError` type.
///
/// # Examples
///
/// ```rust
/// use solana_sniper_bot::core::result::AppResult;
/// use solana_sniper_bot::core::error::AppError;
///
/// fn example_function() -> AppResult<String> {
///     Ok("Success".to_string())
/// }
///
/// fn failing_function() -> AppResult<()> {
///     Err(AppError::validation("Invalid input"))
/// }
/// ```
pub type AppResult<T> = std::result::Result<T, AppError>;

/// Extension trait for `Result` to provide additional utility methods
pub trait ResultExt<T> {
    /// Map an error to a configuration error with additional context
    fn map_config_err<F>(self, f: F) -> AppResult<T>
    where
        F: FnOnce() -> String;

    /// Map an error to a database error with operation context
    fn map_db_err<F>(self, operation: &str, f: F) -> AppResult<T>
    where
        F: FnOnce() -> String;

    /// Map an error to a network error with endpoint context
    fn map_network_err<F>(self, endpoint: Option<String>, f: F) -> AppResult<T>
    where
        F: FnOnce() -> String;

    /// Map an error to a trading error with trade context
    fn map_trading_err<F>(self, trade_id: Option<String>, f: F) -> AppResult<T>
    where
        F: FnOnce() -> String;

    /// Map an error to a validation error with field context
    fn map_validation_err<F>(self, field: Option<String>, f: F) -> AppResult<T>
    where
        F: FnOnce() -> String;

    /// Add context to any error
    fn with_context<F>(self, f: F) -> AppResult<T>
    where
        F: FnOnce() -> String;
}

impl<T, E> ResultExt<T> for std::result::Result<T, E>
where
    E: std::fmt::Display,
{
    fn map_config_err<F>(self, f: F) -> AppResult<T>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|_| AppError::config(f()))
    }

    fn map_db_err<F>(self, operation: &str, f: F) -> AppResult<T>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|_| AppError::database(f(), operation))
    }

    fn map_network_err<F>(self, endpoint: Option<String>, f: F) -> AppResult<T>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|_| {
            let mut error = AppError::network(f());
            if let AppError::Network { endpoint: ep, .. } = &mut error {
                *ep = endpoint;
            }
            error
        })
    }

    fn map_trading_err<F>(self, trade_id: Option<String>, f: F) -> AppResult<T>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|_| {
            let mut error = AppError::trading(f());
            if let AppError::Trading { trade_id: tid, .. } = &mut error {
                *tid = trade_id;
            }
            error
        })
    }

    fn map_validation_err<F>(self, field: Option<String>, f: F) -> AppResult<T>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|_| {
            let mut error = AppError::validation(f());
            if let AppError::Validation { field: f, .. } = &mut error {
                *f = field;
            }
            error
        })
    }

    fn with_context<F>(self, f: F) -> AppResult<T>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|_| AppError::internal(f()))
    }
}

/// Utility functions for working with results
pub mod utils {
    use super::*;
    use std::future::Future;
    use tokio::time::{timeout, Duration};

    /// Execute a future with a timeout, converting timeout to AppError
    pub async fn with_timeout<F, T>(
        duration: Duration,
        operation: &str,
        future: F,
    ) -> AppResult<T>
    where
        F: Future<Output = AppResult<T>>,
    {
        match timeout(duration, future).await {
            Ok(result) => result,
            Err(_) => Err(AppError::timeout(
                format!("Operation '{}' timed out", operation),
                operation.to_string(),
                duration.as_millis() as u64,
            )),
        }
    }

    /// Retry an operation with exponential backoff
    pub async fn retry_with_backoff<F, Fut, T>(
        mut operation: F,
        max_retries: u32,
        initial_delay: Duration,
        operation_name: &str,
    ) -> AppResult<T>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = AppResult<T>>,
    {
        let mut last_error = None;

        for attempt in 0..=max_retries {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(error) => {
                    if !error.is_retryable() || attempt == max_retries {
                        return Err(error);
                    }

                    let delay = initial_delay * 2_u32.pow(attempt);
                    tracing::warn!(
                        "Operation '{}' failed (attempt {}/{}), retrying in {:?}: {}",
                        operation_name,
                        attempt + 1,
                        max_retries + 1,
                        delay,
                        error
                    );

                    tokio::time::sleep(delay).await;
                    last_error = Some(error);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            AppError::internal(format!("Retry operation '{}' failed", operation_name))
        }))
    }

    /// Collect results, returning the first error encountered
    pub fn collect_results<T>(results: Vec<AppResult<T>>) -> AppResult<Vec<T>> {
        results.into_iter().collect()
    }

    /// Collect results, ignoring errors and returning successful values
    pub fn collect_ok<T>(results: Vec<AppResult<T>>) -> Vec<T> {
        results.into_iter().filter_map(|r| r.ok()).collect()
    }

    /// Collect results, returning successful values and logging errors
    pub fn collect_with_logging<T>(
        results: Vec<AppResult<T>>,
        operation: &str,
    ) -> Vec<T> {
        results
            .into_iter()
            .filter_map(|r| match r {
                Ok(value) => Some(value),
                Err(error) => {
                    tracing::error!(
                        "Error in operation '{}': {}",
                        operation,
                        error
                    );
                    None
                }
            })
            .collect()
    }
}

/// Macros for convenient error handling
#[macro_export]
macro_rules! bail_config {
    ($msg:expr) => {
        return Err($crate::core::error::AppError::config($msg))
    };
    ($fmt:expr, $($arg:tt)*) => {
        return Err($crate::core::error::AppError::config(format!($fmt, $($arg)*)))
    };
}

#[macro_export]
macro_rules! bail_trading {
    ($msg:expr) => {
        return Err($crate::core::error::AppError::trading($msg))
    };
    ($fmt:expr, $($arg:tt)*) => {
        return Err($crate::core::error::AppError::trading(format!($fmt, $($arg)*)))
    };
}

#[macro_export]
macro_rules! bail_validation {
    ($msg:expr) => {
        return Err($crate::core::error::AppError::validation($msg))
    };
    ($fmt:expr, $($arg:tt)*) => {
        return Err($crate::core::error::AppError::validation(format!($fmt, $($arg)*)))
    };
}

#[macro_export]
macro_rules! ensure {
    ($cond:expr, $err:expr) => {
        if !($cond) {
            return Err($err.into());
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::Duration;

    #[test]
    fn test_result_extensions() {
        let result: Result<(), &str> = Err("test error");
        let app_result = result.map_config_err(|| "Configuration failed".to_string());

        assert!(app_result.is_err());
        assert!(matches!(app_result.unwrap_err(), AppError::Config { .. }));
    }

    #[tokio::test]
    async fn test_timeout_utility() {
        let slow_operation = async {
            tokio::time::sleep(Duration::from_millis(100)).await;
            Ok::<(), AppError>(())
        };
        let result = utils::with_timeout(
            Duration::from_millis(50),
            "test_operation",
            slow_operation,
        ).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::Timeout { .. }));
    }

    #[tokio::test]
    async fn test_retry_with_backoff() {
        let mut attempt_count = 0;
        let operation = || {
            attempt_count += 1;
            async move {
                if attempt_count < 3 {
                    Err(AppError::network("Temporary failure"))
                } else {
                    Ok("Success")
                }
            }
        };

        let result = utils::retry_with_backoff(
            operation,
            3,
            Duration::from_millis(1),
            "test_retry",
        ).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Success");
        assert_eq!(attempt_count, 3);
    }

    #[test]
    fn test_collect_results() {
        let results = vec![
            Ok(1),
            Ok(2),
            Err(AppError::validation("test")),
            Ok(3),
        ];

        let collected = utils::collect_results(results.clone());
        assert!(collected.is_err());

        let ok_values = utils::collect_ok(results);
        assert_eq!(ok_values, vec![1, 2, 3]);
    }

    #[test]
    fn test_macros() {
        fn test_bail_config() -> AppResult<()> {
            bail_config!("Test config error");
        }

        fn test_bail_trading() -> AppResult<()> {
            bail_trading!("Test trading error with value: {}", 42);
        }

        assert!(matches!(test_bail_config().unwrap_err(), AppError::Config { .. }));
        assert!(matches!(test_bail_trading().unwrap_err(), AppError::Trading { .. }));
    }

    #[test]
    fn test_ensure_macro() {
        fn test_ensure(value: i32) -> AppResult<()> {
            ensure!(value > 0, AppError::validation("Value must be positive"));
            Ok(())
        }

        assert!(test_ensure(1).is_ok());
        assert!(test_ensure(-1).is_err());
    }
}