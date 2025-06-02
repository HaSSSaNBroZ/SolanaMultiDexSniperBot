//! Authentication service implementation
//!
//! This module provides user authentication, session management,
//! role-based access control, and Telegram user verification.

use crate::config::models::{SecurityConfig, TelegramConfig};
use crate::core::result::AppResult;
use crate::core::error::AppError;
use super::{SecurityEvent, SecuritySeverity, utils};

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc, Duration};
use tracing::{debug, info, warn, error, instrument};
use serde::{Serialize, Deserialize};

/// Result type for authentication operations
pub type AuthenticationResult<T> = Result<T, AuthenticationError>;

/// Authentication-specific errors
#[derive(Debug, thiserror::Error)]
pub enum AuthenticationError {
    /// Invalid credentials
    #[error("Invalid credentials")]
    InvalidCredentials,

    /// Account locked
    #[error("Account locked until {0}")]
    AccountLocked(DateTime<Utc>),

    /// Session expired
    #[error("Session expired")]
    SessionExpired,

    /// Session not found
    #[error("Session not found")]
    SessionNotFound,

    /// Unauthorized access
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    /// User not found
    #[error("User not found: {0}")]
    UserNotFound(String),

    /// Invalid token
    #[error("Invalid token")]
    InvalidToken,

    /// Two-factor authentication required
    #[error("Two-factor authentication required")]
    TwoFactorRequired,

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
}

/// Authentication service
#[derive(Debug)]
pub struct AuthenticationService {
    /// Session manager
    session_manager: Arc<SessionManager>,
    /// User store
    user_store: Arc<RwLock<UserStore>>,
    /// Configuration
    config: SecurityConfig,
    /// Telegram configuration
    telegram_config: TelegramConfig,
    /// Failed login attempts tracker
    failed_attempts: Arc<RwLock<HashMap<String, FailedAttempts>>>,
    /// Rate limiter
    rate_limiter: Arc<RwLock<RateLimiter>>,
}

impl AuthenticationService {
    /// Create a new authentication service
    #[instrument(skip(security_config, telegram_config))]
    pub fn new(security_config: &SecurityConfig, telegram_config: &TelegramConfig) -> AppResult<Self> {
        info!("🔓 Initializing authentication service");

        let session_manager = Arc::new(SessionManager::new(security_config));
        let user_store = Arc::new(RwLock::new(UserStore::new(telegram_config)));
        let failed_attempts = Arc::new(RwLock::new(HashMap::new()));
        let rate_limiter = Arc::new(RwLock::new(RateLimiter::new(security_config)));

        info!("✅ Authentication service initialized");

        Ok(Self {
            session_manager,
            user_store,
            config: security_config.clone(),
            telegram_config: telegram_config.clone(),
            failed_attempts,
            rate_limiter,
        })
    }

    /// Authenticate a Telegram user
    #[instrument(skip(self))]
    pub async fn authenticate_telegram_user(
        &self,
        telegram_id: i64,
        username: Option<String>,
        first_name: Option<String>,
        ip_address: Option<String>,
    ) -> AuthenticationResult<Session> {
        debug!("🔐 Authenticating Telegram user: {}", telegram_id);

        // Check rate limiting
        self.check_rate_limit(&telegram_id.to_string(), &ip_address).await?;

        // Check if account is locked
        self.check_account_lock(&telegram_id.to_string()).await?;

        // Verify user is allowed
        let user = self.verify_telegram_user(telegram_id, username, first_name).await?;

        // Create session
        let session = self.session_manager.create_session(
            user.id.clone(),
            user.role.clone(),
            ip_address.clone(),
        ).await?;

        // Clear failed attempts
        self.clear_failed_attempts(&telegram_id.to_string()).await;

        info!("✅ User authenticated: {} ({})", user.id, user.role);

        Ok(session)
    }

    /// Verify Telegram user
    async fn verify_telegram_user(
        &self,
        telegram_id: i64,
        username: Option<String>,
        first_name: Option<String>,
    ) -> AuthenticationResult<User> {
        let mut user_store = self.user_store.write().await;

        // Check if user exists
        if let Some(user) = user_store.get_user_by_telegram_id(telegram_id).await {
            // Update user info if changed
            if user.username != username || user.first_name != first_name {
                user_store.update_user_info(telegram_id, username, first_name).await?;
            }
            return Ok(user);
        }

        // Check if user is allowed
        if !self.telegram_config.allowed_user_ids.is_empty() &&
            !self.telegram_config.allowed_user_ids.contains(&telegram_id) {
            return Err(AuthenticationError::Unauthorized(
                "User not in allowed list".to_string()
            ));
        }

        // Determine role
        let role = if telegram_id == self.telegram_config.admin_chat_id {
            UserRole::Admin
        } else {
            UserRole::User
        };

        // Create new user
        let user = user_store.create_user(
            telegram_id,
            username,
            first_name,
            role,
        ).await?;

        Ok(user)
    }

    /// Validate session token
    #[instrument(skip(self, token))]
    pub async fn validate_session(&self, token: &str) -> AuthenticationResult<Session> {
        self.session_manager.validate_session(token).await
    }

    /// Invalidate session
    #[instrument(skip(self, token))]
    pub async fn invalidate_session(&self, token: &str) -> AuthenticationResult<()> {
        self.session_manager.invalidate_session(token).await
    }

    /// Check user permission
    #[instrument(skip(self))]
    pub async fn check_permission(
        &self,
        session: &Session,
        permission: Permission,
    ) -> AuthenticationResult<()> {
        if !session.role.has_permission(&permission) {
            return Err(AuthenticationError::Unauthorized(
                format!("Missing permission: {:?}", permission)
            ));
        }

        Ok(())
    }

    /// Get active sessions for user
    pub async fn get_user_sessions(&self, user_id: &str) -> Vec<Session> {
        self.session_manager.get_user_sessions(user_id).await
    }

    /// Check rate limiting
    async fn check_rate_limit(
        &self,
        user_id: &str,
        ip_address: &Option<String>,
    ) -> AuthenticationResult<()> {
        let mut rate_limiter = self.rate_limiter.write().await;

        if !rate_limiter.check_limit(user_id, ip_address) {
            warn!("🚫 Rate limit exceeded for user: {}", user_id);
            return Err(AuthenticationError::Unauthorized(
                "Rate limit exceeded".to_string()
            ));
        }

        Ok(())
    }

    /// Check if account is locked
    async fn check_account_lock(&self, user_id: &str) -> AuthenticationResult<()> {
        let attempts = self.failed_attempts.read().await;

        if let Some(failed) = attempts.get(user_id) {
            if let Some(locked_until) = failed.locked_until {
                if Utc::now() < locked_until {
                    return Err(AuthenticationError::AccountLocked(locked_until));
                }
            }
        }

        Ok(())
    }

    /// Record failed login attempt
    pub async fn record_failed_attempt(&self, user_id: &str) -> AuthenticationResult<()> {
        let mut attempts = self.failed_attempts.write().await;

        let entry = attempts.entry(user_id.to_string()).or_insert_with(|| FailedAttempts {
            count: 0,
            first_attempt_at: Utc::now(),
            last_attempt_at: Utc::now(),
            locked_until: None,
        });

        entry.count += 1;
        entry.last_attempt_at = Utc::now();

        if entry.count >= self.config.max_failed_attempts {
            let lock_duration = Duration::minutes(self.config.lockout_duration_minutes as i64);
            entry.locked_until = Some(Utc::now() + lock_duration);

            warn!("🔒 Account locked for user {} until {}",
                  user_id, entry.locked_until.unwrap());
        }

        Ok(())
    }

    /// Clear failed attempts
    async fn clear_failed_attempts(&self, user_id: &str) {
        let mut attempts = self.failed_attempts.write().await;
        attempts.remove(user_id);
    }

    /// Clear all sessions
    pub async fn clear_sessions(&self) -> AppResult<()> {
        self.session_manager.clear_all_sessions().await
    }

    /// Health check
    pub async fn health_check(&self) -> AppResult<()> {
        // Check session manager
        let active_sessions = self.session_manager.get_active_session_count().await;
        debug!("Active sessions: {}", active_sessions);

        // Check user store
        let user_count = self.user_store.read().await.get_user_count();
        debug!("Registered users: {}", user_count);

        Ok(())
    }

    /// Get authentication metrics
    pub async fn get_metrics(&self) -> AuthenticationMetrics {
        AuthenticationMetrics {
            active_sessions: self.session_manager.get_active_session_count().await,
            registered_users: self.user_store.read().await.get_user_count(),
            failed_attempts: self.failed_attempts.read().await.len(),
            locked_accounts: self.failed_attempts.read().await
                .values()
                .filter(|a| a.locked_until.is_some())
                .count(),
        }
    }
}

/// Session manager
#[derive(Debug)]
pub struct SessionManager {
    /// Active sessions
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    /// Session timeout duration
    session_timeout: Duration,
    /// Enable two-factor authentication
    enable_2fa: bool,
}

impl SessionManager {
    fn new(config: &SecurityConfig) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            session_timeout: Duration::minutes(config.session_timeout_minutes as i64),
            enable_2fa: config.enable_2fa,
        }
    }

    async fn create_session(
        &self,
        user_id: String,
        role: UserRole,
        ip_address: Option<String>,
    ) -> AuthenticationResult<Session> {
        let token = SessionToken::generate();
        let expires_at = Utc::now() + self.session_timeout;

        let session = Session {
            id: uuid::Uuid::new_v4().to_string(),
            token: token.clone(),
            user_id,
            role,
            created_at: Utc::now(),
            expires_at,
            last_activity: Utc::now(),
            ip_address,
            two_factor_verified: !self.enable_2fa, // Skip if 2FA disabled
            metadata: HashMap::new(),
        };

        let mut sessions = self.sessions.write().await;
        sessions.insert(token.value.clone(), session.clone());

        Ok(session)
    }

    async fn validate_session(&self, token: &str) -> AuthenticationResult<Session> {
        let mut sessions = self.sessions.write().await;

        match sessions.get_mut(token) {
            Some(session) => {
                // Check expiration
                if Utc::now() > session.expires_at {
                    sessions.remove(token);
                    return Err(AuthenticationError::SessionExpired);
                }

                // Check 2FA requirement
                if self.enable_2fa && !session.two_factor_verified {
                    return Err(AuthenticationError::TwoFactorRequired);
                }

                // Update last activity
                session.last_activity = Utc::now();

                Ok(session.clone())
            }
            None => Err(AuthenticationError::SessionNotFound),
        }
    }

    async fn invalidate_session(&self, token: &str) -> AuthenticationResult<()> {
        let mut sessions = self.sessions.write().await;

        if sessions.remove(token).is_some() {
            Ok(())
        } else {
            Err(AuthenticationError::SessionNotFound)
        }
    }

    async fn get_user_sessions(&self, user_id: &str) -> Vec<Session> {
        let sessions = self.sessions.read().await;

        sessions.values()
            .filter(|s| s.user_id == user_id)
            .cloned()
            .collect()
    }

    async fn get_active_session_count(&self) -> usize {
        let sessions = self.sessions.read().await;
        sessions.len()
    }

    async fn clear_all_sessions(&self) -> AppResult<()> {
        let mut sessions = self.sessions.write().await;
        sessions.clear();
        Ok(())
    }

    /// Clean up expired sessions
    pub async fn cleanup_expired_sessions(&self) -> usize {
        let mut sessions = self.sessions.write().await;
        let now = Utc::now();

        let expired_tokens: Vec<String> = sessions
            .iter()
            .filter(|(_, session)| session.expires_at < now)
            .map(|(token, _)| token.clone())
            .collect();

        let count = expired_tokens.len();

        for token in expired_tokens {
            sessions.remove(&token);
        }

        if count > 0 {
            debug!("🧹 Cleaned up {} expired sessions", count);
        }

        count
    }
}

/// Session information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Session ID
    pub id: String,
    /// Session token
    pub token: SessionToken,
    /// User ID
    pub user_id: String,
    /// User role
    pub role: UserRole,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Expiration timestamp
    pub expires_at: DateTime<Utc>,
    /// Last activity timestamp
    pub last_activity: DateTime<Utc>,
    /// IP address
    pub ip_address: Option<String>,
    /// Two-factor authentication verified
    pub two_factor_verified: bool,
    /// Session metadata
    pub metadata: HashMap<String, String>,
}

/// Session token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionToken {
    /// Token value
    pub value: String,
}

impl SessionToken {
    /// Generate a new session token
    pub fn generate() -> Self {
        Self {
            value: utils::generate_secure_token(super::constants::SESSION_TOKEN_LENGTH),
        }
    }
}

/// User roles
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserRole {
    /// Administrator with full access
    Admin,
    /// Regular user with trading access
    User,
    /// View-only access
    Viewer,
}

impl UserRole {
    /// Check if role has permission
    pub fn has_permission(&self, permission: &Permission) -> bool {
        match self {
            UserRole::Admin => true, // Admin has all permissions
            UserRole::User => match permission {
                Permission::ViewDashboard => true,
                Permission::ExecuteTrades => true,
                Permission::ManageWallets => true,
                Permission::ViewReports => true,
                Permission::ConfigureSettings => false,
                Permission::ManageUsers => false,
                Permission::ViewAuditLogs => false,
            },
            UserRole::Viewer => match permission {
                Permission::ViewDashboard => true,
                Permission::ViewReports => true,
                _ => false,
            },
        }
    }
}

impl std::fmt::Display for UserRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserRole::Admin => write!(f, "Admin"),
            UserRole::User => write!(f, "User"),
            UserRole::Viewer => write!(f, "Viewer"),
        }
    }
}

/// Permissions
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Permission {
    /// View dashboard
    ViewDashboard,
    /// Execute trades
    ExecuteTrades,
    /// Manage wallets
    ManageWallets,
    /// View reports
    ViewReports,
    /// Configure settings
    ConfigureSettings,
    /// Manage users
    ManageUsers,
    /// View audit logs
    ViewAuditLogs,
}

/// User information
#[derive(Debug, Clone)]
pub struct User {
    /// User ID
    pub id: String,
    /// Telegram ID
    pub telegram_id: i64,
    /// Username
    pub username: Option<String>,
    /// First name
    pub first_name: Option<String>,
    /// User role
    pub role: UserRole,
    /// Account created at
    pub created_at: DateTime<Utc>,
    /// Last login
    pub last_login: Option<DateTime<Utc>>,
    /// Account active
    pub is_active: bool,
}

/// User store
#[derive(Debug)]
struct UserStore {
    /// Users by Telegram ID
    users: HashMap<i64, User>,
    /// Admin chat ID
    admin_chat_id: i64,
    /// Allowed user IDs
    allowed_user_ids: Vec<i64>,
}

impl UserStore {
    fn new(telegram_config: &TelegramConfig) -> Self {
        Self {
            users: HashMap::new(),
            admin_chat_id: telegram_config.admin_chat_id,
            allowed_user_ids: telegram_config.allowed_user_ids.clone(),
        }
    }

    async fn get_user_by_telegram_id(&self, telegram_id: i64) -> Option<User> {
        self.users.get(&telegram_id).cloned()
    }

    async fn create_user(
        &mut self,
        telegram_id: i64,
        username: Option<String>,
        first_name: Option<String>,
        role: UserRole,
    ) -> AuthenticationResult<User> {
        let user = User {
            id: format!("user_{}", telegram_id),
            telegram_id,
            username,
            first_name,
            role,
            created_at: Utc::now(),
            last_login: Some(Utc::now()),
            is_active: true,
        };

        self.users.insert(telegram_id, user.clone());

        Ok(user)
    }

    async fn update_user_info(
        &mut self,
        telegram_id: i64,
        username: Option<String>,
        first_name: Option<String>,
    ) -> AuthenticationResult<()> {
        if let Some(user) = self.users.get_mut(&telegram_id) {
            user.username = username;
            user.first_name = first_name;
            user.last_login = Some(Utc::now());
            Ok(())
        } else {
            Err(AuthenticationError::UserNotFound(telegram_id.to_string()))
        }
    }

    fn get_user_count(&self) -> usize {
        self.users.len()
    }
}

/// Failed login attempts tracker
#[derive(Debug)]
struct FailedAttempts {
    /// Number of failed attempts
    count: u32,
    /// First attempt timestamp
    first_attempt_at: DateTime<Utc>,
    /// Last attempt timestamp
    last_attempt_at: DateTime<Utc>,
    /// Account locked until
    locked_until: Option<DateTime<Utc>>,
}

/// Rate limiter
#[derive(Debug)]
struct RateLimiter {
    /// Request counts by key
    requests: HashMap<String, Vec<DateTime<Utc>>>,
    /// Rate limit per minute
    limit_per_minute: u32,
    /// Window duration
    window: Duration,
}

impl RateLimiter {
    fn new(config: &SecurityConfig) -> Self {
        Self {
            requests: HashMap::new(),
            limit_per_minute: config.rate_limit_per_minute.unwrap_or(60),
            window: Duration::minutes(1),
        }
    }

    fn check_limit(&mut self, user_id: &str, ip_address: &Option<String>) -> bool {
        let key = if let Some(ip) = ip_address {
            format!("{}:{}", user_id, ip)
        } else {
            user_id.to_string()
        };

        let now = Utc::now();
        let window_start = now - self.window;

        // Get or create request list
        let requests = self.requests.entry(key).or_insert_with(Vec::new);

        // Remove old requests
        requests.retain(|&req_time| req_time > window_start);

        // Check if under limit
        if requests.len() >= self.limit_per_minute as usize {
            return false;
        }

        // Add new request
        requests.push(now);
        true
    }
}

/// Authentication metrics
#[derive(Debug, Clone)]
pub struct AuthenticationMetrics {
    /// Number of active sessions
    pub active_sessions: usize,
    /// Number of registered users
    pub registered_users: usize,
    /// Number of users with failed attempts
    pub failed_attempts: usize,
    /// Number of locked accounts
    pub locked_accounts: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConfigLoader;

    #[tokio::test]
    async fn test_authentication_service() {
        let config = ConfigLoader::new().without_env().create_default_config();
        let service = AuthenticationService::new(&config.security, &config.telegram).unwrap();

        // Test session creation and validation
        let session = service.authenticate_telegram_user(
            123456789,
            Some("testuser".to_string()),
            Some("Test".to_string()),
            Some("127.0.0.1".to_string()),
        ).await;

        // Should fail if user not in allowed list (default is empty = all allowed)
        if !config.telegram.allowed_user_ids.is_empty() {
            assert!(session.is_err());
        } else {
            assert!(session.is_ok());
        }
    }

    #[test]
    fn test_user_role_permissions() {
        let admin = UserRole::Admin;
        assert!(admin.has_permission(&Permission::ManageUsers));
        assert!(admin.has_permission(&Permission::ExecuteTrades));

        let user = UserRole::User;
        assert!(!user.has_permission(&Permission::ManageUsers));
        assert!(user.has_permission(&Permission::ExecuteTrades));

        let viewer = UserRole::Viewer;
        assert!(!viewer.has_permission(&Permission::ExecuteTrades));
        assert!(viewer.has_permission(&Permission::ViewDashboard));
    }

    #[test]
    fn test_session_token_generation() {
        let token1 = SessionToken::generate();
        let token2 = SessionToken::generate();

        assert_eq!(token1.value.len(), super::super::constants::SESSION_TOKEN_LENGTH);
        assert_ne!(token1.value, token2.value); // Should be unique
    }

    #[tokio::test]
    async fn test_rate_limiting() {
        let mut config = ConfigLoader::new().without_env().create_default_config();
        config.security.enable_rate_limiting = true;
        config.security.rate_limit_per_minute = Some(3);

        let mut rate_limiter = RateLimiter::new(&config.security);

        // First 3 requests should pass
        assert!(rate_limiter.check_limit("user1", &None));
        assert!(rate_limiter.check_limit("user1", &None));
        assert!(rate_limiter.check_limit("user1", &None));

        // 4th request should fail
        assert!(!rate_limiter.check_limit("user1", &None));

        // Different user should have separate limit
        assert!(rate_limiter.check_limit("user2", &None));
    }
}