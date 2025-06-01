// Package config provides configuration management for the Solana Sniper Bot
// It handles loading, validation, and management of all application settings
// including trading parameters, risk management, and system configurations.
package config

import (
	"time"
)

// Config represents the complete application configuration
type Config struct {
	// Environment settings
	Environment string `mapstructure:"environment" validate:"required,oneof=development staging production"`
	Debug       bool   `mapstructure:"debug"`

	// Server configuration
	Server ServerConfig `mapstructure:"server"`

	// Database configuration
	Database DatabaseConfig `mapstructure:"database"`

	// Redis configuration
	Redis RedisConfig `mapstructure:"redis"`

	// Solana blockchain configuration
	Solana SolanaConfig `mapstructure:"solana"`

	// Trading configuration
	Trading TradingConfig `mapstructure:"trading"`

	// Risk management configuration
	Risk RiskConfig `mapstructure:"risk"`

	// Telegram bot configuration
	Telegram TelegramConfig `mapstructure:"telegram"`

	// Logging configuration
	Log LogConfig `mapstructure:"log"`

	// Performance configuration
	Performance PerformanceConfig `mapstructure:"performance"`

	// Security configuration
	Security SecurityConfig `mapstructure:"security"`
}

// ServerConfig holds HTTP server configuration
type ServerConfig struct {
	Port           int           `mapstructure:"port" validate:"required,min=1,max=65535"`
	Host           string        `mapstructure:"host"`
	ReadTimeout    time.Duration `mapstructure:"read_timeout"`
	WriteTimeout   time.Duration `mapstructure:"write_timeout"`
	IdleTimeout    time.Duration `mapstructure:"idle_timeout"`
	MaxHeaderBytes int           `mapstructure:"max_header_bytes"`
	TLS            TLSConfig     `mapstructure:"tls"`
}

// TLSConfig holds TLS/SSL configuration
type TLSConfig struct {
	Enabled  bool   `mapstructure:"enabled"`
	CertFile string `mapstructure:"cert_file"`
	KeyFile  string `mapstructure:"key_file"`
}

// DatabaseConfig holds database connection configuration
type DatabaseConfig struct {
	Host            string        `mapstructure:"host" validate:"required"`
	Port            int           `mapstructure:"port" validate:"required,min=1,max=65535"`
	Name            string        `mapstructure:"name" validate:"required"`
	User            string        `mapstructure:"user" validate:"required"`
	Password        string        `mapstructure:"password" validate:"required"`
	SSLMode         string        `mapstructure:"ssl_mode" validate:"oneof=disable require verify-ca verify-full"`
	MaxOpenConns    int           `mapstructure:"max_open_conns"`
	MaxIdleConns    int           `mapstructure:"max_idle_conns"`
	ConnMaxLifetime time.Duration `mapstructure:"conn_max_lifetime"`
	ConnMaxIdleTime time.Duration `mapstructure:"conn_max_idle_time"`
}

// RedisConfig holds Redis configuration
type RedisConfig struct {
	Host         string        `mapstructure:"host" validate:"required"`
	Port         int           `mapstructure:"port" validate:"required,min=1,max=65535"`
	Password     string        `mapstructure:"password"`
	DB           int           `mapstructure:"db" validate:"min=0,max=15"`
	MaxRetries   int           `mapstructure:"max_retries"`
	PoolSize     int           `mapstructure:"pool_size"`
	DialTimeout  time.Duration `mapstructure:"dial_timeout"`
	ReadTimeout  time.Duration `mapstructure:"read_timeout"`
	WriteTimeout time.Duration `mapstructure:"write_timeout"`
	IdleTimeout  time.Duration `mapstructure:"idle_timeout"`
}

// SolanaConfig holds Solana blockchain configuration
type SolanaConfig struct {
	// RPC endpoints
	MainnetRPC string   `mapstructure:"mainnet_rpc" validate:"required,url"`
	DevnetRPC  string   `mapstructure:"devnet_rpc" validate:"url"`
	TestnetRPC string   `mapstructure:"testnet_rpc" validate:"url"`
	RPCPool    []string `mapstructure:"rpc_pool"`

	// Network settings
	Network        string        `mapstructure:"network" validate:"required,oneof=mainnet devnet testnet"`
	Commitment     string        `mapstructure:"commitment" validate:"oneof=processed confirmed finalized"`
	MaxRetries     int           `mapstructure:"max_retries"`
	RequestTimeout time.Duration `mapstructure:"request_timeout"`
	ConnectionPool int           `mapstructure:"connection_pool"`

	// WebSocket settings
	WSEndpoint     string        `mapstructure:"ws_endpoint"`
	WSReconnect    bool          `mapstructure:"ws_reconnect"`
	WSTimeout      time.Duration `mapstructure:"ws_timeout"`
	WSPingInterval time.Duration `mapstructure:"ws_ping_interval"`

	// Priority fees and MEV
	PriorityFee    uint64 `mapstructure:"priority_fee"`
	MaxPriorityFee uint64 `mapstructure:"max_priority_fee"`
	JitoEnabled    bool   `mapstructure:"jito_enabled"`
	JitoTipAccount string `mapstructure:"jito_tip_account"`
	JitoEndpoint   string `mapstructure:"jito_endpoint"`
}

// TradingConfig holds trading-specific configuration
type TradingConfig struct {
	// Trading mode and automation
	Mode        string `mapstructure:"mode" validate:"required,oneof=manual semi-auto full-auto"`
	AutoTrading bool   `mapstructure:"auto_trading"`
	DryRun      bool   `mapstructure:"dry_run"`

	// Position management
	MaxPositions        int     `mapstructure:"max_positions" validate:"min=1,max=100"`
	MaxConcurrentTrades int     `mapstructure:"max_concurrent_trades" validate:"min=1,max=50"`
	DefaultPositionSize float64 `mapstructure:"default_position_size" validate:"min=0.001,max=1000"`
	MaxPositionSize     float64 `mapstructure:"max_position_size" validate:"min=0.001,max=1000"`
	MinPositionSize     float64 `mapstructure:"min_position_size" validate:"min=0.001,max=1"`

	// Slippage and fees
	DefaultSlippage float64 `mapstructure:"default_slippage" validate:"min=0.1,max=50"`
	MaxSlippage     float64 `mapstructure:"max_slippage" validate:"min=0.1,max=100"`
	MaxFee          float64 `mapstructure:"max_fee" validate:"min=0,max=10"`

	// Liquidity requirements
	MinLiquidity float64 `mapstructure:"min_liquidity" validate:"min=0.1,max=10000"`
	MinMarketCap float64 `mapstructure:"min_market_cap"`
	MaxMarketCap float64 `mapstructure:"max_market_cap"`
	MinHolders   int     `mapstructure:"min_holders" validate:"min=1,max=10000"`

	// Timing settings
	MaxTokenAge      time.Duration `mapstructure:"max_token_age"`
	MaxHoldTime      time.Duration `mapstructure:"max_hold_time"`
	ScanInterval     time.Duration `mapstructure:"scan_interval"`
	ExecutionTimeout time.Duration `mapstructure:"execution_timeout"`
	RetryDelay       time.Duration `mapstructure:"retry_delay"`
	MaxRetries       int           `mapstructure:"max_retries"`

	// DEX settings
	EnabledDEXs []string             `mapstructure:"enabled_dexs"`
	DEXPriority map[string]int       `mapstructure:"dex_priority"`
	DEXConfig   map[string]DEXConfig `mapstructure:"dex_config"`
}

// DEXConfig holds configuration for specific DEX
type DEXConfig struct {
	Enabled        bool    `mapstructure:"enabled"`
	Priority       int     `mapstructure:"priority"`
	MinLiquidity   float64 `mapstructure:"min_liquidity"`
	MaxSlippage    float64 `mapstructure:"max_slippage"`
	ProgramID      string  `mapstructure:"program_id"`
	FactoryAddress string  `mapstructure:"factory_address"`
	RouterAddress  string  `mapstructure:"router_address"`
}

// RiskConfig holds risk management configuration
type RiskConfig struct {
	// Global risk settings
	Level          string  `mapstructure:"level" validate:"required,oneof=conservative moderate aggressive"`
	MaxDailyLoss   float64 `mapstructure:"max_daily_loss" validate:"min=0,max=100"`
	MaxWeeklyLoss  float64 `mapstructure:"max_weekly_loss" validate:"min=0,max=100"`
	MaxMonthlyLoss float64 `mapstructure:"max_monthly_loss" validate:"min=0,max=100"`
	MaxDrawdown    float64 `mapstructure:"max_drawdown" validate:"min=0,max=100"`

	// Position sizing
	PositionSizeMethod string  `mapstructure:"position_size_method" validate:"oneof=fixed percentage kelly"`
	RiskPerTrade       float64 `mapstructure:"risk_per_trade" validate:"min=0.1,max=10"`
	MaxRiskPerToken    float64 `mapstructure:"max_risk_per_token" validate:"min=0.1,max=50"`

	// Stop loss and take profit
	DefaultStopLoss     float64 `mapstructure:"default_stop_loss" validate:"min=1,max=90"`
	DefaultTakeProfit   float64 `mapstructure:"default_take_profit" validate:"min=5,max=1000"`
	TrailingStopEnabled bool    `mapstructure:"trailing_stop_enabled"`
	TrailingStopPercent float64 `mapstructure:"trailing_stop_percent" validate:"min=0.1,max=50"`

	// Portfolio diversification
	MaxTokensPerSector    int     `mapstructure:"max_tokens_per_sector"`
	MaxAllocationPerToken float64 `mapstructure:"max_allocation_per_token" validate:"min=1,max=100"`
	DiversificationLimit  int     `mapstructure:"diversification_limit" validate:"min=1,max=50"`

	// Emergency controls
	EmergencyStopLoss       float64 `mapstructure:"emergency_stop_loss" validate:"min=10,max=90"`
	PanicSellEnabled        bool    `mapstructure:"panic_sell_enabled"`
	CircuitBreakerEnabled   bool    `mapstructure:"circuit_breaker_enabled"`
	CircuitBreakerThreshold float64 `mapstructure:"circuit_breaker_threshold" validate:"min=5,max=50"`

	// Honeypot and scam protection
	HoneypotCheckEnabled  bool          `mapstructure:"honeypot_check_enabled"`
	ContractVerification  bool          `mapstructure:"contract_verification"`
	BlacklistedDevelopers []string      `mapstructure:"blacklisted_developers"`
	BlacklistedTokens     []string      `mapstructure:"blacklisted_tokens"`
	MinLiquidityLockTime  time.Duration `mapstructure:"min_liquidity_lock_time"`
}

// TelegramConfig holds Telegram bot configuration
type TelegramConfig struct {
	// Bot settings
	BotToken    string `mapstructure:"bot_token" validate:"required"`
	BotUsername string `mapstructure:"bot_username"`
	WebhookURL  string `mapstructure:"webhook_url"`
	WebhookPort int    `mapstructure:"webhook_port"`

	// User management
	AdminUserIDs    []int64 `mapstructure:"admin_user_ids"`
	AllowedUserIDs  []int64 `mapstructure:"allowed_user_ids"`
	AllowedGroupIDs []int64 `mapstructure:"allowed_group_ids"`

	// Notification settings
	NotificationsEnabled bool `mapstructure:"notifications_enabled"`
	TradeAlerts          bool `mapstructure:"trade_alerts"`
	ProfitAlerts         bool `mapstructure:"profit_alerts"`
	LossAlerts           bool `mapstructure:"loss_alerts"`
	RiskAlerts           bool `mapstructure:"risk_alerts"`
	SystemAlerts         bool `mapstructure:"system_alerts"`

	// Message settings
	DeleteMessages    bool          `mapstructure:"delete_messages"`
	MessageLifetime   time.Duration `mapstructure:"message_lifetime"`
	CommandCooldown   time.Duration `mapstructure:"command_cooldown"`
	MaxMessagesPerMin int           `mapstructure:"max_messages_per_min"`

	// Interface settings
	Language        string `mapstructure:"language" validate:"oneof=en ar es fr de"`
	Timezone        string `mapstructure:"timezone"`
	CurrencyDisplay string `mapstructure:"currency_display" validate:"oneof=USD EUR SOL"`
	DecimalPlaces   int    `mapstructure:"decimal_places" validate:"min=2,max=8"`
}

// LogConfig holds logging configuration
type LogConfig struct {
	Level      string `mapstructure:"level" validate:"required,oneof=debug info warn error fatal"`
	Format     string `mapstructure:"format" validate:"oneof=json text"`
	Output     string `mapstructure:"output" validate:"oneof=stdout stderr file"`
	FilePath   string `mapstructure:"file_path"`
	MaxSize    int    `mapstructure:"max_size"`    // MB
	MaxAge     int    `mapstructure:"max_age"`     // Days
	MaxBackups int    `mapstructure:"max_backups"` // Number of files
	Compress   bool   `mapstructure:"compress"`

	// Structured logging
	EnableColors     bool `mapstructure:"enable_colors"`
	EnableCaller     bool `mapstructure:"enable_caller"`
	EnableStacktrace bool `mapstructure:"enable_stacktrace"`

	// Performance logging
	SlowQueryThreshold time.Duration `mapstructure:"slow_query_threshold"`
	LogQueries         bool          `mapstructure:"log_queries"`
	LogRequests        bool          `mapstructure:"log_requests"`
}

// PerformanceConfig holds performance-related configuration
type PerformanceConfig struct {
	// Worker pools
	ScannerWorkers  int `mapstructure:"scanner_workers" validate:"min=1,max=100"`
	ExecutorWorkers int `mapstructure:"executor_workers" validate:"min=1,max=50"`
	AnalyzerWorkers int `mapstructure:"analyzer_workers" validate:"min=1,max=20"`

	// Queue settings
	ScannerQueueSize  int `mapstructure:"scanner_queue_size" validate:"min=100,max=10000"`
	ExecutorQueueSize int `mapstructure:"executor_queue_size" validate:"min=50,max=5000"`
	AnalyzerQueueSize int `mapstructure:"analyzer_queue_size" validate:"min=100,max=5000"`

	// Timeouts and intervals
	ScanInterval     time.Duration `mapstructure:"scan_interval"`
	AnalysisTimeout  time.Duration `mapstructure:"analysis_timeout"`
	ExecutionTimeout time.Duration `mapstructure:"execution_timeout"`
	CleanupInterval  time.Duration `mapstructure:"cleanup_interval"`

	// Caching
	CacheEnabled  bool          `mapstructure:"cache_enabled"`
	CacheSize     int           `mapstructure:"cache_size"`
	CacheTTL      time.Duration `mapstructure:"cache_ttl"`
	TokenCacheTTL time.Duration `mapstructure:"token_cache_ttl"`
	PriceCacheTTL time.Duration `mapstructure:"price_cache_ttl"`

	// Rate limiting
	RPCRateLimit      int `mapstructure:"rpc_rate_limit"`      // Requests per second
	APIRateLimit      int `mapstructure:"api_rate_limit"`      // Requests per second
	TelegramRateLimit int `mapstructure:"telegram_rate_limit"` // Messages per minute

	// Memory management
	MaxMemoryUsage  int64         `mapstructure:"max_memory_usage"` // Bytes
	GCInterval      time.Duration `mapstructure:"gc_interval"`
	GCTargetPercent int           `mapstructure:"gc_target_percent"`

	// Monitoring
	MetricsEnabled   bool `mapstructure:"metrics_enabled"`
	MetricsPort      int  `mapstructure:"metrics_port"`
	HealthCheckPort  int  `mapstructure:"health_check_port"`
	ProfilingEnabled bool `mapstructure:"profiling_enabled"`
	ProfilingPort    int  `mapstructure:"profiling_port"`
}

// SecurityConfig holds security-related configuration
type SecurityConfig struct {
	// Encryption
	EncryptionKey       string `mapstructure:"encryption_key" validate:"required"`
	EncryptionAlgorithm string `mapstructure:"encryption_algorithm" validate:"oneof=AES-256-GCM ChaCha20-Poly1305"`

	// JWT settings
	JWTSecret            string        `mapstructure:"jwt_secret" validate:"required"`
	JWTExpiration        time.Duration `mapstructure:"jwt_expiration"`
	JWTRefreshExpiration time.Duration `mapstructure:"jwt_refresh_expiration"`

	// Rate limiting
	RateLimitEnabled  bool          `mapstructure:"rate_limit_enabled"`
	RateLimitRequests int           `mapstructure:"rate_limit_requests"`
	RateLimitWindow   time.Duration `mapstructure:"rate_limit_window"`

	// CORS settings
	CORSEnabled bool     `mapstructure:"cors_enabled"`
	CORSOrigins []string `mapstructure:"cors_origins"`
	CORSMethods []string `mapstructure:"cors_methods"`
	CORSHeaders []string `mapstructure:"cors_headers"`

	// Security headers
	SecurityHeadersEnabled bool `mapstructure:"security_headers_enabled"`
	HSTSEnabled            bool `mapstructure:"hsts_enabled"`
	HSTSMaxAge             int  `mapstructure:"hsts_max_age"`

	// Wallet security
	WalletEncryption     bool          `mapstructure:"wallet_encryption"`
	WalletBackupEnabled  bool          `mapstructure:"wallet_backup_enabled"`
	WalletBackupInterval time.Duration `mapstructure:"wallet_backup_interval"`

	// Audit logging
	AuditLogEnabled  bool   `mapstructure:"audit_log_enabled"`
	AuditLogFile     string `mapstructure:"audit_log_file"`
	AuditLogRotation bool   `mapstructure:"audit_log_rotation"`

	// IP security
	IPWhitelistEnabled bool     `mapstructure:"ip_whitelist_enabled"`
	IPWhitelist        []string `mapstructure:"ip_whitelist"`
	IPBlacklistEnabled bool     `mapstructure:"ip_blacklist_enabled"`
	IPBlacklist        []string `mapstructure:"ip_blacklist"`
}

// GetDefault returns a default configuration
func GetDefault() *Config {
	return &Config{
		Environment: "development",
		Debug:       true,

		Server: ServerConfig{
			Port:           8080,
			Host:           "0.0.0.0",
			ReadTimeout:    30 * time.Second,
			WriteTimeout:   30 * time.Second,
			IdleTimeout:    60 * time.Second,
			MaxHeaderBytes: 1 << 20, // 1 MB
		},

		Database: DatabaseConfig{
			Host:            "localhost",
			Port:            5432,
			Name:            "sniper_bot",
			User:            "postgres",
			Password:        "password",
			SSLMode:         "disable",
			MaxOpenConns:    10,
			MaxIdleConns:    5,
			ConnMaxLifetime: time.Hour,
			ConnMaxIdleTime: 30 * time.Minute,
		},

		Redis: RedisConfig{
			Host:         "localhost",
			Port:         6379,
			DB:           0,
			MaxRetries:   3,
			PoolSize:     10,
			DialTimeout:  5 * time.Second,
			ReadTimeout:  3 * time.Second,
			WriteTimeout: 3 * time.Second,
			IdleTimeout:  5 * time.Minute,
		},

		Solana: SolanaConfig{
			MainnetRPC:     "https://api.mainnet-beta.solana.com",
			DevnetRPC:      "https://api.devnet.solana.com",
			Network:        "mainnet",
			Commitment:     "confirmed",
			MaxRetries:     3,
			RequestTimeout: 10 * time.Second,
			ConnectionPool: 10,
			WSReconnect:    true,
			WSTimeout:      30 * time.Second,
			WSPingInterval: 30 * time.Second,
			PriorityFee:    10000,
			MaxPriorityFee: 100000,
			JitoEnabled:    false,
		},

		Trading: TradingConfig{
			Mode:                "manual",
			AutoTrading:         false,
			DryRun:              true,
			MaxPositions:        5,
			MaxConcurrentTrades: 3,
			DefaultPositionSize: 0.1,
			MaxPositionSize:     1.0,
			MinPositionSize:     0.01,
			DefaultSlippage:     3.0,
			MaxSlippage:         10.0,
			MaxFee:              1.0,
			MinLiquidity:        1.0,
			MinHolders:          10,
			MaxTokenAge:         60 * time.Second,
			MaxHoldTime:         24 * time.Hour,
			ScanInterval:        100 * time.Millisecond,
			ExecutionTimeout:    5 * time.Second,
			RetryDelay:          1 * time.Second,
			MaxRetries:          3,
			EnabledDEXs:         []string{"raydium", "pumpfun"},
		},

		Risk: RiskConfig{
			Level:                   "conservative",
			MaxDailyLoss:            10.0,
			MaxWeeklyLoss:           20.0,
			MaxMonthlyLoss:          30.0,
			MaxDrawdown:             20.0,
			PositionSizeMethod:      "fixed",
			RiskPerTrade:            2.0,
			MaxRiskPerToken:         10.0,
			DefaultStopLoss:         20.0,
			DefaultTakeProfit:       50.0,
			TrailingStopEnabled:     true,
			TrailingStopPercent:     5.0,
			MaxTokensPerSector:      3,
			MaxAllocationPerToken:   20.0,
			DiversificationLimit:    5,
			EmergencyStopLoss:       50.0,
			PanicSellEnabled:        true,
			CircuitBreakerEnabled:   true,
			CircuitBreakerThreshold: 15.0,
			HoneypotCheckEnabled:    true,
			ContractVerification:    true,
			MinLiquidityLockTime:    24 * time.Hour,
		},

		Telegram: TelegramConfig{
			NotificationsEnabled: true,
			TradeAlerts:          true,
			ProfitAlerts:         true,
			LossAlerts:           true,
			RiskAlerts:           true,
			SystemAlerts:         true,
			DeleteMessages:       false,
			MessageLifetime:      24 * time.Hour,
			CommandCooldown:      1 * time.Second,
			MaxMessagesPerMin:    30,
			Language:             "en",
			Timezone:             "UTC",
			CurrencyDisplay:      "USD",
			DecimalPlaces:        4,
		},

		Log: LogConfig{
			Level:              "info",
			Format:             "json",
			Output:             "stdout",
			MaxSize:            100,
			MaxAge:             30,
			MaxBackups:         10,
			Compress:           true,
			EnableColors:       true,
			EnableCaller:       true,
			EnableStacktrace:   false,
			SlowQueryThreshold: 500 * time.Millisecond,
			LogQueries:         false,
			LogRequests:        true,
		},

		Performance: PerformanceConfig{
			ScannerWorkers:    10,
			ExecutorWorkers:   5,
			AnalyzerWorkers:   3,
			ScannerQueueSize:  1000,
			ExecutorQueueSize: 500,
			AnalyzerQueueSize: 300,
			ScanInterval:      100 * time.Millisecond,
			AnalysisTimeout:   5 * time.Second,
			ExecutionTimeout:  10 * time.Second,
			CleanupInterval:   1 * time.Hour,
			CacheEnabled:      true,
			CacheSize:         1000,
			CacheTTL:          5 * time.Minute,
			TokenCacheTTL:     1 * time.Minute,
			PriceCacheTTL:     30 * time.Second,
			RPCRateLimit:      100,
			APIRateLimit:      50,
			TelegramRateLimit: 20,
			MaxMemoryUsage:    512 * 1024 * 1024, // 512 MB
			GCInterval:        30 * time.Second,
			GCTargetPercent:   100,
			MetricsEnabled:    true,
			MetricsPort:       9090,
			HealthCheckPort:   8081,
			ProfilingEnabled:  false,
			ProfilingPort:     6060,
		},

		Security: SecurityConfig{
			EncryptionAlgorithm:    "AES-256-GCM",
			JWTExpiration:          24 * time.Hour,
			JWTRefreshExpiration:   7 * 24 * time.Hour,
			RateLimitEnabled:       true,
			RateLimitRequests:      100,
			RateLimitWindow:        1 * time.Minute,
			CORSEnabled:            true,
			CORSOrigins:            []string{"*"},
			CORSMethods:            []string{"GET", "POST", "PUT", "DELETE", "OPTIONS"},
			CORSHeaders:            []string{"Origin", "Content-Type", "Accept", "Authorization"},
			SecurityHeadersEnabled: true,
			HSTSEnabled:            true,
			HSTSMaxAge:             31536000,
			WalletEncryption:       true,
			WalletBackupEnabled:    true,
			WalletBackupInterval:   24 * time.Hour,
			AuditLogEnabled:        true,
			AuditLogRotation:       true,
			IPWhitelistEnabled:     false,
			IPBlacklistEnabled:     false,
		},
	}
}
