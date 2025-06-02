//! Time utilities and helpers for consistent time handling across the application
//!
//! This module provides utilities for time formatting, duration calculations,
//! timezone handling, and performance timing measurements.

use chrono::{DateTime, Duration, TimeZone, Utc};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use anyhow::{anyhow, Result};

/// Standard time format used throughout the application
pub const STANDARD_TIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S UTC";

/// ISO 8601 time format
pub const ISO_TIME_FORMAT: &str = "%Y-%m-%dT%H:%M:%S%.3fZ";

/// Simple date format
pub const DATE_FORMAT: &str = "%Y-%m-%d";

/// Time format for logs
pub const LOG_TIME_FORMAT: &str = "%H:%M:%S%.3f";

/// Get current UTC timestamp
pub fn now() -> DateTime<Utc> {
    Utc::now()
}

/// Get current Unix timestamp in seconds
pub fn unix_timestamp() -> i64 {
    now().timestamp()
}

/// Get current Unix timestamp in milliseconds
pub fn unix_timestamp_millis() -> i64 {
    now().timestamp_millis()
}

/// Get current Unix timestamp in microseconds
pub fn unix_timestamp_micros() -> i64 {
    now().timestamp_micros()
}

/// Convert Unix timestamp to DateTime<Utc>
pub fn from_unix_timestamp(timestamp: i64) -> Result<DateTime<Utc>> {
    Utc.timestamp_opt(timestamp, 0)
        .single()
        .ok_or_else(|| anyhow!("Invalid timestamp: {}", timestamp))
}

/// Convert Unix timestamp in milliseconds to DateTime<Utc>
pub fn from_unix_timestamp_millis(timestamp_millis: i64) -> Result<DateTime<Utc>> {
    let secs = timestamp_millis / 1000;
    let nsecs = ((timestamp_millis % 1000) * 1_000_000) as u32;

    Utc.timestamp_opt(secs, nsecs)
        .single()
        .ok_or_else(|| anyhow!("Invalid timestamp: {}", timestamp_millis))
}

/// Format a DateTime as a standard string
pub fn format_datetime(dt: &DateTime<Utc>) -> String {
    dt.format(STANDARD_TIME_FORMAT).to_string()
}

/// Format a DateTime as ISO 8601 string
pub fn format_iso(dt: &DateTime<Utc>) -> String {
    dt.format(ISO_TIME_FORMAT).to_string()
}

/// Format a DateTime for logging
pub fn format_log_time(dt: &DateTime<Utc>) -> String {
    dt.format(LOG_TIME_FORMAT).to_string()
}

/// Parse a datetime string in standard format
pub fn parse_datetime(s: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_str(s, STANDARD_TIME_FORMAT)
        .map(|dt| dt.with_timezone(&Utc))
        .or_else(|_| {
            // Try ISO format as fallback
            DateTime::parse_from_str(s, ISO_TIME_FORMAT)
                .map(|dt| dt.with_timezone(&Utc))
        })
        .map_err(|e| anyhow!("Failed to parse datetime '{}': {}", s, e))
}

/// Calculate duration between two timestamps
pub fn duration_between(start: &DateTime<Utc>, end: &DateTime<Utc>) -> Duration {
    *end - *start
}

/// Format a duration as human-readable string
pub fn format_duration(duration: Duration) -> String {
    let total_seconds = duration.num_seconds().abs();

    if total_seconds < 60 {
        format!("{}s", total_seconds)
    } else if total_seconds < 3600 {
        let minutes = total_seconds / 60;
        let seconds = total_seconds % 60;
        format!("{}m {}s", minutes, seconds)
    } else if total_seconds < 86400 {
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        format!("{}h {}m", hours, minutes)
    } else {
        let days = total_seconds / 86400;
        let hours = (total_seconds % 86400) / 3600;
        format!("{}d {}h", days, hours)
    }
}

/// Format duration in milliseconds
pub fn format_duration_ms(duration: Duration) -> String {
    let millis = duration.num_milliseconds();
    format!("{}ms", millis)
}

/// Format duration in microseconds
pub fn format_duration_us(duration: Duration) -> String {
    let micros = duration.num_microseconds().unwrap_or(0);
    format!("{}μs", micros)
}

/// Get time since a specific datetime
pub fn time_since(start: &DateTime<Utc>) -> Duration {
    now() - *start
}

/// Get time until a specific datetime
pub fn time_until(end: &DateTime<Utc>) -> Duration {
    *end - now()
}

/// Check if a datetime is in the past
pub fn is_past(dt: &DateTime<Utc>) -> bool {
    *dt < now()
}

/// Check if a datetime is in the future
pub fn is_future(dt: &DateTime<Utc>) -> bool {
    *dt > now()
}

/// Add duration to a datetime
pub fn add_duration(dt: &DateTime<Utc>, duration: Duration) -> DateTime<Utc> {
    *dt + duration
}

/// Subtract duration from a datetime
pub fn subtract_duration(dt: &DateTime<Utc>, duration: Duration) -> DateTime<Utc> {
    *dt - duration
}

/// Get the start of the current day
pub fn start_of_day(dt: &DateTime<Utc>) -> DateTime<Utc> {
    dt.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc()
}

/// Get the end of the current day
pub fn end_of_day(dt: &DateTime<Utc>) -> DateTime<Utc> {
    dt.date_naive().and_hms_opt(23, 59, 59).unwrap().and_utc()
}

/// Get the start of the current hour
pub fn start_of_hour(dt: &DateTime<Utc>) -> DateTime<Utc> {
    dt.date_naive()
        .and_hms_opt(dt.hour(), 0, 0)
        .unwrap()
        .and_utc()
}

/// Performance timing utilities
pub mod perf {
    use super::*;
    use std::time::Instant;

    /// High-precision timer for performance measurements
    #[derive(Debug, Clone)]
    pub struct Timer {
        start: Instant,
        name: String,
    }

    impl Timer {
        /// Start a new timer with a name
        pub fn start(name: impl Into<String>) -> Self {
            Self {
                start: Instant::now(),
                name: name.into(),
            }
        }

        /// Get elapsed time since timer start
        pub fn elapsed(&self) -> std::time::Duration {
            self.start.elapsed()
        }

        /// Get elapsed time in milliseconds
        pub fn elapsed_millis(&self) -> u128 {
            self.elapsed().as_millis()
        }

        /// Get elapsed time in microseconds
        pub fn elapsed_micros(&self) -> u128 {
            self.elapsed().as_micros()
        }

        /// Get elapsed time in nanoseconds
        pub fn elapsed_nanos(&self) -> u128 {
            self.elapsed().as_nanos()
        }

        /// Stop the timer and log the elapsed time
        pub fn stop_and_log(self) {
            let elapsed = self.elapsed();
            tracing::debug!(
                "Timer '{}' completed in {:.3}ms",
                self.name,
                elapsed.as_secs_f64() * 1000.0
            );
        }

        /// Stop the timer and return the elapsed duration
        pub fn stop(self) -> std::time::Duration {
            self.elapsed()
        }
    }

    /// Time a closure and return the result with elapsed time
    pub fn time_it<F, R>(name: &str, f: F) -> (R, std::time::Duration)
    where
        F: FnOnce() -> R,
    {
        let timer = Timer::start(name);
        let result = f();
        let elapsed = timer.stop();
        (result, elapsed)
    }

    /// Time an async closure and return the result with elapsed time
    pub async fn time_it_async<F, Fut, R>(name: &str, f: F) -> (R, std::time::Duration)
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = R>,
    {
        let timer = Timer::start(name);
        let result = f().await;
        let elapsed = timer.stop();
        (result, elapsed)
    }
}

/// Sleep utilities
pub mod sleep {
    use tokio::time::{sleep, Duration as TokioDuration};

    /// Sleep for a specified number of milliseconds
    pub async fn sleep_millis(millis: u64) {
        sleep(TokioDuration::from_millis(millis)).await;
    }

    /// Sleep for a specified number of seconds
    pub async fn sleep_secs(secs: u64) {
        sleep(TokioDuration::from_secs(secs)).await;
    }

    /// Sleep with exponential backoff
    pub async fn sleep_with_backoff(attempt: u32, base_millis: u64, max_millis: u64) {
        let delay_millis = (base_millis * 2_u64.pow(attempt)).min(max_millis);
        sleep_millis(delay_millis).await;
    }
}

/// Rate limiting utilities
pub mod rate_limit {
    use super::*;
    use std::collections::HashMap;
    use tokio::sync::Mutex;
    use std::sync::Arc;

    /// Simple rate limiter based on token bucket algorithm
    #[derive(Debug)]
    pub struct RateLimiter {
        buckets: Arc<Mutex<HashMap<String, TokenBucket>>>,
        capacity: u32,
        refill_rate: u32,
        window_seconds: u64,
    }

    #[derive(Debug)]
    struct TokenBucket {
        tokens: u32,
        last_refill: DateTime<Utc>,
    }

    impl RateLimiter {
        /// Create a new rate limiter
        pub fn new(capacity: u32, refill_rate: u32, window_seconds: u64) -> Self {
            Self {
                buckets: Arc::new(Mutex::new(HashMap::new())),
                capacity,
                refill_rate,
                window_seconds,
            }
        }

        /// Check if a request is allowed for the given key
        pub async fn is_allowed(&self, key: &str) -> bool {
            let mut buckets = self.buckets.lock().await;
            let now = now();

            let bucket = buckets.entry(key.to_string()).or_insert(TokenBucket {
                tokens: self.capacity,
                last_refill: now,
            });

            // Refill tokens based on elapsed time
            let elapsed = duration_between(&bucket.last_refill, &now);
            let elapsed_seconds = elapsed.num_seconds() as u64;

            if elapsed_seconds > 0 {
                let tokens_to_add = (elapsed_seconds * self.refill_rate as u64) as u32;
                bucket.tokens = (bucket.tokens + tokens_to_add).min(self.capacity);
                bucket.last_refill = now;
            }

            // Check if we have tokens available
            if bucket.tokens > 0 {
                bucket.tokens -= 1;
                true
            } else {
                false
            }
        }

        /// Get the number of seconds until the next token is available
        pub async fn retry_after(&self, key: &str) -> u64 {
            let buckets = self.buckets.lock().await;
            if let Some(bucket) = buckets.get(key) {
                if bucket.tokens == 0 {
                    return self.window_seconds / self.refill_rate as u64;
                }
            }
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration as TokioDuration};

    #[test]
    fn test_timestamp_functions() {
        let now_ts = unix_timestamp();
        let now_ms = unix_timestamp_millis();

        assert!(now_ts > 0);
        assert!(now_ms > now_ts * 1000);

        let dt = from_unix_timestamp(now_ts).unwrap();
        assert_eq!(dt.timestamp(), now_ts);
    }

    #[test]
    fn test_datetime_formatting() {
        let dt = Utc::now();
        let formatted = format_datetime(&dt);
        let iso_formatted = format_iso(&dt);

        assert!(!formatted.is_empty());
        assert!(!iso_formatted.is_empty());
        assert!(formatted.contains("UTC"));
        assert!(iso_formatted.contains("T"));
    }

    #[test]
    fn test_duration_formatting() {
        let duration = Duration::seconds(3661); // 1 hour, 1 minute, 1 second
        let formatted = format_duration(duration);
        assert_eq!(formatted, "1h 1m");

        let short_duration = Duration::seconds(30);
        let short_formatted = format_duration(short_duration);
        assert_eq!(short_formatted, "30s");
    }

    #[test]
    fn test_datetime_operations() {
        let dt = Utc::now();
        let future = add_duration(&dt, Duration::hours(1));
        let past = subtract_duration(&dt, Duration::hours(1));

        assert!(is_future(&future));
        assert!(is_past(&past));

        let duration = duration_between(&past, &future);
        assert_eq!(duration, Duration::hours(2));
    }

    #[test]
    fn test_day_boundaries() {
        let dt = Utc::now();
        let start = start_of_day(&dt);
        let end = end_of_day(&dt);

        assert_eq!(start.hour(), 0);
        assert_eq!(start.minute(), 0);
        assert_eq!(start.second(), 0);

        assert_eq!(end.hour(), 23);
        assert_eq!(end.minute(), 59);
        assert_eq!(end.second(), 59);
    }

    #[test]
    fn test_performance_timer() {
        let timer = perf::Timer::start("test_timer");
        std::thread::sleep(std::time::Duration::from_millis(10));
        let elapsed = timer.stop();

        assert!(elapsed.as_millis() >= 10);
    }

    #[test]
    fn test_time_it() {
        let (result, elapsed) = perf::time_it("test_operation", || {
            std::thread::sleep(std::time::Duration::from_millis(5));
            42
        });

        assert_eq!(result, 42);
        assert!(elapsed.as_millis() >= 5);
    }

    #[tokio::test]
    async fn test_rate_limiter() {
        let limiter = rate_limit::RateLimiter::new(2, 1, 60); // 2 requests per minute

        // First two requests should be allowed
        assert!(limiter.is_allowed("user1").await);
        assert!(limiter.is_allowed("user1").await);

        // Third request should be denied
        assert!(!limiter.is_allowed("user1").await);

        // Different user should have separate bucket
        assert!(limiter.is_allowed("user2").await);
    }

    #[tokio::test]
    async fn test_sleep_utilities() {
        let start = std::time::Instant::now();
        sleep::sleep_millis(10).await;
        let elapsed = start.elapsed();

        assert!(elapsed.as_millis() >= 10);
    }

    #[tokio::test]
    async fn test_backoff_sleep() {
        let start = std::time::Instant::now();
        sleep::sleep_with_backoff(1, 10, 1000).await; // Should sleep ~20ms
        let elapsed = start.elapsed();

        assert!(elapsed.as_millis() >= 20);
    }
}