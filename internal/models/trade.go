// Package models - Trade model and related structures
package models

import (
	"database/sql/driver"
	"encoding/json"
	"fmt"
	"time"

	"gorm.io/gorm"
)

// Trade represents a trading transaction
type Trade struct {
	BaseModel

	// User and Position References
	UserID     uint      `gorm:"not null;index" json:"user_id"`
	User       User      `gorm:"foreignKey:UserID" json:"user,omitempty"`
	PositionID *uint     `gorm:"index" json:"position_id,omitempty"`
	Position   *Position `gorm:"foreignKey:PositionID" json:"position,omitempty"`

	// Token Information
	TokenAddress  string `gorm:"not null;index" json:"token_address"`
	TokenSymbol   string `gorm:"index" json:"token_symbol"`
	TokenName     string `json:"token_name"`
	TokenDecimals int    `gorm:"default:9" json:"token_decimals"`

	// Trade Details
	Type     TradeType     `gorm:"type:varchar(20);not null" json:"type"`
	Side     TradeSide     `gorm:"type:varchar(10);not null" json:"side"`
	Status   TradeStatus   `gorm:"type:varchar(20);default:'pending'" json:"status"`
	Strategy TradeStrategy `gorm:"type:varchar(30)" json:"strategy,omitempty"`

	// Quantities and Pricing
	Quantity         float64 `gorm:"type:decimal(30,18);not null" json:"quantity"`
	Price            float64 `gorm:"type:decimal(30,18);not null" json:"price"`
	QuoteAmount      float64 `gorm:"type:decimal(30,18);not null" json:"quote_amount"` // SOL amount
	ExecutedQuantity float64 `gorm:"type:decimal(30,18);default:0" json:"executed_quantity"`
	ExecutedPrice    float64 `gorm:"type:decimal(30,18);default:0" json:"executed_price"`
	ExecutedAmount   float64 `gorm:"type:decimal(30,18);default:0" json:"executed_amount"`

	// Slippage and Fees
	RequestedSlippage float64 `gorm:"type:decimal(5,2)" json:"requested_slippage"`
	ActualSlippage    float64 `gorm:"type:decimal(5,2);default:0" json:"actual_slippage"`
	TradingFee        float64 `gorm:"type:decimal(20,8);default:0" json:"trading_fee"`
	PlatformFee       float64 `gorm:"type:decimal(20,8);default:0" json:"platform_fee"`
	GasFee            float64 `gorm:"type:decimal(20,8);default:0" json:"gas_fee"`
	TotalFees         float64 `gorm:"type:decimal(20,8);default:0" json:"total_fees"`

	// DEX and Transaction Info
	DEX              string `gorm:"index" json:"dex"`
	PairAddress      string `json:"pair_address,omitempty"`
	TransactionHash  string `gorm:"uniqueIndex" json:"transaction_hash,omitempty"`
	BlockNumber      uint64 `json:"block_number,omitempty"`
	TransactionIndex uint   `json:"transaction_index,omitempty"`

	// Timing Information
	RequestedAt      time.Time  `gorm:"not null" json:"requested_at"`
	ExecutedAt       *time.Time `json:"executed_at,omitempty"`
	ConfirmedAt      *time.Time `json:"confirmed_at,omitempty"`
	ExecutionTime    int64      `json:"execution_time_ms,omitempty"`    // Milliseconds
	ConfirmationTime int64      `json:"confirmation_time_ms,omitempty"` // Milliseconds

	// Market Data at Trade Time
	MarketData       MarketData `gorm:"type:json" json:"market_data"`
	PriceImpact      float64    `gorm:"type:decimal(5,2)" json:"price_impact,omitempty"`
	LiquidityAtTrade float64    `gorm:"type:decimal(20,8)" json:"liquidity_at_trade,omitempty"`

	// Stop Loss and Take Profit
	StopLoss     *float64 `gorm:"type:decimal(30,18)" json:"stop_loss,omitempty"`
	TakeProfit   *float64 `gorm:"type:decimal(30,18)" json:"take_profit,omitempty"`
	TrailingStop *float64 `gorm:"type:decimal(5,2)" json:"trailing_stop,omitempty"`

	// Risk Management
	RiskScore    float64      `gorm:"type:decimal(3,1)" json:"risk_score,omitempty"`
	RiskFactors  []string     `gorm:"type:json" json:"risk_factors,omitempty"`
	SafetyChecks SafetyChecks `gorm:"type:json" json:"safety_checks"`

	// Performance Tracking
	ProfitLoss        float64 `gorm:"type:decimal(20,8);default:0" json:"profit_loss"`
	ProfitLossPercent float64 `gorm:"type:decimal(10,4);default:0" json:"profit_loss_percent"`
	ROI               float64 `gorm:"type:decimal(10,4);default:0" json:"roi"`

	// Error Information
	ErrorCode    string `json:"error_code,omitempty"`
	ErrorMessage string `json:"error_message,omitempty"`
	RetryCount   int    `gorm:"default:0" json:"retry_count"`
	MaxRetries   int    `gorm:"default:3" json:"max_retries"`

	// Metadata
	TradeSource string                 `json:"trade_source,omitempty"` // telegram, api, auto
	Tags        []string               `gorm:"type:json" json:"tags,omitempty"`
	Notes       string                 `json:"notes,omitempty"`
	Metadata    map[string]interface{} `gorm:"type:json" json:"metadata,omitempty"`

	// Automation Info
	IsAutomated      bool    `gorm:"default:false" json:"is_automated"`
	TriggerCondition string  `json:"trigger_condition,omitempty"`
	ParentTradeID    *uint   `json:"parent_trade_id,omitempty"`
	ChildTrades      []Trade `gorm:"foreignKey:ParentTradeID" json:"child_trades,omitempty"`
}

// TradeType represents the type of trade
type TradeType string

const (
	TradeTypeMarket     TradeType = "market"
	TradeTypeLimit      TradeType = "limit"
	TradeTypeStopLoss   TradeType = "stop_loss"
	TradeTypeTakeProfit TradeType = "take_profit"
	TradeTypeTrailing   TradeType = "trailing"
	TradeTypeSnipe      TradeType = "snipe"
)

// TradeSide represents whether it's a buy or sell
type TradeSide string

const (
	TradeSideBuy  TradeSide = "buy"
	TradeSideSell TradeSide = "sell"
)

// TradeStatus represents the current status of a trade
type TradeStatus string

const (
	TradeStatusPending   TradeStatus = "pending"
	TradeStatusSubmitted TradeStatus = "submitted"
	TradeStatusExecuted  TradeStatus = "executed"
	TradeStatusConfirmed TradeStatus = "confirmed"
	TradeStatusFailed    TradeStatus = "failed"
	TradeStatusCancelled TradeStatus = "cancelled"
	TradeStatusPartial   TradeStatus = "partial"
	TradeStatusExpired   TradeStatus = "expired"
)

// TradeStrategy represents the trading strategy used
type TradeStrategy string

const (
	StrategySnipe      TradeStrategy = "snipe"
	StrategyDCA        TradeStrategy = "dca"
	StrategyGrid       TradeStrategy = "grid"
	StrategyMomentum   TradeStrategy = "momentum"
	StrategyMeanRevert TradeStrategy = "mean_revert"
	StrategyArbitrage  TradeStrategy = "arbitrage"
	StrategyManual     TradeStrategy = "manual"
)

// MarketData holds market information at the time of trade
type MarketData struct {
	Price           float64   `json:"price"`
	Volume24h       float64   `json:"volume_24h"`
	MarketCap       float64   `json:"market_cap,omitempty"`
	Liquidity       float64   `json:"liquidity"`
	PriceChange24h  float64   `json:"price_change_24h"`
	HolderCount     int       `json:"holder_count,omitempty"`
	TokenAge        int64     `json:"token_age_seconds,omitempty"`
	VolatilityScore float64   `json:"volatility_score,omitempty"`
	TrendScore      float64   `json:"trend_score,omitempty"`
	Timestamp       time.Time `json:"timestamp"`
}

// SafetyChecks holds the results of various safety checks
type SafetyChecks struct {
	HoneypotCheck     CheckResult `json:"honeypot_check"`
	ContractVerified  CheckResult `json:"contract_verified"`
	LiquidityLocked   CheckResult `json:"liquidity_locked"`
	RenounceOwnership CheckResult `json:"renounce_ownership"`
	MintingDisabled   CheckResult `json:"minting_disabled"`
	FreezingDisabled  CheckResult `json:"freezing_disabled"`
	TaxCheck          CheckResult `json:"tax_check"`
	RugPullRisk       CheckResult `json:"rug_pull_risk"`
	TimestampChecked  time.Time   `json:"timestamp_checked"`
}

// CheckResult represents the result of a safety check
type CheckResult struct {
	Passed  bool                   `json:"passed"`
	Score   float64                `json:"score,omitempty"`
	Message string                 `json:"message,omitempty"`
	Details map[string]interface{} `json:"details,omitempty"`
}

// GORM Hooks

// BeforeCreate is called before creating a trade
func (t *Trade) BeforeCreate(tx *gorm.DB) error {
	if t.RequestedAt.IsZero() {
		t.RequestedAt = time.Now()
	}

	// Calculate total fees
	t.TotalFees = t.TradingFee + t.PlatformFee + t.GasFee

	// Set default values
	if t.MaxRetries == 0 {
		t.MaxRetries = 3
	}

	return nil
}

// AfterCreate is called after creating a trade
func (t *Trade) AfterCreate(tx *gorm.DB) error {
	// Update user statistics
	return t.updateUserStats(tx)
}

// BeforeUpdate is called before updating a trade
func (t *Trade) BeforeUpdate(tx *gorm.DB) error {
	// Update execution timing
	if t.Status == TradeStatusExecuted && t.ExecutedAt == nil {
		now := time.Now()
		t.ExecutedAt = &now
		t.ExecutionTime = now.Sub(t.RequestedAt).Milliseconds()
	}

	// Update confirmation timing
	if t.Status == TradeStatusConfirmed && t.ConfirmedAt == nil {
		now := time.Now()
		t.ConfirmedAt = &now
		if t.ExecutedAt != nil {
			t.ConfirmationTime = now.Sub(*t.ExecutedAt).Milliseconds()
		}
	}

	// Calculate P&L if trade is completed
	if t.Status == TradeStatusConfirmed || t.Status == TradeStatusExecuted {
		t.calculateProfitLoss()
	}

	// Recalculate total fees
	t.TotalFees = t.TradingFee + t.PlatformFee + t.GasFee

	return nil
}

// AfterUpdate is called after updating a trade
func (t *Trade) AfterUpdate(tx *gorm.DB) error {
	// Update user statistics when trade status changes
	return t.updateUserStats(tx)
}

// Trade Methods

// IsCompleted checks if the trade is in a final state
func (t *Trade) IsCompleted() bool {
	return t.Status == TradeStatusConfirmed ||
		t.Status == TradeStatusFailed ||
		t.Status == TradeStatusCancelled ||
		t.Status == TradeStatusExpired
}

// IsSuccessful checks if the trade was successful
func (t *Trade) IsSuccessful() bool {
	return t.Status == TradeStatusConfirmed || t.Status == TradeStatusExecuted
}

// GetFillPercentage returns the percentage of the order that was filled
// GetFillPercentage returns the percentage of the order that was filled
func (t *Trade) GetFillPercentage() float64 {
	if t.Quantity == 0 {
		return 0
	}
	return (t.ExecutedQuantity / t.Quantity) * 100
}

// CalculateActualSlippage calculates the actual slippage experienced
func (t *Trade) CalculateActualSlippage() float64 {
	if t.Price == 0 || t.ExecutedPrice == 0 {
		return 0
	}

	var slippage float64
	if t.Side == TradeSideBuy {
		// For buys, slippage is positive if we paid more than expected
		slippage = ((t.ExecutedPrice - t.Price) / t.Price) * 100
	} else {
		// For sells, slippage is positive if we received less than expected
		slippage = ((t.Price - t.ExecutedPrice) / t.Price) * 100
	}

	return slippage
}

// calculateProfitLoss calculates profit/loss for the trade
func (t *Trade) calculateProfitLoss() {
	if !t.IsSuccessful() {
		return
	}

	switch t.Side {
	case TradeSideBuy:
		// For buy orders, we don't calculate P&L until we sell
		// This will be calculated when the position is closed
		return
	case TradeSideSell:
		// For sell orders, we need the original buy price
		// This requires position tracking which will be implemented in later phases
		return
	}
}

// updateUserStats updates user statistics after trade changes
func (t *Trade) updateUserStats(tx *gorm.DB) error {
	if !t.IsCompleted() {
		return nil
	}

	var updates map[string]interface{}

	if t.IsSuccessful() {
		updates = map[string]interface{}{
			"total_trades":      gorm.Expr("total_trades + 1"),
			"successful_trades": gorm.Expr("successful_trades + 1"),
			"total_volume":      gorm.Expr("total_volume + ?", t.ExecutedAmount),
		}

		if t.ProfitLoss != 0 {
			if t.ProfitLoss > 0 {
				updates["total_profit"] = gorm.Expr("total_profit + ?", t.ProfitLoss)
			} else {
				updates["total_loss"] = gorm.Expr("total_loss + ?", -t.ProfitLoss)
			}
		}
	} else {
		updates = map[string]interface{}{
			"total_trades": gorm.Expr("total_trades + 1"),
		}
	}

	return tx.Model(&User{}).Where("id = ?", t.UserID).Updates(updates).Error
}

// GetNetAmount returns the net amount after fees
func (t *Trade) GetNetAmount() float64 {
	if t.Side == TradeSideBuy {
		return t.ExecutedAmount + t.TotalFees
	}
	return t.ExecutedAmount - t.TotalFees
}

// GetEffectivePrice returns the effective price including fees
func (t *Trade) GetEffectivePrice() float64 {
	if t.ExecutedQuantity == 0 {
		return 0
	}

	netAmount := t.GetNetAmount()
	return netAmount / t.ExecutedQuantity
}

// IsExpired checks if the trade has expired
func (t *Trade) IsExpired() bool {
	if t.Type != TradeTypeLimit {
		return false
	}

	// Implement expiration logic based on trade type and configuration
	// This will be enhanced in later phases
	return false
}

// CanRetry checks if the trade can be retried
func (t *Trade) CanRetry() bool {
	return t.Status == TradeStatusFailed && t.RetryCount < t.MaxRetries
}

// GetTradeDuration returns the total duration of the trade
func (t *Trade) GetTradeDuration() time.Duration {
	if t.ConfirmedAt != nil {
		return t.ConfirmedAt.Sub(t.RequestedAt)
	}
	if t.ExecutedAt != nil {
		return t.ExecutedAt.Sub(t.RequestedAt)
	}
	return time.Since(t.RequestedAt)
}

// Custom JSON marshaling and database scanning

// Scan implements the sql.Scanner interface for MarketData
func (md *MarketData) Scan(value interface{}) error {
	if value == nil {
		return nil
	}

	bytes, ok := value.([]byte)
	if !ok {
		return fmt.Errorf("cannot scan %T into MarketData", value)
	}

	return json.Unmarshal(bytes, md)
}

// Value implements the driver.Valuer interface for MarketData
func (md MarketData) Value() (driver.Value, error) {
	return json.Marshal(md)
}

// Scan implements the sql.Scanner interface for SafetyChecks
func (sc *SafetyChecks) Scan(value interface{}) error {
	if value == nil {
		return nil
	}

	bytes, ok := value.([]byte)
	if !ok {
		return fmt.Errorf("cannot scan %T into SafetyChecks", value)
	}

	return json.Unmarshal(bytes, sc)
}

// Value implements the driver.Valuer interface for SafetyChecks
func (sc SafetyChecks) Value() (driver.Value, error) {
	return json.Marshal(sc)
}

// Trade Repository Interface (to be implemented in Phase 2)
type TradeRepository interface {
	Create(trade *Trade) error
	GetByID(id uint) (*Trade, error)
	GetByTxHash(hash string) (*Trade, error)
	GetByUserID(userID uint, limit, offset int) ([]*Trade, error)
	GetByStatus(status TradeStatus, limit, offset int) ([]*Trade, error)
	GetPendingTrades() ([]*Trade, error)
	Update(trade *Trade) error
	Delete(id uint) error
	GetUserTradeStats(userID uint) (*TradeStatistics, error)
	GetTradesByDateRange(userID uint, start, end time.Time) ([]*Trade, error)
	GetTradesByToken(tokenAddress string, limit, offset int) ([]*Trade, error)
}

// Trade Service Interface (to be implemented in later phases)
type TradeService interface {
	CreateTrade(request *TradeRequest) (*Trade, error)
	ExecuteTrade(tradeID uint) error
	CancelTrade(tradeID uint) error
	RetryTrade(tradeID uint) error
	GetTrade(tradeID uint) (*Trade, error)
	GetUserTrades(userID uint, filters *TradeFilters) ([]*Trade, error)
	GetTradeHistory(userID uint, limit, offset int) ([]*Trade, error)
	UpdateTradeStatus(tradeID uint, status TradeStatus, txHash string) error
	CalculateTradeMetrics(userID uint) (*TradeMetrics, error)
}

// TradeRequest represents a request to create a new trade
type TradeRequest struct {
	UserID       uint          `json:"user_id" validate:"required"`
	TokenAddress string        `json:"token_address" validate:"required"`
	Type         TradeType     `json:"type" validate:"required"`
	Side         TradeSide     `json:"side" validate:"required"`
	Quantity     float64       `json:"quantity" validate:"required,gt=0"`
	Price        float64       `json:"price,omitempty"`
	QuoteAmount  float64       `json:"quote_amount,omitempty"`
	Slippage     float64       `json:"slippage" validate:"min=0,max=50"`
	DEX          string        `json:"dex" validate:"required"`
	StopLoss     *float64      `json:"stop_loss,omitempty"`
	TakeProfit   *float64      `json:"take_profit,omitempty"`
	TrailingStop *float64      `json:"trailing_stop,omitempty"`
	Strategy     TradeStrategy `json:"strategy,omitempty"`
	Source       string        `json:"source,omitempty"`
	Notes        string        `json:"notes,omitempty"`
	Tags         []string      `json:"tags,omitempty"`
	MaxRetries   int           `json:"max_retries,omitempty"`
}

// TradeFilters represents filters for querying trades
type TradeFilters struct {
	Status       []TradeStatus   `json:"status,omitempty"`
	Type         []TradeType     `json:"type,omitempty"`
	Side         []TradeSide     `json:"side,omitempty"`
	Strategy     []TradeStrategy `json:"strategy,omitempty"`
	DEX          []string        `json:"dex,omitempty"`
	TokenAddress string          `json:"token_address,omitempty"`
	DateFrom     *time.Time      `json:"date_from,omitempty"`
	DateTo       *time.Time      `json:"date_to,omitempty"`
	MinAmount    *float64        `json:"min_amount,omitempty"`
	MaxAmount    *float64        `json:"max_amount,omitempty"`
	Profitable   *bool           `json:"profitable,omitempty"`
	Tags         []string        `json:"tags,omitempty"`
	Limit        int             `json:"limit,omitempty"`
	Offset       int             `json:"offset,omitempty"`
	SortBy       string          `json:"sort_by,omitempty"`
	SortOrder    string          `json:"sort_order,omitempty"`
}

// TradeStatistics represents aggregated trade statistics
type TradeStatistics struct {
	TotalTrades      int64   `json:"total_trades"`
	SuccessfulTrades int64   `json:"successful_trades"`
	FailedTrades     int64   `json:"failed_trades"`
	WinRate          float64 `json:"win_rate"`
	TotalVolume      float64 `json:"total_volume"`
	TotalFees        float64 `json:"total_fees"`
	NetProfit        float64 `json:"net_profit"`
	GrossProfit      float64 `json:"gross_profit"`
	GrossLoss        float64 `json:"gross_loss"`
	AverageProfit    float64 `json:"average_profit"`
	AverageLoss      float64 `json:"average_loss"`
	LargestWin       float64 `json:"largest_win"`
	LargestLoss      float64 `json:"largest_loss"`
	ProfitFactor     float64 `json:"profit_factor"`
	SharpeRatio      float64 `json:"sharpe_ratio,omitempty"`
	MaxDrawdown      float64 `json:"max_drawdown"`
	RecoveryFactor   float64 `json:"recovery_factor,omitempty"`

	// Timing statistics
	AverageExecutionTime    int64 `json:"average_execution_time_ms"`
	AverageConfirmationTime int64 `json:"average_confirmation_time_ms"`
	FastestExecution        int64 `json:"fastest_execution_ms"`
	SlowestExecution        int64 `json:"slowest_execution_ms"`

	// Token statistics
	UniqueTokens        int                `json:"unique_tokens"`
	MostTradedToken     string             `json:"most_traded_token,omitempty"`
	MostProfitableToken string             `json:"most_profitable_token,omitempty"`
	TokenPerformance    map[string]float64 `json:"token_performance,omitempty"`

	// DEX statistics
	DEXBreakdown   map[string]int64   `json:"dex_breakdown,omitempty"`
	DEXPerformance map[string]float64 `json:"dex_performance,omitempty"`

	// Time-based statistics
	DailyAverageVolume float64    `json:"daily_average_volume"`
	TradingDays        int        `json:"trading_days"`
	BestTradingDay     *time.Time `json:"best_trading_day,omitempty"`
	WorstTradingDay    *time.Time `json:"worst_trading_day,omitempty"`

	// Risk metrics
	AverageSlippage  float64 `json:"average_slippage"`
	MaxSlippage      float64 `json:"max_slippage"`
	AverageRiskScore float64 `json:"average_risk_score"`

	// Period information
	PeriodStart time.Time `json:"period_start"`
	PeriodEnd   time.Time `json:"period_end"`
	LastUpdated time.Time `json:"last_updated"`
}

// TradeMetrics represents real-time trading metrics
type TradeMetrics struct {
	CurrentPositions int     `json:"current_positions"`
	TodayTrades      int64   `json:"today_trades"`
	TodayVolume      float64 `json:"today_volume"`
	TodayPnL         float64 `json:"today_pnl"`
	WeekTrades       int64   `json:"week_trades"`
	WeekVolume       float64 `json:"week_volume"`
	WeekPnL          float64 `json:"week_pnl"`
	MonthTrades      int64   `json:"month_trades"`
	MonthVolume      float64 `json:"month_volume"`
	MonthPnL         float64 `json:"month_pnl"`

	// Running statistics
	RunningWinRate      float64 `json:"running_win_rate"`
	RunningProfitFactor float64 `json:"running_profit_factor"`
	CurrentDrawdown     float64 `json:"current_drawdown"`
	DaysInDrawdown      int     `json:"days_in_drawdown"`

	// Performance trends
	Last30DaysTrend  float64 `json:"last_30_days_trend"`
	Last7DaysTrend   float64 `json:"last_7_days_trend"`
	PerformanceScore float64 `json:"performance_score"`

	// Risk metrics
	CurrentRiskExposure   float64 `json:"current_risk_exposure"`
	RemainingDailyLimit   int     `json:"remaining_daily_limit"`
	RemainingMonthlyLimit int     `json:"remaining_monthly_limit"`

	LastUpdated time.Time `json:"last_updated"`
}

// Helper functions for trade analysis

// CalculateProfitFactor calculates the profit factor (gross profit / gross loss)
func CalculateProfitFactor(grossProfit, grossLoss float64) float64 {
	if grossLoss == 0 {
		return 0
	}
	return grossProfit / grossLoss
}

// CalculateWinRate calculates the win rate percentage
func CalculateWinRate(successfulTrades, totalTrades int64) float64 {
	if totalTrades == 0 {
		return 0
	}
	return (float64(successfulTrades) / float64(totalTrades)) * 100
}

// CalculateSharpeRatio calculates the Sharpe ratio for trading performance
func CalculateSharpeRatio(returns []float64, riskFreeRate float64) float64 {
	if len(returns) < 2 {
		return 0
	}

	// Calculate mean return
	var sumReturns float64
	for _, ret := range returns {
		sumReturns += ret
	}
	meanReturn := sumReturns / float64(len(returns))

	// Calculate standard deviation
	var sumSquaredDiffs float64
	for _, ret := range returns {
		diff := ret - meanReturn
		sumSquaredDiffs += diff * diff
	}
	stdDev := sumSquaredDiffs / float64(len(returns)-1)

	if stdDev == 0 {
		return 0
	}

	return (meanReturn - riskFreeRate) / stdDev
}

// GetTradePerformanceLevel returns a human-readable performance level
func GetTradePerformanceLevel(winRate float64) string {
	switch {
	case winRate >= 80:
		return "Excellent"
	case winRate >= 70:
		return "Very Good"
	case winRate >= 60:
		return "Good"
	case winRate >= 50:
		return "Average"
	case winRate >= 40:
		return "Below Average"
	default:
		return "Poor"
	}
}

// FormatTradeAmount formats trade amounts with proper precision
func FormatTradeAmount(amount float64, decimals int) string {
	format := fmt.Sprintf("%%.%df", decimals)
	return fmt.Sprintf(format, amount)
}

// TradeEventType represents different types of trade events
type TradeEventType string

const (
	EventTradeCreated   TradeEventType = "trade_created"
	EventTradeSubmitted TradeEventType = "trade_submitted"
	EventTradeExecuted  TradeEventType = "trade_executed"
	EventTradeConfirmed TradeEventType = "trade_confirmed"
	EventTradeFailed    TradeEventType = "trade_failed"
	EventTradeCancelled TradeEventType = "trade_cancelled"
	EventTradeRetried   TradeEventType = "trade_retried"
)

// TradeEvent represents an event in the trade lifecycle
type TradeEvent struct {
	BaseModel
	TradeID   uint                   `gorm:"not null;index" json:"trade_id"`
	Type      TradeEventType         `gorm:"type:varchar(30);not null" json:"type"`
	Status    TradeStatus            `gorm:"type:varchar(20)" json:"status,omitempty"`
	Message   string                 `json:"message,omitempty"`
	ErrorCode string                 `json:"error_code,omitempty"`
	Metadata  map[string]interface{} `gorm:"type:json" json:"metadata,omitempty"`
	Timestamp time.Time              `gorm:"not null" json:"timestamp"`
}

// BeforeCreate sets the timestamp for trade events
func (te *TradeEvent) BeforeCreate(tx *gorm.DB) error {
	if te.Timestamp.IsZero() {
		te.Timestamp = time.Now()
	}
	return nil
}
