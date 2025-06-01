// Package models - Token model and related structures
package models

import (
	"database/sql/driver"
	"encoding/json"
	"fmt"
	"time"

	"gorm.io/gorm"
)

// Token represents a Solana token with comprehensive information
type Token struct {
	BaseModel

	// Basic Token Information
	Address  string  `gorm:"uniqueIndex;not null" json:"address"`
	Symbol   string  `gorm:"index" json:"symbol"`
	Name     string  `json:"name"`
	Decimals int     `gorm:"default:9" json:"decimals"`
	Supply   float64 `gorm:"type:decimal(40,18)" json:"supply"`

	// Token Status and Classification
	Status   TokenStatus   `gorm:"type:varchar(20);default:'active'" json:"status"`
	Type     TokenType     `gorm:"type:varchar(30);default:'unknown'" json:"type"`
	Category TokenCategory `gorm:"type:varchar(50)" json:"category,omitempty"`
	Tags     []string      `gorm:"type:json" json:"tags,omitempty"`

	// Market Data
	CurrentPrice   float64 `gorm:"type:decimal(30,18)" json:"current_price"`
	MarketCap      float64 `gorm:"type:decimal(30,18)" json:"market_cap"`
	Volume24h      float64 `gorm:"type:decimal(30,18)" json:"volume_24h"`
	PriceChange24h float64 `gorm:"type:decimal(10,4)" json:"price_change_24h"`
	PriceChange7d  float64 `gorm:"type:decimal(10,4)" json:"price_change_7d"`
	AllTimeHigh    float64 `gorm:"type:decimal(30,18)" json:"all_time_high"`
	AllTimeLow     float64 `gorm:"type:decimal(30,18)" json:"all_time_low"`

	// Launch Information
	LaunchDate     *time.Time `json:"launch_date,omitempty"`
	LaunchPrice    float64    `gorm:"type:decimal(30,18)" json:"launch_price"`
	LaunchedBy     string     `json:"launched_by,omitempty"`
	LaunchPlatform string     `json:"launch_platform,omitempty"`

	// Technical Details
	ContractInfo   ContractInfo   `gorm:"type:json" json:"contract_info"`
	TokenomicsInfo TokenomicsInfo `gorm:"type:json" json:"tokenomics_info"`
	SecurityInfo   SecurityInfo   `gorm:"type:json" json:"security_info"`

	// Trading Information
	PairAddresses  []string        `gorm:"type:json" json:"pair_addresses,omitempty"`
	DEXListings    []DEXListing    `gorm:"type:json" json:"dex_listings,omitempty"`
	LiquidityPools []LiquidityPool `gorm:"type:json" json:"liquidity_pools,omitempty"`

	// Community and Social
	SocialInfo   SocialInfo `gorm:"type:json" json:"social_info"`
	HolderCount  int        `json:"holder_count"`
	WhaleHolders int        `json:"whale_holders"`

	// Risk Assessment
	RiskScore   float64   `gorm:"type:decimal(3,1)" json:"risk_score"`
	RiskLevel   RiskLevel `gorm:"type:varchar(20)" json:"risk_level"`
	RiskFactors []string  `gorm:"type:json" json:"risk_factors,omitempty"`
	SafetyScore float64   `gorm:"type:decimal(3,1)" json:"safety_score"`

	// Analysis Data
	TechnicalAnalysis TechnicalAnalysis `gorm:"type:json" json:"technical_analysis"`
	FundamentalScore  float64           `gorm:"type:decimal(3,1)" json:"fundamental_score"`
	SentimentScore    float64           `gorm:"type:decimal(3,1)" json:"sentiment_score"`

	// Trading Statistics
	TradingStats    TradingStatistics `gorm:"type:json" json:"trading_stats"`
	VolatilityScore float64           `gorm:"type:decimal(5,2)" json:"volatility_score"`
	TrendScore      float64           `gorm:"type:decimal(3,1)" json:"trend_score"`

	// Metadata and Tracking
	Description string `json:"description,omitempty"`
	Website     string `json:"website,omitempty"`
	LogoURL     string `json:"logo_url,omitempty"`
	Verified    bool   `gorm:"default:false" json:"verified"`
	Featured    bool   `gorm:"default:false" json:"featured"`
	Trending    bool   `gorm:"default:false" json:"trending"`

	// Monitoring Information
	FirstSeenAt      time.Time        `json:"first_seen_at"`
	LastSeenAt       time.Time        `json:"last_seen_at"`
	MonitoringStatus MonitoringStatus `gorm:"type:varchar(20);default:'active'" json:"monitoring_status"`
	ScanInterval     time.Duration    `json:"scan_interval"`
	LastScanAt       *time.Time       `json:"last_scan_at,omitempty"`

	// Relationships
	Trades       []Trade        `gorm:"foreignKey:TokenAddress;references:Address" json:"trades,omitempty"`
	Positions    []Position     `gorm:"foreignKey:TokenAddress;references:Address" json:"positions,omitempty"`
	PriceHistory []PriceHistory `gorm:"foreignKey:TokenAddress;references:Address" json:"price_history,omitempty"`
}

// TokenStatus represents the current status of a token
type TokenStatus string

const (
	TokenStatusActive     TokenStatus = "active"
	TokenStatusInactive   TokenStatus = "inactive"
	TokenStatusSuspicious TokenStatus = "suspicious"
	TokenStatusScam       TokenStatus = "scam"
	TokenStatusDead       TokenStatus = "dead"
	TokenStatusDelisted   TokenStatus = "delisted"
)

// TokenType represents the type/category of token
type TokenType string

const (
	TokenTypeUtility    TokenType = "utility"
	TokenTypeGovernance TokenType = "governance"
	TokenTypeSecurity   TokenType = "security"
	TokenTypeStablecoin TokenType = "stablecoin"
	TokenTypeMeme       TokenType = "meme"
	TokenTypeGameFi     TokenType = "gamefi"
	TokenTypeDeFi       TokenType = "defi"
	TokenTypeNFT        TokenType = "nft"
	TokenTypeUnknown    TokenType = "unknown"
)

// TokenCategory represents broader categorization
type TokenCategory string

const (
	CategoryDeFi           TokenCategory = "defi"
	CategoryGameFi         TokenCategory = "gamefi"
	CategoryNFT            TokenCategory = "nft"
	CategoryMeme           TokenCategory = "meme"
	CategoryInfrastructure TokenCategory = "infrastructure"
	CategoryStablecoin     TokenCategory = "stablecoin"
	CategoryPrivacy        TokenCategory = "privacy"
	CategoryOracle         TokenCategory = "oracle"
	CategoryDAO            TokenCategory = "dao"
	CategorySocialFi       TokenCategory = "socialfi"
	CategoryMetaverse      TokenCategory = "metaverse"
	CategoryAI             TokenCategory = "ai"
)

// RiskLevel represents the risk assessment level
type RiskLevel string

const (
	RiskLevelVeryLow  RiskLevel = "very_low"
	RiskLevelLow      RiskLevel = "low"
	RiskLevelMedium   RiskLevel = "medium"
	RiskLevelHigh     RiskLevel = "high"
	RiskLevelVeryHigh RiskLevel = "very_high"
	RiskLevelExtreme  RiskLevel = "extreme"
)

// MonitoringStatus represents the monitoring status
type MonitoringStatus string

const (
	MonitoringActive   MonitoringStatus = "active"
	MonitoringPaused   MonitoringStatus = "paused"
	MonitoringStopped  MonitoringStatus = "stopped"
	MonitoringArchived MonitoringStatus = "archived"
)

// ContractInfo holds contract-specific information
type ContractInfo struct {
	MintAuthority   string    `json:"mint_authority,omitempty"`
	FreezeAuthority string    `json:"freeze_authority,omitempty"`
	Owner           string    `json:"owner,omitempty"`
	Mintable        bool      `json:"mintable"`
	Freezable       bool      `json:"freezable"`
	Renounced       bool      `json:"renounced"`
	ProgramID       string    `json:"program_id,omitempty"`
	TokenStandard   string    `json:"token_standard,omitempty"`
	MetadataAccount string    `json:"metadata_account,omitempty"`
	CreatedAt       time.Time `json:"created_at"`
	UpdatedAt       time.Time `json:"updated_at"`
}

// TokenomicsInfo holds tokenomics information
type TokenomicsInfo struct {
	TotalSupply       float64           `json:"total_supply"`
	CirculatingSupply float64           `json:"circulating_supply"`
	MaxSupply         *float64          `json:"max_supply,omitempty"`
	BurnedTokens      float64           `json:"burned_tokens"`
	InflationRate     float64           `json:"inflation_rate"`
	Distribution      TokenDistribution `json:"distribution"`
	VestingSchedule   []VestingSchedule `json:"vesting_schedule,omitempty"`
	SupplyModel       string            `json:"supply_model"` // deflationary, inflationary, fixed
}

// TokenDistribution represents token allocation
type TokenDistribution struct {
	PublicSale  float64 `json:"public_sale"`
	PrivateSale float64 `json:"private_sale"`
	Team        float64 `json:"team"`
	Development float64 `json:"development"`
	Marketing   float64 `json:"marketing"`
	Liquidity   float64 `json:"liquidity"`
	Reserve     float64 `json:"reserve"`
	Staking     float64 `json:"staking"`
	Other       float64 `json:"other"`
}

// VestingSchedule represents vesting information
type VestingSchedule struct {
	Beneficiary     string    `json:"beneficiary"`
	Amount          float64   `json:"amount"`
	StartDate       time.Time `json:"start_date"`
	CliffPeriod     int       `json:"cliff_period"`   // Days
	VestingPeriod   int       `json:"vesting_period"` // Days
	ReleasedAmount  float64   `json:"released_amount"`
	NextReleaseDate time.Time `json:"next_release_date"`
}

// SecurityInfo holds security assessment information
type SecurityInfo struct {
	HoneypotRisk       bool            `json:"honeypot_risk"`
	LiquidityLocked    bool            `json:"liquidity_locked"`
	LockDuration       *time.Duration  `json:"lock_duration,omitempty"`
	LockExpiry         *time.Time      `json:"lock_expiry,omitempty"`
	RugPullRisk        float64         `json:"rug_pull_risk"`
	OwnershipRenounced bool            `json:"ownership_renounced"`
	AuditStatus        AuditStatus     `json:"audit_status"`
	AuditReports       []AuditReport   `json:"audit_reports,omitempty"`
	SecurityScore      float64         `json:"security_score"`
	Vulnerabilities    []Vulnerability `json:"vulnerabilities,omitempty"`
	LastSecurityCheck  time.Time       `json:"last_security_check"`
}

// AuditStatus represents audit status
type AuditStatus string

const (
	AuditNotAudited AuditStatus = "not_audited"
	AuditPending    AuditStatus = "pending"
	AuditPassed     AuditStatus = "passed"
	AuditFailed     AuditStatus = "failed"
	AuditPartial    AuditStatus = "partial"
)

// AuditReport represents an audit report
type AuditReport struct {
	Auditor        string      `json:"auditor"`
	Date           time.Time   `json:"date"`
	Status         AuditStatus `json:"status"`
	Score          float64     `json:"score"`
	ReportURL      string      `json:"report_url,omitempty"`
	Issues         int         `json:"issues"`
	CriticalIssues int         `json:"critical_issues"`
	Summary        string      `json:"summary,omitempty"`
}

// Vulnerability represents a security vulnerability
type Vulnerability struct {
	Type        string     `json:"type"`
	Severity    string     `json:"severity"`
	Description string     `json:"description"`
	Status      string     `json:"status"`
	FoundAt     time.Time  `json:"found_at"`
	FixedAt     *time.Time `json:"fixed_at,omitempty"`
}

// SocialInfo holds social media and community information
type SocialInfo struct {
	Website       string `json:"website,omitempty"`
	Twitter       string `json:"twitter,omitempty"`
	Telegram      string `json:"telegram,omitempty"`
	Discord       string `json:"discord,omitempty"`
	Reddit        string `json:"reddit,omitempty"`
	Medium        string `json:"medium,omitempty"`
	GitHub        string `json:"github,omitempty"`
	CoinGecko     string `json:"coingecko,omitempty"`
	CoinMarketCap string `json:"coinmarketcap,omitempty"`

	// Social metrics
	TwitterFollowers int `json:"twitter_followers"`
	TelegramMembers  int `json:"telegram_members"`
	DiscordMembers   int `json:"discord_members"`
	RedditMembers    int `json:"reddit_members"`
	GitHubStars      int `json:"github_stars"`

	// Engagement metrics
	SocialScore       float64   `json:"social_score"`
	CommunityHealth   float64   `json:"community_health"`
	DeveloperActivity float64   `json:"developer_activity"`
	LastSocialUpdate  time.Time `json:"last_social_update"`
}

// DEXListing represents a DEX listing
type DEXListing struct {
	DEX         string    `json:"dex"`
	PairAddress string    `json:"pair_address"`
	BaseToken   string    `json:"base_token"`
	QuoteToken  string    `json:"quote_token"`
	ListedAt    time.Time `json:"listed_at"`
	IsActive    bool      `json:"is_active"`
	Volume24h   float64   `json:"volume_24h"`
	Liquidity   float64   `json:"liquidity"`
	FeePercent  float64   `json:"fee_percent"`
}

// LiquidityPool represents a liquidity pool
type LiquidityPool struct {
	Address        string    `json:"address"`
	DEX            string    `json:"dex"`
	Token0         string    `json:"token0"`
	Token1         string    `json:"token1"`
	Reserve0       float64   `json:"reserve0"`
	Reserve1       float64   `json:"reserve1"`
	TotalLiquidity float64   `json:"total_liquidity"`
	Volume24h      float64   `json:"volume_24h"`
	Fees24h        float64   `json:"fees_24h"`
	APR            float64   `json:"apr"`
	CreatedAt      time.Time `json:"created_at"`
	LastUpdated    time.Time `json:"last_updated"`
}

// TechnicalAnalysis holds technical analysis data
type TechnicalAnalysis struct {
	// Moving averages
	SMA7   float64 `json:"sma_7"`
	SMA25  float64 `json:"sma_25"`
	SMA50  float64 `json:"sma_50"`
	SMA200 float64 `json:"sma_200"`
	EMA7   float64 `json:"ema_7"`
	EMA25  float64 `json:"ema_25"`
	EMA50  float64 `json:"ema_50"`

	// Technical indicators
	RSI            float64 `json:"rsi"`
	MACD           float64 `json:"macd"`
	MACDSignal     float64 `json:"macd_signal"`
	BollingerUpper float64 `json:"bollinger_upper"`
	BollingerLower float64 `json:"bollinger_lower"`
	StochasticK    float64 `json:"stochastic_k"`
	StochasticD    float64 `json:"stochastic_d"`

	// Volume indicators
	VolumeMA float64 `json:"volume_ma"`
	OBV      float64 `json:"obv"`

	// Support and resistance
	SupportLevels    []float64 `json:"support_levels"`
	ResistanceLevels []float64 `json:"resistance_levels"`

	// Trend analysis
	TrendDirection string  `json:"trend_direction"` // bullish, bearish, sideways
	TrendStrength  float64 `json:"trend_strength"`

	// Signals
	BuySignals    []string `json:"buy_signals"`
	SellSignals   []string `json:"sell_signals"`
	OverallSignal string   `json:"overall_signal"` // strong_buy, buy, hold, sell, strong_sell

	// Analysis timestamp
	CalculatedAt time.Time `json:"calculated_at"`
	ValidUntil   time.Time `json:"valid_until"`
}

// TradingStatistics holds trading statistics for the token
type TradingStatistics struct {
	// Volume statistics
	Volume1h        float64 `json:"volume_1h"`
	Volume24h       float64 `json:"volume_24h"`
	Volume7d        float64 `json:"volume_7d"`
	Volume30d       float64 `json:"volume_30d"`
	VolumeChange24h float64 `json:"volume_change_24h"`

	// Transaction statistics
	Transactions24h int64   `json:"transactions_24h"`
	Buys24h         int64   `json:"buys_24h"`
	Sells24h        int64   `json:"sells_24h"`
	BuyVsSellRatio  float64 `json:"buy_vs_sell_ratio"`

	// Price statistics
	HighPrice24h  float64 `json:"high_price_24h"`
	LowPrice24h   float64 `json:"low_price_24h"`
	OpenPrice24h  float64 `json:"open_price_24h"`
	ClosePrice24h float64 `json:"close_price_24h"`
	VWAP24h       float64 `json:"vwap_24h"`

	// Volatility metrics
	Volatility24h float64 `json:"volatility_24h"`
	Volatility7d  float64 `json:"volatility_7d"`
	Volatility30d float64 `json:"volatility_30d"`
	BetaValue     float64 `json:"beta_value"`

	// Liquidity metrics
	AverageLiquidity   float64 `json:"average_liquidity"`
	LiquidityChange24h float64 `json:"liquidity_change_24h"`

	// Market metrics
	MarketCapRank int `json:"market_cap_rank"`
	VolumeRank    int `json:"volume_rank"`

	// Holder metrics
	NewHolders24h    int     `json:"new_holders_24h"`
	ActiveHolders24h int     `json:"active_holders_24h"`
	WhaleActivity    float64 `json:"whale_activity"`

	LastUpdated time.Time `json:"last_updated"`
}

// PriceHistory represents historical price data
type PriceHistory struct {
	BaseModel
	TokenAddress string    `gorm:"not null;index" json:"token_address"`
	Timestamp    time.Time `gorm:"not null;index" json:"timestamp"`
	Open         float64   `gorm:"type:decimal(30,18)" json:"open"`
	High         float64   `gorm:"type:decimal(30,18)" json:"high"`
	Low          float64   `gorm:"type:decimal(30,18)" json:"low"`
	Close        float64   `gorm:"type:decimal(30,18)" json:"close"`
	Volume       float64   `gorm:"type:decimal(30,18)" json:"volume"`
	MarketCap    float64   `gorm:"type:decimal(30,18)" json:"market_cap"`
	Liquidity    float64   `gorm:"type:decimal(30,18)" json:"liquidity"`
	Transactions int64     `json:"transactions"`
	Interval     string    `gorm:"index" json:"interval"` // 1m, 5m, 15m, 1h, 4h, 1d
}

// GORM Hooks

// BeforeCreate is called before creating a token
func (t *Token) BeforeCreate(tx *gorm.DB) error {
	if t.FirstSeenAt.IsZero() {
		t.FirstSeenAt = time.Now()
	}

	if t.LastSeenAt.IsZero() {
		t.LastSeenAt = time.Now()
	}

	// Set default scan interval
	if t.ScanInterval == 0 {
		t.ScanInterval = 1 * time.Minute
	}

	// Calculate initial risk score if not set
	if t.RiskScore == 0 {
		t.calculateRiskScore()
	}

	return nil
}

// BeforeUpdate is called before updating a token
func (t *Token) BeforeUpdate(tx *gorm.DB) error {
	t.LastSeenAt = time.Now()

	// Recalculate risk score
	t.calculateRiskScore()

	// Update risk level based on score
	t.updateRiskLevel()

	return nil
}

// Token Methods

// calculateRiskScore calculates the risk score based on various factors
func (t *Token) calculateRiskScore() {
	var score float64 = 5.0 // Start with medium risk

	// Contract security factors
	if t.SecurityInfo.HoneypotRisk {
		score += 3.0
	}

	if !t.SecurityInfo.LiquidityLocked {
		score += 2.0
	}

	if !t.SecurityInfo.OwnershipRenounced {
		score += 1.5
	}

	// Market factors
	if t.MarketCap < 10000 { // Very small market cap
		score += 2.0
	} else if t.MarketCap < 100000 {
		score += 1.0
	}

	// Volatility factors
	if t.VolatilityScore > 80 {
		score += 1.5
	} else if t.VolatilityScore > 60 {
		score += 1.0
	}

	// Age factors
	if t.LaunchDate != nil {
		age := time.Since(*t.LaunchDate)
		if age < 24*time.Hour {
			score += 2.0
		} else if age < 7*24*time.Hour {
			score += 1.0
		}
	}

	// Liquidity factors
	totalLiquidity := t.getTotalLiquidity()
	if totalLiquidity < 1 { // Less than 1 SOL
		score += 3.0
	} else if totalLiquidity < 10 {
		score += 2.0
	} else if totalLiquidity < 100 {
		score += 1.0
	}

	// Holder distribution
	if t.HolderCount < 10 {
		score += 2.0
	} else if t.HolderCount < 100 {
		score += 1.0
	}

	// Audit status
	switch t.SecurityInfo.AuditStatus {
	case AuditPassed:
		score -= 1.0
	case AuditFailed:
		score += 2.0
	case AuditNotAudited:
		score += 0.5
	}

	// Social presence
	if t.SocialInfo.SocialScore < 3.0 {
		score += 1.0
	} else if t.SocialInfo.SocialScore > 7.0 {
		score -= 0.5
	}

	// Ensure score is within bounds
	if score < 0 {
		score = 0
	} else if score > 10 {
		score = 10
	}

	t.RiskScore = score
}

// updateRiskLevel updates the risk level based on the risk score
func (t *Token) updateRiskLevel() {
	switch {
	case t.RiskScore <= 2.0:
		t.RiskLevel = RiskLevelVeryLow
	case t.RiskScore <= 4.0:
		t.RiskLevel = RiskLevelLow
	case t.RiskScore <= 6.0:
		t.RiskLevel = RiskLevelMedium
	case t.RiskScore <= 8.0:
		t.RiskLevel = RiskLevelHigh
	case t.RiskScore <= 9.0:
		t.RiskLevel = RiskLevelVeryHigh
	default:
		t.RiskLevel = RiskLevelExtreme
	}
}

// getTotalLiquidity calculates total liquidity across all pools
func (t *Token) getTotalLiquidity() float64 {
	var total float64
	for _, pool := range t.LiquidityPools {
		total += pool.TotalLiquidity
	}
	return total
}

// IsTradeRecommended checks if trading this token is recommended
func (t *Token) IsTradeRecommended() bool {
	return t.Status == TokenStatusActive &&
		t.RiskScore <= 7.0 &&
		t.getTotalLiquidity() >= 1.0 &&
		!t.SecurityInfo.HoneypotRisk
}

// GetAgeInHours returns the age of the token in hours
func (t *Token) GetAgeInHours() float64 {
	if t.LaunchDate == nil {
		return 0
	}
	return time.Since(*t.LaunchDate).Hours()
}

// IsNewListing checks if this is a newly listed token (< 1 hour old)
func (t *Token) IsNewListing() bool {
	return t.GetAgeInHours() < 1.0
}

// GetPriceChangePercent returns price change percentage for specified period
func (t *Token) GetPriceChangePercent(period string) float64 {
	switch period {
	case "24h":
		return t.PriceChange24h
	case "7d":
		return t.PriceChange7d
	default:
		return 0
	}
}

// NeedsRiskUpdate checks if risk assessment needs updating
func (t *Token) NeedsRiskUpdate() bool {
	return time.Since(t.SecurityInfo.LastSecurityCheck) > 24*time.Hour
}

// Custom JSON marshaling and database scanning

// Scan implements the sql.Scanner interface for ContractInfo
func (ci *ContractInfo) Scan(value interface{}) error {
	if value == nil {
		return nil
	}

	bytes, ok := value.([]byte)
	if !ok {
		return fmt.Errorf("cannot scan %T into ContractInfo", value)
	}

	return json.Unmarshal(bytes, ci)
}

// Value implements the driver.Valuer interface for ContractInfo
func (ci ContractInfo) Value() (driver.Value, error) {
	return json.Marshal(ci)
}

// Scan implements the sql.Scanner interface for TokenomicsInfo
func (ti *TokenomicsInfo) Scan(value interface{}) error {
	if value == nil {
		return nil
	}

	bytes, ok := value.([]byte)
	if !ok {
		return fmt.Errorf("cannot scan %T into TokenomicsInfo", value)
	}

	return json.Unmarshal(bytes, ti)
}

// Value implements the driver.Valuer interface for TokenomicsInfo
func (ti TokenomicsInfo) Value() (driver.Value, error) {
	return json.Marshal(ti)
}

// Scan implements the sql.Scanner interface for SecurityInfo
func (si *SecurityInfo) Scan(value interface{}) error {
	if value == nil {
		return nil
	}

	bytes, ok := value.([]byte)
	if !ok {
		return fmt.Errorf("cannot scan %T into SecurityInfo", value)
	}

	return json.Unmarshal(bytes, si)
}

// Value implements the driver.Valuer interface for SecurityInfo
func (si SecurityInfo) Value() (driver.Value, error) {
	return json.Marshal(si)
}

// Scan implements the sql.Scanner interface for SocialInfo
func (si *SocialInfo) Scan(value interface{}) error {
	if value == nil {
		return nil
	}

	bytes, ok := value.([]byte)
	if !ok {
		return fmt.Errorf("cannot scan %T into SocialInfo", value)
	}

	return json.Unmarshal(bytes, si)
}

// Value implements the driver.Valuer interface for SocialInfo
func (si SocialInfo) Value() (driver.Value, error) {
	return json.Marshal(si)
}

// Scan implements the sql.Scanner interface for TechnicalAnalysis
func (ta *TechnicalAnalysis) Scan(value interface{}) error {
	if value == nil {
		return nil
	}

	bytes, ok := value.([]byte)
	if !ok {
		return fmt.Errorf("cannot scan %T into TechnicalAnalysis", value)
	}

	return json.Unmarshal(bytes, ta)
}

// Value implements the driver.Valuer interface for TechnicalAnalysis
func (ta TechnicalAnalysis) Value() (driver.Value, error) {
	return json.Marshal(ta)
}

// Scan implements the sql.Scanner interface for TradingStatistics
func (ts *TradingStatistics) Scan(value interface{}) error {
	if value == nil {
		return nil
	}

	bytes, ok := value.([]byte)
	if !ok {
		return fmt.Errorf("cannot scan %T into TradingStatistics", value)
	}

	return json.Unmarshal(bytes, ts)
}

// Value implements the driver.Valuer interface for TradingStatistics
func (ts TradingStatistics) Value() (driver.Value, error) {
	return json.Marshal(ts)
}

// Repository and Service Interfaces

// TokenRepository interface (to be implemented in Phase 2)
type TokenRepository interface {
	Create(token *Token) error
	GetByAddress(address string) (*Token, error)
	GetBySymbol(symbol string) ([]*Token, error)
	Update(token *Token) error
	Delete(address string) error
	List(filters *TokenFilters) ([]*Token, error)
	GetNewTokens(since time.Time) ([]*Token, error)
	GetTrendingTokens(limit int) ([]*Token, error)
	GetTokensByRiskLevel(riskLevel RiskLevel, limit int) ([]*Token, error)
	Search(query string, limit int) ([]*Token, error)
	UpdatePrice(address string, price float64, marketCap float64) error
	BulkUpdatePrices(prices map[string]float64) error
}

// TokenService interface (to be implemented in later phases)
type TokenService interface {
	DiscoverNewToken(address string) (*Token, error)
	UpdateTokenInfo(address string) error
	AnalyzeToken(address string) (*TokenAnalysis, error)
	GetTokenRecommendations(userID uint) ([]*Token, error)
	UpdateRiskAssessment(address string) error
	UpdateMarketData(address string) error
	UpdateTechnicalAnalysis(address string) error
	GetTokensByFilters(filters *TokenFilters) ([]*Token, error)
	MonitorToken(address string) error
	StopMonitoring(address string) error
}

// TokenFilters represents filters for querying tokens
type TokenFilters struct {
	Status          []TokenStatus   `json:"status,omitempty"`
	Type            []TokenType     `json:"type,omitempty"`
	Category        []TokenCategory `json:"category,omitempty"`
	RiskLevel       []RiskLevel     `json:"risk_level,omitempty"`
	MinMarketCap    *float64        `json:"min_market_cap,omitempty"`
	MaxMarketCap    *float64        `json:"max_market_cap,omitempty"`
	MinLiquidity    *float64        `json:"min_liquidity,omitempty"`
	MaxAge          *time.Duration  `json:"max_age,omitempty"`
	MinHolders      *int            `json:"min_holders,omitempty"`
	Verified        *bool           `json:"verified,omitempty"`
	Featured        *bool           `json:"featured,omitempty"`
	Trending        *bool           `json:"trending,omitempty"`
	HasAudit        *bool           `json:"has_audit,omitempty"`
	LiquidityLocked *bool           `json:"liquidity_locked,omitempty"`
	Tags            []string        `json:"tags,omitempty"`
	DEX             []string        `json:"dex,omitempty"`
	SortBy          string          `json:"sort_by,omitempty"`
	SortOrder       string          `json:"sort_order,omitempty"`
	Limit           int             `json:"limit,omitempty"`
	Offset          int             `json:"offset,omitempty"`
}

// TokenAnalysis represents comprehensive token analysis results
type TokenAnalysis struct {
	Token               *Token                 `json:"token"`
	RiskAssessment      *RiskAssessment        `json:"risk_assessment"`
	TechnicalAnalysis   *TechnicalAnalysis     `json:"technical_analysis"`
	FundamentalAnalysis *FundamentalAnalysis   `json:"fundamental_analysis"`
	SentimentAnalysis   *SentimentAnalysis     `json:"sentiment_analysis"`
	TradingSignals      *TradingSignals        `json:"trading_signals"`
	Recommendation      *TradingRecommendation `json:"recommendation"`
	AnalyzedAt          time.Time              `json:"analyzed_at"`
}

// RiskAssessment represents detailed risk assessment
type RiskAssessment struct {
	OverallRisk     RiskLevel       `json:"overall_risk"`
	RiskScore       float64         `json:"risk_score"`
	RiskFactors     []RiskFactor    `json:"risk_factors"`
	SecurityRisks   []SecurityRisk  `json:"security_risks"`
	MarketRisks     []MarketRisk    `json:"market_risks"`
	LiquidityRisks  []LiquidityRisk `json:"liquidity_risks"`
	Recommendations []string        `json:"recommendations"`
	LastAssessed    time.Time       `json:"last_assessed"`
}

// RiskFactor represents a specific risk factor
type RiskFactor struct {
	Type        string  `json:"type"`
	Severity    string  `json:"severity"`
	Impact      float64 `json:"impact"`
	Description string  `json:"description"`
	Mitigation  string  `json:"mitigation,omitempty"`
}

// SecurityRisk represents security-related risks
type SecurityRisk struct {
	Type        string                 `json:"type"`
	Severity    string                 `json:"severity"`
	Description string                 `json:"description"`
	Detected    bool                   `json:"detected"`
	Details     map[string]interface{} `json:"details,omitempty"`
}

// MarketRisk represents market-related risks
type MarketRisk struct {
	Type        string  `json:"type"`
	Level       string  `json:"level"`
	Description string  `json:"description"`
	Impact      float64 `json:"impact"`
	Probability float64 `json:"probability"`
}

// LiquidityRisk represents liquidity-related risks
type LiquidityRisk struct {
	Type             string  `json:"type"`
	Level            string  `json:"level"`
	Description      string  `json:"description"`
	CurrentLiquidity float64 `json:"current_liquidity"`
	MinimumLiquidity float64 `json:"minimum_liquidity"`
	LiquidityRatio   float64 `json:"liquidity_ratio"`
}

// FundamentalAnalysis represents fundamental analysis
type FundamentalAnalysis struct {
	ProjectScore    float64            `json:"project_score"`
	TeamScore       float64            `json:"team_score"`
	TechnologyScore float64            `json:"technology_score"`
	CommunityScore  float64            `json:"community_score"`
	TokenomicsScore float64            `json:"tokenomics_score"`
	OverallScore    float64            `json:"overall_score"`
	StrengthFactors []string           `json:"strength_factors"`
	WeaknessFactors []string           `json:"weakness_factors"`
	KeyMetrics      map[string]float64 `json:"key_metrics"`
	LastAnalyzed    time.Time          `json:"last_analyzed"`
}

// SentimentAnalysis represents sentiment analysis
type SentimentAnalysis struct {
	OverallSentiment string    `json:"overall_sentiment"`
	SentimentScore   float64   `json:"sentiment_score"`
	SocialSentiment  float64   `json:"social_sentiment"`
	NewsSentiment    float64   `json:"news_sentiment"`
	TradingSentiment float64   `json:"trading_sentiment"`
	SentimentTrend   string    `json:"sentiment_trend"`
	KeyTopics        []string  `json:"key_topics"`
	MentionVolume    int       `json:"mention_volume"`
	LastAnalyzed     time.Time `json:"last_analyzed"`
}

// TradingSignals represents trading signals
type TradingSignals struct {
	OverallSignal  string          `json:"overall_signal"`
	SignalStrength float64         `json:"signal_strength"`
	BuySignals     []TradingSignal `json:"buy_signals"`
	SellSignals    []TradingSignal `json:"sell_signals"`
	NeutralSignals []TradingSignal `json:"neutral_signals"`
	Confidence     float64         `json:"confidence"`
	TimeHorizon    string          `json:"time_horizon"`
	LastUpdated    time.Time       `json:"last_updated"`
}

// TradingSignal represents a specific trading signal
type TradingSignal struct {
	Type        string    `json:"type"`
	Direction   string    `json:"direction"`
	Strength    float64   `json:"strength"`
	Confidence  float64   `json:"confidence"`
	Description string    `json:"description"`
	Source      string    `json:"source"`
	GeneratedAt time.Time `json:"generated_at"`
}

// TradingRecommendation represents a trading recommendation
type TradingRecommendation struct {
	Action          string    `json:"action"` // buy, sell, hold, avoid
	Confidence      float64   `json:"confidence"`
	RecommendedSize float64   `json:"recommended_size"`
	EntryPrice      float64   `json:"entry_price"`
	StopLoss        float64   `json:"stop_loss"`
	TakeProfit      float64   `json:"take_profit"`
	TimeHorizon     string    `json:"time_horizon"`
	RiskReward      float64   `json:"risk_reward"`
	Reasoning       []string  `json:"reasoning"`
	Warnings        []string  `json:"warnings"`
	GeneratedAt     time.Time `json:"generated_at"`
	ValidUntil      time.Time `json:"valid_until"`
}

// Helper functions

// GetTokenRiskColor returns a color code based on risk level
func GetTokenRiskColor(riskLevel RiskLevel) string {
	switch riskLevel {
	case RiskLevelVeryLow:
		return "#00FF00" // Green
	case RiskLevelLow:
		return "#90EE90" // Light Green
	case RiskLevelMedium:
		return "#FFD700" // Gold
	case RiskLevelHigh:
		return "#FFA500" // Orange
	case RiskLevelVeryHigh:
		return "#FF4500" // Red Orange
	case RiskLevelExtreme:
		return "#FF0000" // Red
	default:
		return "#808080" // Gray
	}
}

// FormatTokenAmount formats token amounts with proper decimals
func FormatTokenAmount(amount float64, decimals int) string {
	if decimals > 9 {
		decimals = 9 // Cap at 9 decimals for display
	}

	format := fmt.Sprintf("%%.%df", decimals)
	return fmt.Sprintf(format, amount)
}

// CalculateMarketCap calculates market cap from price and supply
func CalculateMarketCap(price, circulatingSupply float64) float64 {
	return price * circulatingSupply
}

// CalculatePriceChange calculates price change percentage
func CalculatePriceChange(oldPrice, newPrice float64) float64 {
	if oldPrice == 0 {
		return 0
	}
	return ((newPrice - oldPrice) / oldPrice) * 100
}
