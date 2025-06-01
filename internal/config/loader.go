// Package config provides configuration loading and validation functionality
package config

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/go-playground/validator/v10"
	"github.com/spf13/viper"
)

// Loader handles configuration loading from various sources
type Loader struct {
	validator *validator.Validate
}

// NewLoader creates a new configuration loader
func NewLoader() *Loader {
	return &Loader{
		validator: validator.New(),
	}
}

// Load loads configuration from multiple sources with priority:
// 1. Environment variables
// 2. Configuration files (YAML, JSON, TOML)
// 3. Default values
func Load() (*Config, error) {
	loader := NewLoader()
	return loader.LoadConfig()
}

// LoadConfig loads and validates the configuration
func (l *Loader) LoadConfig() (*Config, error) {
	// Initialize Viper
	v := viper.New()

	// Set configuration file search paths and names
	l.setupViper(v)

	// Load configuration from file
	if err := l.loadFromFile(v); err != nil {
		return nil, fmt.Errorf("failed to load config file: %w", err)
	}

	// Load environment variables
	l.loadFromEnv(v)

	// Unmarshal configuration
	config := GetDefault() // Start with defaults
	if err := v.Unmarshal(config); err != nil {
		return nil, fmt.Errorf("failed to unmarshal config: %w", err)
	}

	// Validate configuration
	if err := l.validateConfig(config); err != nil {
		return nil, fmt.Errorf("config validation failed: %w", err)
	}

	// Post-process configuration
	l.postProcessConfig(config)

	return config, nil
}

// setupViper configures Viper settings
func (l *Loader) setupViper(v *viper.Viper) {
	// Set configuration file name and type
	v.SetConfigName("config")
	v.SetConfigType("yaml")

	// Add configuration file search paths
	v.AddConfigPath(".")
	v.AddConfigPath("./configs/")
	v.AddConfigPath("./config/")
	v.AddConfigPath("/etc/sniper-bot/")
	v.AddConfigPath("$HOME/.sniper-bot/")

	// Enable environment variable reading
	v.AutomaticEnv()

	// Set environment variable prefix
	v.SetEnvPrefix("SNIPER")

	// Replace dots with underscores in environment variable names
	v.SetEnvKeyReplacer(strings.NewReplacer(".", "_"))
}

// loadFromFile loads configuration from file
func (l *Loader) loadFromFile(v *viper.Viper) error {
	// Check if specific config file is set via environment
	if configFile := os.Getenv("SNIPER_CONFIG_FILE"); configFile != "" {
		v.SetConfigFile(configFile)
	} else {
		// Try to load environment-specific config
		env := os.Getenv("SNIPER_ENVIRONMENT")
		if env == "" {
			env = "development"
		}

		// Look for environment-specific config files
		configPaths := []string{
			fmt.Sprintf("config.%s.yaml", env),
			fmt.Sprintf("configs/config.%s.yaml", env),
			"config.yaml",
			"configs/config.yaml",
		}

		var configLoaded bool
		for _, path := range configPaths {
			if _, err := os.Stat(path); err == nil {
				v.SetConfigFile(path)
				configLoaded = true
				break
			}
		}

		if !configLoaded {
			// Create default config file if none exists
			if err := l.createDefaultConfigFile(); err != nil {
				return fmt.Errorf("failed to create default config: %w", err)
			}
			v.SetConfigFile("config.yaml")
		}
	}

	// Read configuration file
	if err := v.ReadInConfig(); err != nil {
		if _, ok := err.(viper.ConfigFileNotFoundError); ok {
			// Config file not found, continue with defaults and env vars
			return nil
		}
		return err
	}

	return nil
}

// loadFromEnv loads configuration from environment variables
func (l *Loader) loadFromEnv(v *viper.Viper) {
	// Bind specific environment variables
	envBindings := map[string]string{
		"SNIPER_ENVIRONMENT":             "environment",
		"SNIPER_DEBUG":                   "debug",
		"SNIPER_SERVER_PORT":             "server.port",
		"SNIPER_SERVER_HOST":             "server.host",
		"SNIPER_DATABASE_HOST":           "database.host",
		"SNIPER_DATABASE_PORT":           "database.port",
		"SNIPER_DATABASE_NAME":           "database.name",
		"SNIPER_DATABASE_USER":           "database.user",
		"SNIPER_DATABASE_PASSWORD":       "database.password",
		"SNIPER_REDIS_HOST":              "redis.host",
		"SNIPER_REDIS_PORT":              "redis.port",
		"SNIPER_REDIS_PASSWORD":          "redis.password",
		"SNIPER_SOLANA_MAINNET_RPC":      "solana.mainnet_rpc",
		"SNIPER_SOLANA_NETWORK":          "solana.network",
		"SNIPER_TELEGRAM_BOT_TOKEN":      "telegram.bot_token",
		"SNIPER_TRADING_MODE":            "trading.mode",
		"SNIPER_TRADING_AUTO_TRADING":    "trading.auto_trading",
		"SNIPER_RISK_LEVEL":              "risk.level",
		"SNIPER_LOG_LEVEL":               "log.level",
		"SNIPER_SECURITY_ENCRYPTION_KEY": "security.encryption_key",
		"SNIPER_SECURITY_JWT_SECRET":     "security.jwt_secret",
	}

	for envVar, configKey := range envBindings {
		v.BindEnv(configKey, envVar)
	}
}

// validateConfig validates the loaded configuration
func (l *Loader) validateConfig(config *Config) error {
	if err := l.validator.Struct(config); err != nil {
		if validationErrors, ok := err.(validator.ValidationErrors); ok {
			var errorMessages []string
			for _, validationError := range validationErrors {
				errorMessages = append(errorMessages, l.formatValidationError(validationError))
			}
			return fmt.Errorf("validation errors: %s", strings.Join(errorMessages, "; "))
		}
		return err
	}

	// Custom validation logic
	if err := l.customValidation(config); err != nil {
		return err
	}

	return nil
}

// formatValidationError formats validation errors into human-readable messages
func (l *Loader) formatValidationError(err validator.FieldError) string {
	field := err.Field()
	tag := err.Tag()
	param := err.Param()

	switch tag {
	case "required":
		return fmt.Sprintf("field '%s' is required", field)
	case "min":
		return fmt.Sprintf("field '%s' must be at least %s", field, param)
	case "max":
		return fmt.Sprintf("field '%s' must be at most %s", field, param)
	case "oneof":
		return fmt.Sprintf("field '%s' must be one of: %s", field, param)
	case "url":
		return fmt.Sprintf("field '%s' must be a valid URL", field)
	default:
		return fmt.Sprintf("field '%s' failed validation '%s'", field, tag)
	}
}

// customValidation performs custom validation logic
func (l *Loader) customValidation(config *Config) error {
	// Validate trading configuration
	if err := l.validateTradingConfig(&config.Trading); err != nil {
		return fmt.Errorf("trading config validation failed: %w", err)
	}

	// Validate risk configuration
	if err := l.validateRiskConfig(&config.Risk); err != nil {
		return fmt.Errorf("risk config validation failed: %w", err)
	}

	// Validate Solana configuration
	if err := l.validateSolanaConfig(&config.Solana); err != nil {
		return fmt.Errorf("solana config validation failed: %w", err)
	}

	// Validate security configuration
	if err := l.validateSecurityConfig(&config.Security); err != nil {
		return fmt.Errorf("security config validation failed: %w", err)
	}

	// Validate performance configuration
	if err := l.validatePerformanceConfig(&config.Performance); err != nil {
		return fmt.Errorf("performance config validation failed: %w", err)
	}

	return nil
}

// validateTradingConfig validates trading-specific configuration
func (l *Loader) validateTradingConfig(config *TradingConfig) error {
	// Validate position sizing
	if config.MaxPositionSize < config.MinPositionSize {
		return fmt.Errorf("max_position_size (%f) must be greater than min_position_size (%f)",
			config.MaxPositionSize, config.MinPositionSize)
	}

	if config.DefaultPositionSize > config.MaxPositionSize {
		return fmt.Errorf("default_position_size (%f) must not exceed max_position_size (%f)",
			config.DefaultPositionSize, config.MaxPositionSize)
	}

	if config.DefaultPositionSize < config.MinPositionSize {
		return fmt.Errorf("default_position_size (%f) must not be less than min_position_size (%f)",
			config.DefaultPositionSize, config.MinPositionSize)
	}

	// Validate slippage settings
	if config.DefaultSlippage > config.MaxSlippage {
		return fmt.Errorf("default_slippage (%f) must not exceed max_slippage (%f)",
			config.DefaultSlippage, config.MaxSlippage)
	}

	// Validate market cap settings
	if config.MaxMarketCap > 0 && config.MinMarketCap > 0 && config.MaxMarketCap < config.MinMarketCap {
		return fmt.Errorf("max_market_cap (%f) must be greater than min_market_cap (%f)",
			config.MaxMarketCap, config.MinMarketCap)
	}

	// Validate DEX configuration
	if len(config.EnabledDEXs) == 0 {
		return fmt.Errorf("at least one DEX must be enabled")
	}

	return nil
}

// validateRiskConfig validates risk management configuration
func (l *Loader) validateRiskConfig(config *RiskConfig) error {
	// Validate loss limits
	if config.MaxDailyLoss > config.MaxWeeklyLoss {
		return fmt.Errorf("max_daily_loss (%f) should not exceed max_weekly_loss (%f)",
			config.MaxDailyLoss, config.MaxWeeklyLoss)
	}

	if config.MaxWeeklyLoss > config.MaxMonthlyLoss {
		return fmt.Errorf("max_weekly_loss (%f) should not exceed max_monthly_loss (%f)",
			config.MaxWeeklyLoss, config.MaxMonthlyLoss)
	}

	// Validate stop loss and take profit
	if config.DefaultStopLoss >= config.DefaultTakeProfit {
		return fmt.Errorf("default_stop_loss (%f) must be less than default_take_profit (%f)",
			config.DefaultStopLoss, config.DefaultTakeProfit)
	}

	// Validate trailing stop
	if config.TrailingStopEnabled && config.TrailingStopPercent <= 0 {
		return fmt.Errorf("trailing_stop_percent must be greater than 0 when trailing stop is enabled")
	}

	// Validate emergency stop loss
	if config.EmergencyStopLoss < config.DefaultStopLoss {
		return fmt.Errorf("emergency_stop_loss (%f) should be greater than default_stop_loss (%f)",
			config.EmergencyStopLoss, config.DefaultStopLoss)
	}

	return nil
}

// validateSolanaConfig validates Solana blockchain configuration
func (l *Loader) validateSolanaConfig(config *SolanaConfig) error {
	// Validate network and RPC endpoint match
	switch config.Network {
	case "mainnet":
		if config.MainnetRPC == "" {
			return fmt.Errorf("mainnet_rpc is required when network is mainnet")
		}
	case "devnet":
		if config.DevnetRPC == "" {
			return fmt.Errorf("devnet_rpc is required when network is devnet")
		}
	case "testnet":
		if config.TestnetRPC == "" {
			return fmt.Errorf("testnet_rpc is required when network is testnet")
		}
	}

	// Validate priority fee settings
	if config.PriorityFee > config.MaxPriorityFee {
		return fmt.Errorf("priority_fee (%d) must not exceed max_priority_fee (%d)",
			config.PriorityFee, config.MaxPriorityFee)
	}

	// Validate Jito configuration
	if config.JitoEnabled {
		if config.JitoEndpoint == "" {
			return fmt.Errorf("jito_endpoint is required when Jito is enabled")
		}
		if config.JitoTipAccount == "" {
			return fmt.Errorf("jito_tip_account is required when Jito is enabled")
		}
	}

	return nil
}

// validateSecurityConfig validates security configuration
func (l *Loader) validateSecurityConfig(config *SecurityConfig) error {
	// Validate encryption key
	if len(config.EncryptionKey) < 32 {
		return fmt.Errorf("encryption_key must be at least 32 characters long")
	}

	// Validate JWT secret
	if len(config.JWTSecret) < 32 {
		return fmt.Errorf("jwt_secret must be at least 32 characters long")
	}

	// Validate rate limiting
	if config.RateLimitEnabled {
		if config.RateLimitRequests <= 0 {
			return fmt.Errorf("rate_limit_requests must be greater than 0 when rate limiting is enabled")
		}
		if config.RateLimitWindow <= 0 {
			return fmt.Errorf("rate_limit_window must be greater than 0 when rate limiting is enabled")
		}
	}

	return nil
}

// validatePerformanceConfig validates performance configuration
func (l *Loader) validatePerformanceConfig(config *PerformanceConfig) error {
	// Validate worker counts
	if config.ScannerWorkers > 100 {
		return fmt.Errorf("scanner_workers (%d) exceeds recommended maximum of 100", config.ScannerWorkers)
	}

	if config.ExecutorWorkers > 50 {
		return fmt.Errorf("executor_workers (%d) exceeds recommended maximum of 50", config.ExecutorWorkers)
	}

	// Validate queue sizes
	if config.ScannerQueueSize < config.ScannerWorkers*10 {
		return fmt.Errorf("scanner_queue_size (%d) should be at least 10x scanner_workers (%d)",
			config.ScannerQueueSize, config.ScannerWorkers)
	}

	if config.ExecutorQueueSize < config.ExecutorWorkers*10 {
		return fmt.Errorf("executor_queue_size (%d) should be at least 10x executor_workers (%d)",
			config.ExecutorQueueSize, config.ExecutorWorkers)
	}

	// Validate rate limits
	if config.RPCRateLimit <= 0 {
		return fmt.Errorf("rpc_rate_limit must be greater than 0")
	}

	return nil
}

// postProcessConfig performs post-processing on the configuration
func (l *Loader) postProcessConfig(config *Config) {
	// Set derived values
	l.setDerivedValues(config)

	// Apply environment-specific adjustments
	l.applyEnvironmentAdjustments(config)

	// Optimize performance settings
	l.optimizePerformanceSettings(config)
}

// setDerivedValues sets configuration values derived from others
func (l *Loader) setDerivedValues(config *Config) {
	// Set database connection string if not explicitly configured
	if config.Database.Host != "" && config.Database.Name != "" {
		// This will be used in Phase 2 for actual database connection
	}

	// Set Redis connection string
	if config.Redis.Host != "" && config.Redis.Port > 0 {
		// This will be used in Phase 2 for Redis connection
	}

	// Adjust performance settings based on trading mode
	switch config.Trading.Mode {
	case "full-auto":
		// Increase workers for fully automated trading
		config.Performance.ScannerWorkers = maxInt(config.Performance.ScannerWorkers, 10)
		config.Performance.ExecutorWorkers = maxInt(config.Performance.ExecutorWorkers, 5)
	case "semi-auto":
		// Moderate settings for semi-automated trading
		config.Performance.ScannerWorkers = maxInt(config.Performance.ScannerWorkers, 5)
		config.Performance.ExecutorWorkers = maxInt(config.Performance.ExecutorWorkers, 3)
	case "manual":
		// Conservative settings for manual trading
		config.Performance.ScannerWorkers = maxInt(config.Performance.ScannerWorkers, 3)
		config.Performance.ExecutorWorkers = maxInt(config.Performance.ExecutorWorkers, 2)
	}
}

// applyEnvironmentAdjustments applies environment-specific configuration adjustments
func (l *Loader) applyEnvironmentAdjustments(config *Config) {
	switch config.Environment {
	case "production":
		// Production optimizations
		config.Log.Level = "info"
		config.Performance.ProfilingEnabled = false
		config.Security.SecurityHeadersEnabled = true
		config.Security.HSTSEnabled = true

		// Increase connection pools for production
		config.Database.MaxOpenConns = maxInt(config.Database.MaxOpenConns, 20)
		config.Redis.PoolSize = maxInt(config.Redis.PoolSize, 20)

	case "staging":
		// Staging optimizations
		config.Log.Level = "debug"
		config.Performance.ProfilingEnabled = true

	case "development":
		// Development optimizations
		config.Log.Level = "debug"
		config.Log.EnableColors = true
		config.Performance.ProfilingEnabled = true
		config.Security.CORSOrigins = []string{"*"}

		// Enable dry run by default in development
		config.Trading.DryRun = true
	}
}

// optimizePerformanceSettings optimizes performance settings based on system capabilities
func (l *Loader) optimizePerformanceSettings(config *Config) {
	// These optimizations will be enhanced in later phases when we have
	// better understanding of system resources and requirements

	// Ensure minimum viable settings
	config.Performance.ScannerWorkers = maxInt(config.Performance.ScannerWorkers, 1)
	config.Performance.ExecutorWorkers = maxInt(config.Performance.ExecutorWorkers, 1)
	config.Performance.AnalyzerWorkers = maxInt(config.Performance.AnalyzerWorkers, 1)

	// Ensure queue sizes are reasonable
	config.Performance.ScannerQueueSize = maxInt(config.Performance.ScannerQueueSize, 100)
	config.Performance.ExecutorQueueSize = maxInt(config.Performance.ExecutorQueueSize, 50)
	config.Performance.AnalyzerQueueSize = maxInt(config.Performance.AnalyzerQueueSize, 100)
}

// createDefaultConfigFile creates a default configuration file
func (l *Loader) createDefaultConfigFile() error {
	configDir := "configs"
	configFile := filepath.Join(configDir, "config.yaml")

	// Create configs directory if it doesn't exist
	if err := os.MkdirAll(configDir, 0755); err != nil {
		return fmt.Errorf("failed to create config directory: %w", err)
	}

	// Check if config file already exists
	if _, err := os.Stat(configFile); err == nil {
		return nil // File already exists
	}

	// Create default configuration content
	defaultConfig := `# Solana Sniper Bot Configuration
# This is the main configuration file for the Solana Sniper Bot
# For production use, copy this file and modify the values according to your needs

# Application Environment
environment: "development"
debug: true

# HTTP Server Configuration
server:
 port: 8080
 host: "0.0.0.0"
 read_timeout: "30s"
 write_timeout: "30s"
 idle_timeout: "60s"
 max_header_bytes: 1048576

# Database Configuration (PostgreSQL)
database:
 host: "localhost"
 port: 5432
 name: "sniper_bot"
 user: "postgres"
 password: "password"
 ssl_mode: "disable"
 max_open_conns: 10
 max_idle_conns: 5
 conn_max_lifetime: "1h"
 conn_max_idle_time: "30m"

# Redis Configuration
redis:
 host: "localhost"
 port: 6379
 password: ""
 db: 0
 max_retries: 3
 pool_size: 10
 dial_timeout: "5s"
 read_timeout: "3s"
 write_timeout: "3s"
 idle_timeout: "5m"

# Solana Blockchain Configuration
solana:
 mainnet_rpc: "https://api.mainnet-beta.solana.com"
 devnet_rpc: "https://api.devnet.solana.com"
 network: "mainnet"
 commitment: "confirmed"
 max_retries: 3
 request_timeout: "10s"
 connection_pool: 10
 ws_reconnect: true
 ws_timeout: "30s"
 ws_ping_interval: "30s"
 priority_fee: 10000
 max_priority_fee: 100000
 jito_enabled: false

# Trading Configuration
trading:
 mode: "manual"  # manual, semi-auto, full-auto
 auto_trading: false
 dry_run: true
 max_positions: 5
 max_concurrent_trades: 3
 default_position_size: 0.1
 max_position_size: 1.0
 min_position_size: 0.01
 default_slippage: 3.0
 max_slippage: 10.0
 max_fee: 1.0
 min_liquidity: 1.0
 min_holders: 10
 max_token_age: "60s"
 max_hold_time: "24h"
 scan_interval: "100ms"
 execution_timeout: "5s"
 retry_delay: "1s"
 max_retries: 3
 enabled_dexs:
   - "raydium"
   - "pumpfun"

# Risk Management Configuration
risk:
 level: "conservative"  # conservative, moderate, aggressive
 max_daily_loss: 10.0
 max_weekly_loss: 20.0
 max_monthly_loss: 30.0
 max_drawdown: 20.0
 position_size_method: "fixed"  # fixed, percentage, kelly
 risk_per_trade: 2.0
 max_risk_per_token: 10.0
 default_stop_loss: 20.0
 default_take_profit: 50.0
 trailing_stop_enabled: true
 trailing_stop_percent: 5.0
 max_tokens_per_sector: 3
 max_allocation_per_token: 20.0
 diversification_limit: 5
 emergency_stop_loss: 50.0
 panic_sell_enabled: true
 circuit_breaker_enabled: true
 circuit_breaker_threshold: 15.0
 honeypot_check_enabled: true
 contract_verification: true
 min_liquidity_lock_time: "24h"

# Telegram Bot Configuration
telegram:
 bot_token: ""  # Set via environment variable SNIPER_TELEGRAM_BOT_TOKEN
 notifications_enabled: true
 trade_alerts: true
 profit_alerts: true
 loss_alerts: true
 risk_alerts: true
 system_alerts: true
 delete_messages: false
 message_lifetime: "24h"
 command_cooldown: "1s"
 max_messages_per_min: 30
 language: "en"
 timezone: "UTC"
 currency_display: "USD"
 decimal_places: 4

# Logging Configuration
log:
 level: "info"  # debug, info, warn, error, fatal
 format: "json"  # json, text
 output: "stdout"  # stdout, stderr, file
 max_size: 100
 max_age: 30
 max_backups: 10
 compress: true
 enable_colors: true
 enable_caller: true
 enable_stacktrace: false
 slow_query_threshold: "500ms"
 log_queries: false
 log_requests: true

# Performance Configuration
performance:
 scanner_workers: 10
 executor_workers: 5
 analyzer_workers: 3
 scanner_queue_size: 1000
 executor_queue_size: 500
 analyzer_queue_size: 300
 scan_interval: "100ms"
 analysis_timeout: "5s"
 execution_timeout: "10s"
 cleanup_interval: "1h"
 cache_enabled: true
 cache_size: 1000
 cache_ttl: "5m"
 token_cache_ttl: "1m"
 price_cache_ttl: "30s"
 rpc_rate_limit: 100
 api_rate_limit: 50
 telegram_rate_limit: 20
 max_memory_usage: 536870912  # 512 MB
 gc_interval: "30s"
 gc_target_percent: 100
 metrics_enabled: true
 metrics_port: 9090
 health_check_port: 8081
 profiling_enabled: false
 profiling_port: 6060

# Security Configuration
security:
 encryption_key: ""  # Set via environment variable SNIPER_SECURITY_ENCRYPTION_KEY
 encryption_algorithm: "AES-256-GCM"
 jwt_secret: ""  # Set via environment variable SNIPER_SECURITY_JWT_SECRET
 jwt_expiration: "24h"
 jwt_refresh_expiration: "168h"  # 7 days
 rate_limit_enabled: true
 rate_limit_requests: 100
 rate_limit_window: "1m"
 cors_enabled: true
 cors_origins:
   - "*"
 cors_methods:
   - "GET"
   - "POST"
   - "PUT"
   - "DELETE"
   - "OPTIONS"
 cors_headers:
   - "Origin"
   - "Content-Type"
   - "Accept"
   - "Authorization"
 security_headers_enabled: true
 hsts_enabled: true
 hsts_max_age: 31536000
 wallet_encryption: true
 wallet_backup_enabled: true
 wallet_backup_interval: "24h"
 audit_log_enabled: true
 audit_log_rotation: true
 ip_whitelist_enabled: false
 ip_blacklist_enabled: false
`

	// Write default configuration to file
	if err := os.WriteFile(configFile, []byte(defaultConfig), 0644); err != nil {
		return fmt.Errorf("failed to write default config file: %w", err)
	}

	return nil
}

// Helper functions

// maxInt returns the maximum of two integers
func maxInt(a, b int) int {
	if a > b {
		return a
	}
	return b
}

// GetConfigPath returns the path to the configuration file
func GetConfigPath() string {
	if configFile := os.Getenv("SNIPER_CONFIG_FILE"); configFile != "" {
		return configFile
	}

	env := os.Getenv("SNIPER_ENVIRONMENT")
	if env == "" {
		env = "development"
	}

	return fmt.Sprintf("configs/config.%s.yaml", env)
}

// ReloadConfig reloads the configuration from file
func ReloadConfig() (*Config, error) {
	return Load()
}

// ValidateConfigFile validates a configuration file without loading it
func ValidateConfigFile(filepath string) error {
	v := viper.New()
	v.SetConfigFile(filepath)

	if err := v.ReadInConfig(); err != nil {
		return fmt.Errorf("failed to read config file: %w", err)
	}

	config := &Config{}
	if err := v.Unmarshal(config); err != nil {
		return fmt.Errorf("failed to unmarshal config: %w", err)
	}

	loader := NewLoader()
	return loader.validateConfig(config)
}
