// Package models defines the data models for the Solana Sniper Bot
// This file contains the User model and related structures for user management,
// authentication, and trading preferences.
package models

import (
	"database/sql/driver"
	"encoding/json"
	"fmt"
	"time"

	"gorm.io/gorm"
)

// User represents a user account in the system
type User struct {
	BaseModel

	// Identity and Authentication
	TelegramID   int64  `gorm:"uniqueIndex;not null" json:"telegram_id"`
	Username     string `gorm:"index" json:"username,omitempty"`
	FirstName    string `json:"first_name,omitempty"`
	LastName     string `json:"last_name,omitempty"`
	LanguageCode string `gorm:"size:10" json:"language_code,omitempty"`

	// Account Status
	Status     UserStatus `gorm:"type:varchar(20);default:'active'" json:"status"`
	Role       UserRole   `gorm:"type:varchar(20);default:'user'" json:"role"`
	IsActive   bool       `gorm:"default:true" json:"is_active"`
	IsPremium  bool       `gorm:"default:false" json:"is_premium"`
	IsVerified bool       `gorm:"default:false" json:"is_verified"`

	// Subscription and Limits
	SubscriptionType   SubscriptionType `gorm:"type:varchar(20);default:'free'" json:"subscription_type"`
	SubscriptionExpiry *time.Time       `json:"subscription_expiry,omitempty"`
	DailyTradeLimit    int              `gorm:"default:10" json:"daily_trade_limit"`
	MonthlyTradeLimit  int              `gorm:"default:100" json:"monthly_trade_limit"`

	// Trading Preferences
	TradingPreferences   TradingPreferences   `gorm:"type:json" json:"trading_preferences"`
	RiskPreferences      RiskPreferences      `gorm:"type:json" json:"risk_preferences"`
	NotificationSettings NotificationSettings `gorm:"type:json" json:"notification_settings"`

	// Wallet Information
	WalletAddress   string       `gorm:"index" json:"wallet_address,omitempty"`
	EncryptedWallet string       `json:"-"` // Never expose in JSON
	WalletStatus    WalletStatus `gorm:"type:varchar(20);default:'not_connected'" json:"wallet_status"`
	WalletType      WalletType   `gorm:"type:varchar(20)" json:"wallet_type,omitempty"`

	// Statistics and Performance
	TotalTrades      int64   `gorm:"default:0" json:"total_trades"`
	SuccessfulTrades int64   `gorm:"default:0" json:"successful_trades"`
	TotalVolume      float64 `gorm:"type:decimal(20,8);default:0" json:"total_volume"`
	TotalProfit      float64 `gorm:"type:decimal(20,8);default:0" json:"total_profit"`
	TotalLoss        float64 `gorm:"type:decimal(20,8);default:0" json:"total_loss"`
	WinRate          float64 `gorm:"type:decimal(5,2);default:0" json:"win_rate"`
	AverageProfit    float64 `gorm:"type:decimal(20,8);default:0" json:"average_profit"`
	MaxDrawdown      float64 `gorm:"type:decimal(5,2);default:0" json:"max_drawdown"`

	// Session and Security
	LastLogin        *time.Time `json:"last_login,omitempty"`
	LastActivity     *time.Time `json:"last_activity,omitempty"`
	SessionToken     string     `json:"-"` // Never expose in JSON
	RefreshToken     string     `json:"-"` // Never expose in JSON
	TokenExpiry      *time.Time `json:"-"` // Never expose in JSON
	TwoFactorEnabled bool       `gorm:"default:false" json:"two_factor_enabled"`
	TwoFactorSecret  string     `json:"-"` // Never expose in JSON

	// Referral System
	ReferralCode     string  `gorm:"uniqueIndex" json:"referral_code,omitempty"`
	ReferredBy       *uint   `json:"referred_by,omitempty"`
	ReferredByUser   *User   `gorm:"foreignKey:ReferredBy" json:"referred_by_user,omitempty"`
	Referrals        []User  `gorm:"foreignKey:ReferredBy" json:"referrals,omitempty"`
	TotalReferrals   int     `gorm:"default:0" json:"total_referrals"`
	ReferralEarnings float64 `gorm:"type:decimal(20,8);default:0" json:"referral_earnings"`

	// Metadata
	IPAddress    string            `json:"-"`                  // Never expose in JSON
	UserAgent    string            `json:"-"`                  // Never expose in JSON
	DeviceInfo   map[string]string `gorm:"type:json" json:"-"` // Never expose in JSON
	LoginHistory []LoginRecord     `gorm:"type:json" json:"-"` // Never expose in JSON

	// Relationships
	Trades        []Trade        `gorm:"foreignKey:UserID" json:"trades,omitempty"`
	Positions     []Position     `gorm:"foreignKey:UserID" json:"positions,omitempty"`
	Notifications []Notification `gorm:"foreignKey:UserID" json:"notifications,omitempty"`
	APIKeys       []APIKey       `gorm:"foreignKey:UserID" json:"api_keys,omitempty"`
}

// UserStatus represents the status of a user account
type UserStatus string

const (
	UserStatusActive    UserStatus = "active"
	UserStatusInactive  UserStatus = "inactive"
	UserStatusSuspended UserStatus = "suspended"
	UserStatusBanned    UserStatus = "banned"
	UserStatusPending   UserStatus = "pending"
)

// UserRole represents the role of a user
type UserRole string

const (
	UserRoleUser      UserRole = "user"
	UserRoleAdmin     UserRole = "admin"
	UserRoleModerator UserRole = "moderator"
	UserRoleVIP       UserRole = "vip"
)

// SubscriptionType represents the subscription tier
type SubscriptionType string

const (
	SubscriptionFree       SubscriptionType = "free"
	SubscriptionBasic      SubscriptionType = "basic"
	SubscriptionPremium    SubscriptionType = "premium"
	SubscriptionPro        SubscriptionType = "pro"
	SubscriptionEnterprise SubscriptionType = "enterprise"
)

// WalletStatus represents the status of user's wallet
type WalletStatus string

const (
	WalletStatusNotConnected WalletStatus = "not_connected"
	WalletStatusConnected    WalletStatus = "connected"
	WalletStatusEncrypted    WalletStatus = "encrypted"
	WalletStatusLocked       WalletStatus = "locked"
	WalletStatusError        WalletStatus = "error"
)

// WalletType represents the type of wallet
type WalletType string

const (
	WalletTypePhantom    WalletType = "phantom"
	WalletTypeSolflare   WalletType = "solflare"
	WalletTypeBackpack   WalletType = "backpack"
	WalletTypePrivateKey WalletType = "private_key"
	WalletTypeLedger     WalletType = "ledger"
)

// TradingPreferences holds user's trading preferences
type TradingPreferences struct {
	AutoTrading         bool                   `json:"auto_trading"`
	TradingMode         string                 `json:"trading_mode"`
	DefaultSlippage     float64                `json:"default_slippage"`
	MaxSlippage         float64                `json:"max_slippage"`
	DefaultPositionSize float64                `json:"default_position_size"`
	MaxPositionSize     float64                `json:"max_position_size"`
	MinLiquidity        float64                `json:"min_liquidity"`
	MaxMarketCap        float64                `json:"max_market_cap"`
	EnabledDEXs         []string               `json:"enabled_dexs"`
	PreferredTokens     []string               `json:"preferred_tokens"`
	BlacklistedTokens   []string               `json:"blacklisted_tokens"`
	TradingHours        TradingHours           `json:"trading_hours"`
	AdvancedSettings    map[string]interface{} `json:"advanced_settings"`
}

// RiskPreferences holds user's risk management preferences
type RiskPreferences struct {
	RiskLevel            string  `json:"risk_level"`
	MaxDailyLoss         float64 `json:"max_daily_loss"`
	MaxDrawdown          float64 `json:"max_drawdown"`
	StopLossPercentage   float64 `json:"stop_loss_percentage"`
	TakeProfitPercentage float64 `json:"take_profit_percentage"`
	PositionSizeMethod   string  `json:"position_size_method"`
	MaxConcurrentTrades  int     `json:"max_concurrent_trades"`
	DiversificationLimit int     `json:"diversification_limit"`
	HoneypotProtection   bool    `json:"honeypot_protection"`
	ContractVerification bool    `json:"contract_verification"`
	EmergencyStopEnabled bool    `json:"emergency_stop_enabled"`
}

// NotificationSettings holds user's notification preferences
type NotificationSettings struct {
	Enabled            bool `json:"enabled"`
	TradeExecutions    bool `json:"trade_executions"`
	ProfitAlerts       bool `json:"profit_alerts"`
	LossAlerts         bool `json:"loss_alerts"`
	RiskWarnings       bool `json:"risk_warnings"`
	SystemAlerts       bool `json:"system_alerts"`
	MarketUpdates      bool `json:"market_updates"`
	NewTokenAlerts     bool `json:"new_token_alerts"`
	PriceAlerts        bool `json:"price_alerts"`
	PerformanceReports bool `json:"performance_reports"`
	MaintenanceAlerts  bool `json:"maintenance_alerts"`
	SecurityAlerts     bool `json:"security_alerts"`

	// Notification Timing
	QuietHoursEnabled bool   `json:"quiet_hours_enabled"`
	QuietHoursStart   string `json:"quiet_hours_start"` // Format: "22:00"
	QuietHoursEnd     string `json:"quiet_hours_end"`   // Format: "08:00"
	Timezone          string `json:"timezone"`

	// Notification Thresholds
	MinProfitAlert   float64 `json:"min_profit_alert"`
	MinLossAlert     float64 `json:"min_loss_alert"`
	PriceChangeAlert float64 `json:"price_change_alert"`

	// Channel Preferences
	TelegramEnabled bool `json:"telegram_enabled"`
	EmailEnabled    bool `json:"email_enabled"`
	PushEnabled     bool `json:"push_enabled"`
	SMSEnabled      bool `json:"sms_enabled"`
}

// TradingHours defines when the user wants to trade
type TradingHours struct {
	Enabled     bool                   `json:"enabled"`
	Timezone    string                 `json:"timezone"`
	WeeklyHours map[string]DaySchedule `json:"weekly_hours"`
}

// DaySchedule defines trading hours for a specific day
type DaySchedule struct {
	Enabled   bool   `json:"enabled"`
	StartTime string `json:"start_time"` // Format: "09:00"
	EndTime   string `json:"end_time"`   // Format: "17:00"
}

// LoginRecord represents a login event
type LoginRecord struct {
	Timestamp time.Time `json:"timestamp"`
	IPAddress string    `json:"ip_address"`
	UserAgent string    `json:"user_agent"`
	Location  string    `json:"location,omitempty"`
	Success   bool      `json:"success"`
}

// APIKey represents an API key for programmatic access
type APIKey struct {
	BaseModel
	UserID      uint       `gorm:"not null;index" json:"user_id"`
	Name        string     `gorm:"not null" json:"name"`
	KeyHash     string     `gorm:"uniqueIndex;not null" json:"-"` // Never expose in JSON
	Permissions []string   `gorm:"type:json" json:"permissions"`
	IsActive    bool       `gorm:"default:true" json:"is_active"`
	LastUsed    *time.Time `json:"last_used,omitempty"`
	ExpiresAt   *time.Time `json:"expires_at,omitempty"`
	RateLimit   int        `gorm:"default:1000" json:"rate_limit"` // Requests per hour
	IPWhitelist []string   `gorm:"type:json" json:"ip_whitelist,omitempty"`
}

// GORM Hooks

// BeforeCreate is called before creating a user
func (u *User) BeforeCreate(tx *gorm.DB) error {
	if u.ReferralCode == "" {
		u.ReferralCode = generateReferralCode()
	}

	// Set default preferences if not provided
	if u.TradingPreferences.DefaultSlippage == 0 {
		u.TradingPreferences = getDefaultTradingPreferences()
	}

	if u.RiskPreferences.RiskLevel == "" {
		u.RiskPreferences = getDefaultRiskPreferences()
	}

	if !u.NotificationSettings.Enabled {
		u.NotificationSettings = getDefaultNotificationSettings()
	}

	return nil
}

// AfterCreate is called after creating a user
func (u *User) AfterCreate(tx *gorm.DB) error {
	// Create default settings, analytics records, etc.
	// This will be implemented in later phases
	return nil
}

// BeforeUpdate is called before updating a user
func (u *User) BeforeUpdate(tx *gorm.DB) error {
	// Update calculated fields
	if u.TotalTrades > 0 {
		u.WinRate = (float64(u.SuccessfulTrades) / float64(u.TotalTrades)) * 100
	}

	if u.SuccessfulTrades > 0 {
		u.AverageProfit = u.TotalProfit / float64(u.SuccessfulTrades)
	}

	return nil
}

// User Methods

// IsSubscriptionValid checks if user's subscription is still valid
func (u *User) IsSubscriptionValid() bool {
	if u.SubscriptionType == SubscriptionFree {
		return true
	}

	if u.SubscriptionExpiry == nil {
		return false
	}

	return u.SubscriptionExpiry.After(time.Now())
}

// CanTrade checks if user can execute trades based on limits and status
func (u *User) CanTrade() bool {
	if !u.IsActive || u.Status != UserStatusActive {
		return false
	}

	if u.WalletStatus != WalletStatusConnected && u.WalletStatus != WalletStatusEncrypted {
		return false
	}

	// Check subscription validity
	if !u.IsSubscriptionValid() && u.SubscriptionType != SubscriptionFree {
		return false
	}

	// Additional checks will be added in later phases
	return true
}

// GetDailyTradeCount returns the number of trades executed today
func (u *User) GetDailyTradeCount(tx *gorm.DB) (int64, error) {
	today := time.Now().Truncate(24 * time.Hour)
	var count int64

	err := tx.Model(&Trade{}).
		Where("user_id = ? AND created_at >= ?", u.ID, today).
		Count(&count).Error

	return count, err
}

// GetMonthlyTradeCount returns the number of trades executed this month
func (u *User) GetMonthlyTradeCount(tx *gorm.DB) (int64, error) {
	now := time.Now()
	startOfMonth := time.Date(now.Year(), now.Month(), 1, 0, 0, 0, 0, now.Location())
	var count int64

	err := tx.Model(&Trade{}).
		Where("user_id = ? AND created_at >= ?", u.ID, startOfMonth).
		Count(&count).Error

	return count, err
}

// CanExecuteTrade checks if user can execute a new trade
func (u *User) CanExecuteTrade(tx *gorm.DB) (bool, string) {
	if !u.CanTrade() {
		return false, "User cannot trade due to account status or wallet connection"
	}

	// Check daily limit
	dailyCount, err := u.GetDailyTradeCount(tx)
	if err != nil {
		return false, "Failed to check daily trade count"
	}

	if dailyCount >= int64(u.DailyTradeLimit) {
		return false, "Daily trade limit exceeded"
	}

	// Check monthly limit
	monthlyCount, err := u.GetMonthlyTradeCount(tx)
	if err != nil {
		return false, "Failed to check monthly trade count"
	}

	if monthlyCount >= int64(u.MonthlyTradeLimit) {
		return false, "Monthly trade limit exceeded"
	}

	return true, ""
}

// UpdateLastActivity updates the user's last activity timestamp
func (u *User) UpdateLastActivity(tx *gorm.DB) error {
	now := time.Now()
	return tx.Model(u).Update("last_activity", now).Error
}

// UpdateLoginInfo updates login-related information
func (u *User) UpdateLoginInfo(tx *gorm.DB, ipAddress, userAgent string) error {
	now := time.Now()

	// Add to login history
	loginRecord := LoginRecord{
		Timestamp: now,
		IPAddress: ipAddress,
		UserAgent: userAgent,
		Success:   true,
	}

	// Append to existing login history (keep last 10 records)
	if len(u.LoginHistory) >= 10 {
		u.LoginHistory = u.LoginHistory[1:]
	}
	u.LoginHistory = append(u.LoginHistory, loginRecord)

	// Update last login and activity
	updates := map[string]interface{}{
		"last_login":    now,
		"last_activity": now,
		"ip_address":    ipAddress,
		"user_agent":    userAgent,
		"login_history": u.LoginHistory,
	}

	return tx.Model(u).Updates(updates).Error
}

// GetProfitLoss calculates current profit/loss
func (u *User) GetProfitLoss() float64 {
	return u.TotalProfit - u.TotalLoss
}

// GetROI calculates return on investment
func (u *User) GetROI() float64 {
	if u.TotalVolume == 0 {
		return 0
	}
	return (u.GetProfitLoss() / u.TotalVolume) * 100
}

// Custom JSON marshaling for sensitive fields
func (u *User) MarshalJSON() ([]byte, error) {
	type UserAlias User

	return json.Marshal(&struct {
		*UserAlias
		EncryptedWallet string `json:"-"`
		SessionToken    string `json:"-"`
		RefreshToken    string `json:"-"`
		TokenExpiry     string `json:"-"`
		TwoFactorSecret string `json:"-"`
		IPAddress       string `json:"-"`
		UserAgent       string `json:"-"`
		DeviceInfo      string `json:"-"`
		LoginHistory    string `json:"-"`
	}{
		UserAlias: (*UserAlias)(u),
	})
}

// Custom implementations for JSON fields

// Scan implements the sql.Scanner interface for TradingPreferences
func (tp *TradingPreferences) Scan(value interface{}) error {
	if value == nil {
		*tp = getDefaultTradingPreferences()
		return nil
	}

	bytes, ok := value.([]byte)
	if !ok {
		return fmt.Errorf("cannot scan %T into TradingPreferences", value)
	}

	return json.Unmarshal(bytes, tp)
}

// Value implements the driver.Valuer interface for TradingPreferences
func (tp TradingPreferences) Value() (driver.Value, error) {
	return json.Marshal(tp)
}

// Scan implements the sql.Scanner interface for RiskPreferences
func (rp *RiskPreferences) Scan(value interface{}) error {
	if value == nil {
		*rp = getDefaultRiskPreferences()
		return nil
	}

	bytes, ok := value.([]byte)
	if !ok {
		return fmt.Errorf("cannot scan %T into RiskPreferences", value)
	}

	return json.Unmarshal(bytes, rp)
}

// Value implements the driver.Valuer interface for RiskPreferences
func (rp RiskPreferences) Value() (driver.Value, error) {
	return json.Marshal(rp)
}

// Scan implements the sql.Scanner interface for NotificationSettings
func (ns *NotificationSettings) Scan(value interface{}) error {
	if value == nil {
		*ns = getDefaultNotificationSettings()
		return nil
	}

	bytes, ok := value.([]byte)
	if !ok {
		return fmt.Errorf("cannot scan %T into NotificationSettings", value)
	}

	return json.Unmarshal(bytes, ns)
}

// Value implements the driver.Valuer interface for NotificationSettings
func (ns NotificationSettings) Value() (driver.Value, error) {
	return json.Marshal(ns)
}

// Helper functions

// generateReferralCode generates a unique referral code
func generateReferralCode() string {
	// This will be implemented with proper crypto/rand in later phases
	return fmt.Sprintf("REF%d", time.Now().Unix())
}

// getDefaultTradingPreferences returns default trading preferences
func getDefaultTradingPreferences() TradingPreferences {
	return TradingPreferences{
		AutoTrading:         false,
		TradingMode:         "manual",
		DefaultSlippage:     3.0,
		MaxSlippage:         10.0,
		DefaultPositionSize: 0.1,
		MaxPositionSize:     1.0,
		MinLiquidity:        1.0,
		MaxMarketCap:        100000,
		EnabledDEXs:         []string{"raydium", "pumpfun"},
		PreferredTokens:     []string{},
		BlacklistedTokens:   []string{},
		TradingHours: TradingHours{
			Enabled:  false,
			Timezone: "UTC",
			WeeklyHours: map[string]DaySchedule{
				"monday":    {Enabled: true, StartTime: "09:00", EndTime: "17:00"},
				"tuesday":   {Enabled: true, StartTime: "09:00", EndTime: "17:00"},
				"wednesday": {Enabled: true, StartTime: "09:00", EndTime: "17:00"},
				"thursday":  {Enabled: true, StartTime: "09:00", EndTime: "17:00"},
				"friday":    {Enabled: true, StartTime: "09:00", EndTime: "17:00"},
				"saturday":  {Enabled: false, StartTime: "09:00", EndTime: "17:00"},
				"sunday":    {Enabled: false, StartTime: "09:00", EndTime: "17:00"},
			},
		},
		AdvancedSettings: make(map[string]interface{}),
	}
}

// getDefaultRiskPreferences returns default risk preferences
func getDefaultRiskPreferences() RiskPreferences {
	return RiskPreferences{
		RiskLevel:            "conservative",
		MaxDailyLoss:         10.0,
		MaxDrawdown:          20.0,
		StopLossPercentage:   20.0,
		TakeProfitPercentage: 50.0,
		PositionSizeMethod:   "fixed",
		MaxConcurrentTrades:  3,
		DiversificationLimit: 5,
		HoneypotProtection:   true,
		ContractVerification: true,
		EmergencyStopEnabled: true,
	}
}

// getDefaultNotificationSettings returns default notification settings
func getDefaultNotificationSettings() NotificationSettings {
	return NotificationSettings{
		Enabled:            true,
		TradeExecutions:    true,
		ProfitAlerts:       true,
		LossAlerts:         true,
		RiskWarnings:       true,
		SystemAlerts:       true,
		MarketUpdates:      false,
		NewTokenAlerts:     false,
		PriceAlerts:        false,
		PerformanceReports: true,
		MaintenanceAlerts:  true,
		SecurityAlerts:     true,
		QuietHoursEnabled:  false,
		QuietHoursStart:    "22:00",
		QuietHoursEnd:      "08:00",
		Timezone:           "UTC",
		MinProfitAlert:     10.0,
		MinLossAlert:       5.0,
		PriceChangeAlert:   20.0,
		TelegramEnabled:    true,
		EmailEnabled:       false,
		PushEnabled:        false,
		SMSEnabled:         false,
	}
}

// UserRepository interface (will be implemented in Phase 2)
type UserRepository interface {
	Create(user *User) error
	GetByID(id uint) (*User, error)
	GetByTelegramID(telegramID int64) (*User, error)
	GetByReferralCode(code string) (*User, error)
	Update(user *User) error
	Delete(id uint) error
	List(limit, offset int) ([]*User, error)
	GetActiveUsers() ([]*User, error)
	GetUsersBySubscription(subscriptionType SubscriptionType) ([]*User, error)
}

// UserService interface (will be implemented in later phases)
type UserService interface {
	RegisterUser(telegramID int64, username, firstName, lastName string) (*User, error)
	AuthenticateUser(telegramID int64) (*User, error)
	UpdateUserPreferences(userID uint, preferences *TradingPreferences) error
	UpdateRiskPreferences(userID uint, preferences *RiskPreferences) error
	UpdateNotificationSettings(userID uint, settings *NotificationSettings) error
	ConnectWallet(userID uint, walletAddress string, walletType WalletType) error
	DisconnectWallet(userID uint) error
	UpgradeSubscription(userID uint, subscriptionType SubscriptionType, expiryDate time.Time) error
	ProcessReferral(referralCode string, newUserID uint) error
	GetUserStatistics(userID uint) (*UserStatistics, error)
}

// UserStatistics represents aggregated user statistics
type UserStatistics struct {
	TotalTrades          int64      `json:"total_trades"`
	SuccessfulTrades     int64      `json:"successful_trades"`
	WinRate              float64    `json:"win_rate"`
	TotalVolume          float64    `json:"total_volume"`
	NetProfit            float64    `json:"net_profit"`
	AverageProfit        float64    `json:"average_profit"`
	MaxDrawdown          float64    `json:"max_drawdown"`
	ROI                  float64    `json:"roi"`
	TradingDays          int        `json:"trading_days"`
	AverageDailyTrades   float64    `json:"average_daily_trades"`
	BestPerformingToken  string     `json:"best_performing_token,omitempty"`
	WorstPerformingToken string     `json:"worst_performing_token,omitempty"`
	TotalReferrals       int        `json:"total_referrals"`
	ReferralEarnings     float64    `json:"referral_earnings"`
	LastTradeDate        *time.Time `json:"last_trade_date,omitempty"`
	MemberSince          time.Time  `json:"member_since"`
}
