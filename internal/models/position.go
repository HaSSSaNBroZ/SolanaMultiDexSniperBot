// Package models - Position model and related structures
package models

import (
	"database/sql/driver"
	"encoding/json"
	"fmt"
	"time"

	"gorm.io/gorm"
)

// Position represents an open trading position
type Position struct {
	BaseModel

	// User and Token References
	UserID       uint   `gorm:"not null;index" json:"user_id"`
	User         User   `gorm:"foreignKey:UserID" json:"user,omitempty"`
	TokenAddress string `gorm:"not null;index" json:"token_address"`
	Token        Token  `gorm:"foreignKey:TokenAddress;references:Address" json:"token,omitempty"`

	// Position Details
	Status   PositionStatus `gorm:"type:varchar(20);default:'open'" json:"status"`
	Side     PositionSide   `gorm:"type:varchar(10);not null" json:"side"`
	Strategy string         `json:"strategy,omitempty"`

	// Entry Information
	EntryPrice    float64   `gorm:"type:decimal(30,18);not null" json:"entry_price"`
	EntryQuantity float64   `gorm:"type:decimal(30,18);not null" json:"entry_quantity"`
	EntryValue    float64   `gorm:"type:decimal(30,18);not null" json:"entry_value"`
	EntryFees     float64   `gorm:"type:decimal(20,8);default:0" json:"entry_fees"`
	EntryTime     time.Time `gorm:"not null" json:"entry_time"`
	EntryTradeID  uint      `gorm:"index" json:"entry_trade_id"`
	EntryTxHash   string    `json:"entry_tx_hash,omitempty"`

	// Current State
	CurrentPrice    float64   `gorm:"type:decimal(30,18)" json:"current_price"`
	CurrentQuantity float64   `gorm:"type:decimal(30,18)" json:"current_quantity"`
	CurrentValue    float64   `gorm:"type:decimal(30,18)" json:"current_value"`
	LastPriceUpdate time.Time `json:"last_price_update"`

	// Exit Information (for closed positions)
	ExitPrice    *float64   `gorm:"type:decimal(30,18)" json:"exit_price,omitempty"`
	ExitQuantity *float64   `gorm:"type:decimal(30,18)" json:"exit_quantity,omitempty"`
	ExitValue    *float64   `gorm:"type:decimal(30,18)" json:"exit_value,omitempty"`
	ExitFees     *float64   `gorm:"type:decimal(20,8)" json:"exit_fees,omitempty"`
	ExitTime     *time.Time `json:"exit_time,omitempty"`
	ExitTradeID  *uint      `json:"exit_trade_id,omitempty"`
	ExitTxHash   *string    `json:"exit_tx_hash,omitempty"`
	ExitReason   string     `json:"exit_reason,omitempty"`

	// Risk Management
	// Risk Management
	StopLoss          *float64 `gorm:"type:decimal(30,18)" json:"stop_loss,omitempty"`
	TakeProfit        *float64 `gorm:"type:decimal(30,18)" json:"take_profit,omitempty"`
	TrailingStop      *float64 `gorm:"type:decimal(5,2)" json:"trailing_stop,omitempty"`
	TrailingStopPrice *float64 `gorm:"type:decimal(30,18)" json:"trailing_stop_price,omitempty"`
	MaxLoss           *float64 `gorm:"type:decimal(30,18)" json:"max_loss,omitempty"`
	MaxGain           *float64 `gorm:"type:decimal(30,18)" json:"max_gain,omitempty"`

	// Performance Metrics
	UnrealizedPnL        float64 `gorm:"type:decimal(20,8);default:0" json:"unrealized_pnl"`
	UnrealizedPnLPercent float64 `gorm:"type:decimal(10,4);default:0" json:"unrealized_pnl_percent"`
	RealizedPnL          float64 `gorm:"type:decimal(20,8);default:0" json:"realized_pnl"`
	RealizedPnLPercent   float64 `gorm:"type:decimal(10,4);default:0" json:"realized_pnl_percent"`
	TotalFees            float64 `gorm:"type:decimal(20,8);default:0" json:"total_fees"`
	ROI                  float64 `gorm:"type:decimal(10,4);default:0" json:"roi"`

	// Performance Tracking
	HighestValue   float64 `gorm:"type:decimal(30,18)" json:"highest_value"`
	LowestValue    float64 `gorm:"type:decimal(30,18)" json:"lowest_value"`
	HighestPrice   float64 `gorm:"type:decimal(30,18)" json:"highest_price"`
	LowestPrice    float64 `gorm:"type:decimal(30,18)" json:"lowest_price"`
	MaxDrawdown    float64 `gorm:"type:decimal(5,2);default:0" json:"max_drawdown"`
	MaxGainPercent float64 `gorm:"type:decimal(10,4);default:0" json:"max_gain_percent"`

	// Position Management
	PartialExits []PartialExit        `gorm:"type:json" json:"partial_exits,omitempty"`
	Adjustments  []PositionAdjustment `gorm:"type:json" json:"adjustments,omitempty"`
	DCAEntries   []DCAEntry           `gorm:"type:json" json:"dca_entries,omitempty"`

	// Market Context
	MarketCondition   string  `json:"market_condition,omitempty"`
	VolatilityAtEntry float64 `gorm:"type:decimal(5,2)" json:"volatility_at_entry"`
	LiquidityAtEntry  float64 `gorm:"type:decimal(20,8)" json:"liquidity_at_entry"`

	// Automation Settings
	AutoExitEnabled  bool `gorm:"default:false" json:"auto_exit_enabled"`
	AutoAdjustment   bool `gorm:"default:false" json:"auto_adjustment"`
	RebalanceEnabled bool `gorm:"default:false" json:"rebalance_enabled"`

	// Duration and Timing
	HoldDuration   time.Duration  `json:"hold_duration"`
	TargetHoldTime *time.Duration `json:"target_hold_time,omitempty"`
	MaxHoldTime    *time.Duration `json:"max_hold_time,omitempty"`

	// Risk Scoring
	RiskScore float64 `gorm:"type:decimal(3,1)" json:"risk_score"`
	RiskLevel string  `json:"risk_level,omitempty"`

	// Metadata
	Notes  string   `json:"notes,omitempty"`
	Tags   []string `gorm:"type:json" json:"tags,omitempty"`
	Source string   `json:"source,omitempty"` // manual, auto, signal

	// Relationships
	Trades []Trade `gorm:"foreignKey:PositionID" json:"trades,omitempty"`
}

// PositionStatus represents the status of a position
type PositionStatus string

const (
	PositionStatusOpen       PositionStatus = "open"
	PositionStatusClosed     PositionStatus = "closed"
	PositionStatusPartial    PositionStatus = "partial"
	PositionStatusStopped    PositionStatus = "stopped"
	PositionStatusExpired    PositionStatus = "expired"
	PositionStatusLiquidated PositionStatus = "liquidated"
)

// PositionSide represents whether it's a long or short position
type PositionSide string

const (
	PositionSideLong  PositionSide = "long"
	PositionSideShort PositionSide = "short"
)

// PartialExit represents a partial exit from the position
type PartialExit struct {
	Timestamp  time.Time `json:"timestamp"`
	Quantity   float64   `json:"quantity"`
	Price      float64   `json:"price"`
	Value      float64   `json:"value"`
	Fees       float64   `json:"fees"`
	PnL        float64   `json:"pnl"`
	PnLPercent float64   `json:"pnl_percent"`
	Reason     string    `json:"reason"`
	TradeID    uint      `json:"trade_id"`
	TxHash     string    `json:"tx_hash,omitempty"`
}

// PositionAdjustment represents adjustments made to the position
type PositionAdjustment struct {
	Timestamp    time.Time `json:"timestamp"`
	Type         string    `json:"type"` // stop_loss, take_profit, trailing_stop
	OldValue     *float64  `json:"old_value,omitempty"`
	NewValue     float64   `json:"new_value"`
	Reason       string    `json:"reason"`
	Automatic    bool      `json:"automatic"`
	TriggerPrice *float64  `json:"trigger_price,omitempty"`
}

// DCAEntry represents a Dollar Cost Averaging entry
type DCAEntry struct {
	Timestamp time.Time `json:"timestamp"`
	Quantity  float64   `json:"quantity"`
	Price     float64   `json:"price"`
	Value     float64   `json:"value"`
	Fees      float64   `json:"fees"`
	TradeID   uint      `json:"trade_id"`
	TxHash    string    `json:"tx_hash,omitempty"`
	Strategy  string    `json:"strategy"` // time_based, price_based, volatility_based
}

// GORM Hooks

// BeforeCreate is called before creating a position
func (p *Position) BeforeCreate(tx *gorm.DB) error {
	if p.EntryTime.IsZero() {
		p.EntryTime = time.Now()
	}

	// Initialize current values with entry values
	p.CurrentPrice = p.EntryPrice
	p.CurrentQuantity = p.EntryQuantity
	p.CurrentValue = p.EntryValue
	p.LastPriceUpdate = p.EntryTime

	// Initialize tracking values
	p.HighestValue = p.EntryValue
	p.LowestValue = p.EntryValue
	p.HighestPrice = p.EntryPrice
	p.LowestPrice = p.EntryPrice

	// Initialize fees
	p.TotalFees = p.EntryFees

	return nil
}

// BeforeUpdate is called before updating a position
func (p *Position) BeforeUpdate(tx *gorm.DB) error {
	// Update hold duration
	if p.Status == PositionStatusOpen {
		p.HoldDuration = time.Since(p.EntryTime)
	} else if p.ExitTime != nil {
		p.HoldDuration = p.ExitTime.Sub(p.EntryTime)
	}

	// Update performance metrics
	p.updatePerformanceMetrics()

	// Update tracking values
	p.updateTrackingValues()

	// Calculate total fees
	p.calculateTotalFees()

	return nil
}

// AfterUpdate is called after updating a position
func (p *Position) AfterUpdate(tx *gorm.DB) error {
	// Update user statistics when position is closed
	if p.Status == PositionStatusClosed {
		return p.updateUserStats(tx)
	}

	return nil
}

// Position Methods

// updatePerformanceMetrics updates PnL and ROI calculations
func (p *Position) updatePerformanceMetrics() {
	if p.Status == PositionStatusOpen {
		// Calculate unrealized PnL
		p.UnrealizedPnL = (p.CurrentPrice - p.EntryPrice) * p.CurrentQuantity
		if p.EntryValue != 0 {
			p.UnrealizedPnLPercent = (p.UnrealizedPnL / p.EntryValue) * 100
		}

		// Calculate current ROI
		if p.EntryValue != 0 {
			p.ROI = ((p.CurrentValue - p.EntryValue - p.TotalFees) / p.EntryValue) * 100
		}
	} else if p.Status == PositionStatusClosed && p.ExitValue != nil {
		// Calculate realized PnL
		p.RealizedPnL = *p.ExitValue - p.EntryValue - p.TotalFees
		if p.EntryValue != 0 {
			p.RealizedPnLPercent = (p.RealizedPnL / p.EntryValue) * 100
		}

		// Calculate final ROI
		if p.EntryValue != 0 {
			p.ROI = (p.RealizedPnL / p.EntryValue) * 100
		}
	}
}

// updateTrackingValues updates highest/lowest values and prices
func (p *Position) updateTrackingValues() {
	// Update highest values
	if p.CurrentValue > p.HighestValue {
		p.HighestValue = p.CurrentValue
	}
	if p.CurrentPrice > p.HighestPrice {
		p.HighestPrice = p.CurrentPrice
	}

	// Update lowest values
	if p.CurrentValue < p.LowestValue {
		p.LowestValue = p.CurrentValue
	}
	if p.CurrentPrice < p.LowestPrice {
		p.LowestPrice = p.CurrentPrice
	}

	// Calculate max drawdown from highest value
	if p.HighestValue > 0 {
		drawdown := ((p.HighestValue - p.CurrentValue) / p.HighestValue) * 100
		if drawdown > p.MaxDrawdown {
			p.MaxDrawdown = drawdown
		}
	}

	// Calculate max gain percent from entry
	if p.EntryValue > 0 {
		maxGain := ((p.HighestValue - p.EntryValue) / p.EntryValue) * 100
		if maxGain > p.MaxGainPercent {
			p.MaxGainPercent = maxGain
		}
	}
}

// calculateTotalFees calculates total fees including exit fees
func (p *Position) calculateTotalFees() {
	p.TotalFees = p.EntryFees

	if p.ExitFees != nil {
		p.TotalFees += *p.ExitFees
	}

	// Add partial exit fees
	for _, exit := range p.PartialExits {
		p.TotalFees += exit.Fees
	}

	// Add DCA entry fees
	for _, entry := range p.DCAEntries {
		p.TotalFees += entry.Fees
	}
}

// updateUserStats updates user statistics when position is closed
func (p *Position) updateUserStats(tx *gorm.DB) error {
	updates := map[string]interface{}{
		"total_volume": gorm.Expr("total_volume + ?", p.EntryValue),
	}

	if p.RealizedPnL > 0 {
		updates["total_profit"] = gorm.Expr("total_profit + ?", p.RealizedPnL)
	} else if p.RealizedPnL < 0 {
		updates["total_loss"] = gorm.Expr("total_loss + ?", -p.RealizedPnL)
	}

	return tx.Model(&User{}).Where("id = ?", p.UserID).Updates(updates).Error
}

// UpdateCurrentPrice updates the current price and related calculations
func (p *Position) UpdateCurrentPrice(price float64) {
	p.CurrentPrice = price
	p.CurrentValue = price * p.CurrentQuantity
	p.LastPriceUpdate = time.Now()

	// Update performance metrics
	p.updatePerformanceMetrics()
	p.updateTrackingValues()
}

// ShouldTriggerStopLoss checks if stop loss should be triggered
func (p *Position) ShouldTriggerStopLoss() bool {
	if p.StopLoss == nil || p.Status != PositionStatusOpen {
		return false
	}

	switch p.Side {
	case PositionSideLong:
		return p.CurrentPrice <= *p.StopLoss
	case PositionSideShort:
		return p.CurrentPrice >= *p.StopLoss
	default:
		return false
	}
}

// ShouldTriggerTakeProfit checks if take profit should be triggered
func (p *Position) ShouldTriggerTakeProfit() bool {
	if p.TakeProfit == nil || p.Status != PositionStatusOpen {
		return false
	}

	switch p.Side {
	case PositionSideLong:
		return p.CurrentPrice >= *p.TakeProfit
	case PositionSideShort:
		return p.CurrentPrice <= *p.TakeProfit
	default:
		return false
	}
}

// UpdateTrailingStop updates trailing stop based on current price
func (p *Position) UpdateTrailingStop() {
	if p.TrailingStop == nil || p.Status != PositionStatusOpen {
		return
	}

	trailingPercent := *p.TrailingStop / 100

	switch p.Side {
	case PositionSideLong:
		// For long positions, trailing stop moves up with price
		newStopPrice := p.CurrentPrice * (1 - trailingPercent)
		if p.TrailingStopPrice == nil || newStopPrice > *p.TrailingStopPrice {
			p.TrailingStopPrice = &newStopPrice
		}
	case PositionSideShort:
		// For short positions, trailing stop moves down with price
		newStopPrice := p.CurrentPrice * (1 + trailingPercent)
		if p.TrailingStopPrice == nil || newStopPrice < *p.TrailingStopPrice {
			p.TrailingStopPrice = &newStopPrice
		}
	}
}

// ShouldTriggerTrailingStop checks if trailing stop should be triggered
func (p *Position) ShouldTriggerTrailingStop() bool {
	if p.TrailingStopPrice == nil || p.Status != PositionStatusOpen {
		return false
	}

	switch p.Side {
	case PositionSideLong:
		return p.CurrentPrice <= *p.TrailingStopPrice
	case PositionSideShort:
		return p.CurrentPrice >= *p.TrailingStopPrice
	default:
		return false
	}
}

// AddPartialExit adds a partial exit to the position
func (p *Position) AddPartialExit(exit PartialExit) {
	p.PartialExits = append(p.PartialExits, exit)

	// Update current quantity
	p.CurrentQuantity -= exit.Quantity
	p.CurrentValue = p.CurrentPrice * p.CurrentQuantity

	// Add to realized PnL
	p.RealizedPnL += exit.PnL

	// Check if position should be marked as partial or closed
	if p.CurrentQuantity <= 0 {
		p.Status = PositionStatusClosed
		p.CurrentQuantity = 0
		p.CurrentValue = 0
	} else if len(p.PartialExits) > 0 {
		p.Status = PositionStatusPartial
	}
}

// AddDCAEntry adds a DCA entry to the position
func (p *Position) AddDCAEntry(entry DCAEntry) {
	p.DCAEntries = append(p.DCAEntries, entry)

	// Update average entry price
	totalValue := p.EntryValue + entry.Value
	totalQuantity := p.EntryQuantity + entry.Quantity

	p.EntryPrice = totalValue / totalQuantity
	p.EntryQuantity = totalQuantity
	p.EntryValue = totalValue
	p.CurrentQuantity = totalQuantity

	// Update fees
	p.EntryFees += entry.Fees
}

// AddAdjustment adds a position adjustment record
func (p *Position) AddAdjustment(adjustment PositionAdjustment) {
	p.Adjustments = append(p.Adjustments, adjustment)

	// Apply the adjustment
	switch adjustment.Type {
	case "stop_loss":
		p.StopLoss = &adjustment.NewValue
	case "take_profit":
		p.TakeProfit = &adjustment.NewValue
	case "trailing_stop":
		p.TrailingStop = &adjustment.NewValue
	}
}

// GetCurrentPnL returns current profit/loss
func (p *Position) GetCurrentPnL() float64 {
	if p.Status == PositionStatusOpen {
		return p.UnrealizedPnL
	}
	return p.RealizedPnL
}

// GetCurrentPnLPercent returns current profit/loss percentage
func (p *Position) GetCurrentPnLPercent() float64 {
	if p.Status == PositionStatusOpen {
		return p.UnrealizedPnLPercent
	}
	return p.RealizedPnLPercent
}

// IsProfit checks if the position is currently profitable
func (p *Position) IsProfit() bool {
	return p.GetCurrentPnL() > 0
}

// GetRiskExposure calculates current risk exposure
func (p *Position) GetRiskExposure() float64 {
	if p.Status != PositionStatusOpen {
		return 0
	}

	if p.StopLoss != nil {
		stopLossValue := *p.StopLoss * p.CurrentQuantity
		return p.CurrentValue - stopLossValue
	}

	// If no stop loss, exposure is the full position value
	return p.CurrentValue
}

// GetHoldDurationString returns formatted hold duration
func (p *Position) GetHoldDurationString() string {
	duration := p.HoldDuration
	if p.Status == PositionStatusOpen {
		duration = time.Since(p.EntryTime)
	}

	days := int(duration.Hours()) / 24
	hours := int(duration.Hours()) % 24
	minutes := int(duration.Minutes()) % 60

	if days > 0 {
		return fmt.Sprintf("%dd %dh %dm", days, hours, minutes)
	} else if hours > 0 {
		return fmt.Sprintf("%dh %dm", hours, minutes)
	} else {
		return fmt.Sprintf("%dm", minutes)
	}
}

// Custom JSON marshaling and database scanning

// Scan implements the sql.Scanner interface for PartialExits
func (pe *[]PartialExit) Scan(value interface{}) error {
	if value == nil {
		*pe = []PartialExit{}
		return nil
	}

	bytes, ok := value.([]byte)
	if !ok {
		return fmt.Errorf("cannot scan %T into []PartialExit", value)
	}

	return json.Unmarshal(bytes, pe)
}

// Value implements the driver.Valuer interface for PartialExits
func (pe []PartialExit) Value() (driver.Value, error) {
	return json.Marshal(pe)
}

// Scan implements the sql.Scanner interface for PositionAdjustments
func (pa *[]PositionAdjustment) Scan(value interface{}) error {
	if value == nil {
		*pa = []PositionAdjustment{}
		return nil
	}

	bytes, ok := value.([]byte)
	if !ok {
		return fmt.Errorf("cannot scan %T into []PositionAdjustment", value)
	}

	return json.Unmarshal(bytes, pa)
}

// Value implements the driver.Valuer interface for PositionAdjustments
func (pa []PositionAdjustment) Value() (driver.Value, error) {
	return json.Marshal(pa)
}

// Scan implements the sql.Scanner interface for DCAEntries
func (dca *[]DCAEntry) Scan(value interface{}) error {
	if value == nil {
		*dca = []DCAEntry{}
		return nil
	}

	bytes, ok := value.([]byte)
	if !ok {
		return fmt.Errorf("cannot scan %T into []DCAEntry", value)
	}

	return json.Unmarshal(bytes, dca)
}

// Value implements the driver.Valuer interface for DCAEntries
func (dca []DCAEntry) Value() (driver.Value, error) {
	return json.Marshal(dca)
}

// Repository and Service Interfaces

// PositionRepository interface (to be implemented in Phase 2)
type PositionRepository interface {
	Create(position *Position) error
	GetByID(id uint) (*Position, error)
	GetByUserID(userID uint, status []PositionStatus) ([]*Position, error)
	GetOpenPositions(userID uint) ([]*Position, error)
	GetClosedPositions(userID uint, limit, offset int) ([]*Position, error)
	Update(position *Position) error
	Close(positionID uint, exitPrice float64, exitReason string) error
	GetPositionsByToken(tokenAddress string) ([]*Position, error)
	GetUserPositionStats(userID uint) (*PositionStatistics, error)
	BulkUpdatePrices(priceUpdates map[string]float64) error
}

// PositionService interface (to be implemented in later phases)
type PositionService interface {
	OpenPosition(request *OpenPositionRequest) (*Position, error)
	ClosePosition(positionID uint, reason string) error
	PartialClose(positionID uint, quantity float64, reason string) error
	UpdateStopLoss(positionID uint, stopLoss float64) error
	UpdateTakeProfit(positionID uint, takeProfit float64) error
	UpdateTrailingStop(positionID uint, trailingStop float64) error
	AddDCAEntry(positionID uint, quantity float64, price float64) error
	UpdatePositionPrices(tokenPrices map[string]float64) error
	GetUserPositions(userID uint, filters *PositionFilters) ([]*Position, error)
	GetPositionAnalysis(positionID uint) (*PositionAnalysis, error)
	CheckStopLossAndTakeProfit() error
	AutoRebalancePositions(userID uint) error
}

// OpenPositionRequest represents a request to open a new position
type OpenPositionRequest struct {
	UserID       uint           `json:"user_id" validate:"required"`
	TokenAddress string         `json:"token_address" validate:"required"`
	Side         PositionSide   `json:"side" validate:"required"`
	Quantity     float64        `json:"quantity" validate:"required,gt=0"`
	Price        float64        `json:"price" validate:"required,gt=0"`
	StopLoss     *float64       `json:"stop_loss,omitempty"`
	TakeProfit   *float64       `json:"take_profit,omitempty"`
	TrailingStop *float64       `json:"trailing_stop,omitempty"`
	Strategy     string         `json:"strategy,omitempty"`
	MaxHoldTime  *time.Duration `json:"max_hold_time,omitempty"`
	Notes        string         `json:"notes,omitempty"`
	Tags         []string       `json:"tags,omitempty"`
	Source       string         `json:"source,omitempty"`
}

// PositionFilters represents filters for querying positions
type PositionFilters struct {
	Status       []PositionStatus `json:"status,omitempty"`
	Side         []PositionSide   `json:"side,omitempty"`
	TokenAddress string           `json:"token_address,omitempty"`
	Strategy     []string         `json:"strategy,omitempty"`
	Tags         []string         `json:"tags,omitempty"`
	DateFrom     *time.Time       `json:"date_from,omitempty"`
	DateTo       *time.Time       `json:"date_to,omitempty"`
	MinValue     *float64         `json:"min_value,omitempty"`
	MaxValue     *float64         `json:"max_value,omitempty"`
	Profitable   *bool            `json:"profitable,omitempty"`
	MinHoldTime  *time.Duration   `json:"min_hold_time,omitempty"`
	MaxHoldTime  *time.Duration   `json:"max_hold_time,omitempty"`
	SortBy       string           `json:"sort_by,omitempty"`
	SortOrder    string           `json:"sort_order,omitempty"`
	Limit        int              `json:"limit,omitempty"`
	Offset       int              `json:"offset,omitempty"`
}

// PositionStatistics represents aggregated position statistics
type PositionStatistics struct {
	TotalPositions      int64   `json:"total_positions"`
	OpenPositions       int64   `json:"open_positions"`
	ClosedPositions     int64   `json:"closed_positions"`
	ProfitablePositions int64   `json:"profitable_positions"`
	WinRate             float64 `json:"win_rate"`

	TotalValue      float64 `json:"total_value"`
	TotalPnL        float64 `json:"total_pnl"`
	TotalFees       float64 `json:"total_fees"`
	AverageHoldTime float64 `json:"average_hold_time_hours"`

	LargestWin  float64 `json:"largest_win"`
	LargestLoss float64 `json:"largest_loss"`
	AverageWin  float64 `json:"average_win"`
	AverageLoss float64 `json:"average_loss"`

	MaxDrawdown          float64 `json:"max_drawdown"`
	BestPerformingToken  string  `json:"best_performing_token,omitempty"`
	WorstPerformingToken string  `json:"worst_performing_token,omitempty"`

	RiskExposure   float64 `json:"current_risk_exposure"`
	PortfolioValue float64 `json:"portfolio_value"`

	LastUpdated time.Time `json:"last_updated"`
}

// PositionAnalysis represents detailed position analysis
type PositionAnalysis struct {
	Position           *Position          `json:"position"`
	PerformanceMetrics PerformanceMetrics `json:"performance_metrics"`
	RiskMetrics        RiskMetrics        `json:"risk_metrics"`
	TimeAnalysis       TimeAnalysis       `json:"time_analysis"`
	ComparisonData     ComparisonData     `json:"comparison_data"`
	Recommendations    []string           `json:"recommendations"`
	AnalyzedAt         time.Time          `json:"analyzed_at"`
}

// PerformanceMetrics represents performance analysis
type PerformanceMetrics struct {
	ROI              float64 `json:"roi"`
	AnnualizedReturn float64 `json:"annualized_return"`
	SharpeRatio      float64 `json:"sharpe_ratio"`
	MaxDrawdown      float64 `json:"max_drawdown"`
	WinRate          float64 `json:"win_rate"`
	ProfitFactor     float64 `json:"profit_factor"`
	RecoveryFactor   float64 `json:"recovery_factor"`
	Volatility       float64 `json:"volatility"`
}

// RiskMetrics represents risk analysis
type RiskMetrics struct {
	ValueAtRisk       float64 `json:"value_at_risk"`
	RiskExposure      float64 `json:"risk_exposure"`
	CorrelationRisk   float64 `json:"correlation_risk"`
	LiquidityRisk     float64 `json:"liquidity_risk"`
	ConcentrationRisk float64 `json:"concentration_risk"`
	OverallRiskScore  float64 `json:"overall_risk_score"`
}

// TimeAnalysis represents time-based analysis
type TimeAnalysis struct {
	HoldDuration    time.Duration `json:"hold_duration"`
	OptimalHoldTime time.Duration `json:"optimal_hold_time"`
	TimeInProfit    time.Duration `json:"time_in_profit"`
	TimeInLoss      time.Duration `json:"time_in_loss"`
	BestEntryTime   string        `json:"best_entry_time"`
	WorstEntryTime  string        `json:"worst_entry_time"`
}

// ComparisonData represents comparison with benchmarks
type ComparisonData struct {
	VsMarket      float64 `json:"vs_market"`
	VsToken       float64 `json:"vs_token"`
	VsUserAverage float64 `json:"vs_user_average"`
	Percentile    float64 `json:"percentile"`
}

// Helper functions

// CalculatePositionSize calculates optimal position size based on risk
func CalculatePositionSize(accountValue, riskPercent, entryPrice, stopLoss float64) float64 {
	if stopLoss >= entryPrice || entryPrice <= 0 || stopLoss <= 0 {
		return 0
	}

	riskAmount := accountValue * (riskPercent / 100)
	riskPerShare := entryPrice - stopLoss

	if riskPerShare <= 0 {
		return 0
	}

	return riskAmount / riskPerShare
}

// CalculateStopLossLevel calculates stop loss based on percentage
func CalculateStopLossLevel(entryPrice float64, stopLossPercent float64, side PositionSide) float64 {
	switch side {
	case PositionSideLong:
		return entryPrice * (1 - stopLossPercent/100)
	case PositionSideShort:
		return entryPrice * (1 + stopLossPercent/100)
	default:
		return entryPrice
	}
}

// CalculateTakeProfitLevel calculates take profit based on percentage
func CalculateTakeProfitLevel(entryPrice float64, takeProfitPercent float64, side PositionSide) float64 {
	switch side {
	case PositionSideLong:
		return entryPrice * (1 + takeProfitPercent/100)
	case PositionSideShort:
		return entryPrice * (1 - takeProfitPercent/100)
	default:
		return entryPrice
	}
}

// GetPositionHealthStatus returns health status based on various metrics
func GetPositionHealthStatus(position *Position) string {
	if position.Status != PositionStatusOpen {
		return "closed"
	}

	currentPnLPercent := position.GetCurrentPnLPercent()

	switch {
	case currentPnLPercent >= 20:
		return "excellent"
	case currentPnLPercent >= 10:
		return "very_good"
	case currentPnLPercent >= 5:
		return "good"
	case currentPnLPercent >= 0:
		return "neutral"
	case currentPnLPercent >= -10:
		return "concerning"
	case currentPnLPercent >= -20:
		return "poor"
	default:
		return "critical"
	}
}

// FormatPositionValue formats position values with proper precision
func FormatPositionValue(value float64) string {
	if value >= 1000000 {
		return fmt.Sprintf("%.2fM", value/1000000)
	} else if value >= 1000 {
		return fmt.Sprintf("%.2fK", value/1000)
	} else {
		return fmt.Sprintf("%.4f", value)
	}
}

// CalculateRiskRewardRatio calculates risk-reward ratio for a position
func CalculateRiskRewardRatio(entryPrice, stopLoss, takeProfit float64, side PositionSide) float64 {
	var risk, reward float64

	switch side {
	case PositionSideLong:
		risk = entryPrice - stopLoss
		reward = takeProfit - entryPrice
	case PositionSideShort:
		risk = stopLoss - entryPrice
		reward = entryPrice - takeProfit
	default:
		return 0
	}

	if risk <= 0 {
		return 0
	}

	return reward / risk
}

// EstimateOptimalHoldTime estimates optimal hold time based on position metrics
func EstimateOptimalHoldTime(position *Position) time.Duration {
	// This is a simplified estimation - in practice, this would use
	// machine learning models and historical data analysis

	volatility := position.VolatilityAtEntry

	switch {
	case volatility > 80:
		return 15 * time.Minute // High volatility - short term
	case volatility > 60:
		return 1 * time.Hour // Medium-high volatility
	case volatility > 40:
		return 4 * time.Hour // Medium volatility
	case volatility > 20:
		return 24 * time.Hour // Low-medium volatility
	default:
		return 7 * 24 * time.Hour // Low volatility - longer term
	}
}
