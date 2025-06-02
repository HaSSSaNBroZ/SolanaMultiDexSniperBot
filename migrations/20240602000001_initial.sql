-- Initial database schema for Solana Sniper Bot
-- Created: 2024-06-02
-- Author: Hassan Hafedh Ubaid

-- Enable UUID extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Enable pgcrypto for encryption functions
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- Create enum types
CREATE TYPE scenario_mode AS ENUM ('development', 'production', 'simulation');
CREATE TYPE trade_status AS ENUM ('pending', 'executed', 'failed', 'cancelled');
CREATE TYPE trade_side AS ENUM ('buy', 'sell');
CREATE TYPE risk_level AS ENUM ('low', 'medium', 'high', 'critical');
CREATE TYPE component_status AS ENUM ('healthy', 'degraded', 'unhealthy', 'starting');

-- ============================================================================
-- CORE TABLES
-- ============================================================================

-- Sessions table for tracking trading sessions
CREATE TABLE sessions (
                          id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
                          name VARCHAR(255) NOT NULL,
                          description TEXT,
                          mode scenario_mode NOT NULL DEFAULT 'development',
                          config_snapshot JSONB NOT NULL,
                          started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                          ended_at TIMESTAMPTZ,
                          total_trades INTEGER DEFAULT 0,
                          profitable_trades INTEGER DEFAULT 0,
                          total_pnl DECIMAL(20, 8) DEFAULT 0,
                          max_drawdown DECIMAL(10, 4) DEFAULT 0,
                          is_active BOOLEAN DEFAULT true,
                          created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                          updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Wallets table for managing trading wallets
CREATE TABLE wallets (
                         id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
                         name VARCHAR(100) NOT NULL,
                         address VARCHAR(44) NOT NULL UNIQUE,
                         encrypted_private_key TEXT NOT NULL,
                         is_active BOOLEAN DEFAULT true,
                         balance_sol DECIMAL(20, 8) DEFAULT 0,
                         last_balance_update TIMESTAMPTZ,
                         created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                         updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Tokens table for tracking discovered tokens
CREATE TABLE tokens (
                        id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
                        address VARCHAR(44) NOT NULL UNIQUE,
                        symbol VARCHAR(20),
                        name VARCHAR(100),
                        decimals INTEGER NOT NULL DEFAULT 9,
                        total_supply DECIMAL(30, 0),
                        creator_address VARCHAR(44),
                        program_id VARCHAR(44),
                        market_cap_usd DECIMAL(20, 2),
                        liquidity_sol DECIMAL(20, 8),
                        holder_count INTEGER DEFAULT 0,
                        age_seconds INTEGER DEFAULT 0,
                        first_detected_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                        last_updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                        is_verified BOOLEAN DEFAULT false,
                        is_blacklisted BOOLEAN DEFAULT false,
                        metadata JSONB,
                        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Trades table for recording all trading activity
CREATE TABLE trades (
                        id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
                        session_id UUID NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
                        wallet_id UUID NOT NULL REFERENCES wallets(id),
                        token_id UUID NOT NULL REFERENCES tokens(id),
                        side trade_side NOT NULL,
                        status trade_status NOT NULL DEFAULT 'pending',
                        amount_sol DECIMAL(20, 8) NOT NULL,
                        amount_tokens DECIMAL(30, 8),
                        price_per_token DECIMAL(20, 12),
                        slippage_percent DECIMAL(5, 2),
                        gas_fee_sol DECIMAL(20, 8),
                        transaction_signature VARCHAR(88),
                        dex_used VARCHAR(50),
                        risk_score INTEGER CHECK (risk_score >= 1 AND risk_score <= 10),
                        execution_time_ms INTEGER,
                        pnl_sol DECIMAL(20, 8),
                        pnl_percent DECIMAL(10, 4),
                        exit_reason VARCHAR(50),
                        executed_at TIMESTAMPTZ,
                        created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                        updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Positions table for tracking open positions
CREATE TABLE positions (
                           id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
                           session_id UUID NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
                           wallet_id UUID NOT NULL REFERENCES wallets(id),
                           token_id UUID NOT NULL REFERENCES tokens(id),
                           entry_trade_id UUID NOT NULL REFERENCES trades(id),
                           exit_trade_id UUID REFERENCES trades(id),
                           quantity DECIMAL(30, 8) NOT NULL,
                           entry_price DECIMAL(20, 12) NOT NULL,
                           exit_price DECIMAL(20, 12),
                           current_price DECIMAL(20, 12),
                           unrealized_pnl_sol DECIMAL(20, 8),
                           realized_pnl_sol DECIMAL(20, 8),
                           stop_loss_price DECIMAL(20, 12),
                           take_profit_price DECIMAL(20, 12),
                           is_open BOOLEAN DEFAULT true,
                           opened_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                           closed_at TIMESTAMPTZ,
                           created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                           updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Risk assessments table
CREATE TABLE risk_assessments (
                                  id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
                                  token_id UUID NOT NULL REFERENCES tokens(id),
                                  overall_score INTEGER NOT NULL CHECK (overall_score >= 1 AND overall_score <= 10),
                                  liquidity_score INTEGER CHECK (liquidity_score >= 1 AND liquidity_score <= 10),
                                  holder_score INTEGER CHECK (holder_score >= 1 AND holder_score <= 10),
                                  contract_score INTEGER CHECK (contract_score >= 1 AND contract_score <= 10),
                                  honeypot_detected BOOLEAN DEFAULT false,
                                  honeypot_confidence DECIMAL(3, 2),
                                  rug_pull_risk BOOLEAN DEFAULT false,
                                  whale_concentration DECIMAL(5, 2),
                                  assessment_details JSONB,
                                  assessed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                                  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================================
-- MONITORING & ANALYTICS TABLES
-- ============================================================================

-- Health checks table
CREATE TABLE health_checks (
                               id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
                               component_name VARCHAR(100) NOT NULL,
                               status component_status NOT NULL,
                               message TEXT,
                               response_time_ms INTEGER,
                               checked_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                               created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Metrics table for storing performance metrics
CREATE TABLE metrics (
                         id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
                         metric_name VARCHAR(100) NOT NULL,
                         metric_value DECIMAL(20, 8) NOT NULL,
                         labels JSONB,
                         session_id UUID REFERENCES sessions(id),
                         recorded_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                         created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Error logs table
CREATE TABLE error_logs (
                            id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
                            error_type VARCHAR(100) NOT NULL,
                            error_message TEXT NOT NULL,
                            component VARCHAR(100),
                            session_id UUID REFERENCES sessions(id),
                            stack_trace TEXT,
                            context JSONB,
                            severity VARCHAR(20) DEFAULT 'medium',
                            resolved BOOLEAN DEFAULT false,
                            occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Audit logs table
CREATE TABLE audit_logs (
                            id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
                            user_id VARCHAR(100),
                            action VARCHAR(100) NOT NULL,
                            resource_type VARCHAR(50),
                            resource_id VARCHAR(100),
                            details JSONB,
                            ip_address INET,
                            user_agent TEXT,
                            session_id UUID REFERENCES sessions(id),
                            performed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Configuration changes table
CREATE TABLE config_changes (
                                id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
                                session_id UUID REFERENCES sessions(id),
                                field_path VARCHAR(255) NOT NULL,
                                old_value TEXT,
                                new_value TEXT,
                                changed_by VARCHAR(100),
                                reason TEXT,
                                changed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================================
-- TELEGRAM BOT TABLES
-- ============================================================================

-- Telegram users table
CREATE TABLE telegram_users (
                                id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
                                telegram_id BIGINT NOT NULL UNIQUE,
                                username VARCHAR(100),
                                first_name VARCHAR(100),
                                last_name VARCHAR(100),
                                role VARCHAR(50) DEFAULT 'user',
                                is_active BOOLEAN DEFAULT true,
                                last_seen_at TIMESTAMPTZ,
                                preferences JSONB,
                                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Telegram command logs table
CREATE TABLE telegram_commands (
                                   id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
                                   user_id UUID NOT NULL REFERENCES telegram_users(id),
                                   command VARCHAR(100) NOT NULL,
                                   parameters TEXT,
                                   response_status VARCHAR(20) DEFAULT 'success',
                                   execution_time_ms INTEGER,
                                   session_id UUID REFERENCES sessions(id),
                                   executed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                                   created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================================
-- INDEXES FOR PERFORMANCE
-- ============================================================================

-- Sessions indexes
CREATE INDEX idx_sessions_mode ON sessions(mode);
CREATE INDEX idx_sessions_active ON sessions(is_active);
CREATE INDEX idx_sessions_started_at ON sessions(started_at);

-- Wallets indexes
CREATE INDEX idx_wallets_active ON wallets(is_active);
CREATE INDEX idx_wallets_address ON wallets(address);

-- Tokens indexes
CREATE INDEX idx_tokens_address ON tokens(address);
CREATE INDEX idx_tokens_symbol ON tokens(symbol);
CREATE INDEX idx_tokens_first_detected ON tokens(first_detected_at);
CREATE INDEX idx_tokens_market_cap ON tokens(market_cap_usd);
CREATE INDEX idx_tokens_liquidity ON tokens(liquidity_sol);
CREATE INDEX idx_tokens_blacklisted ON tokens(is_blacklisted);

-- Trades indexes
CREATE INDEX idx_trades_session ON trades(session_id);
CREATE INDEX idx_trades_wallet ON trades(wallet_id);
CREATE INDEX idx_trades_token ON trades(token_id);
CREATE INDEX idx_trades_status ON trades(status);
CREATE INDEX idx_trades_side ON trades(side);
CREATE INDEX idx_trades_executed_at ON trades(executed_at);
CREATE INDEX idx_trades_pnl ON trades(pnl_sol);

-- Positions indexes
CREATE INDEX idx_positions_session ON positions(session_id);
CREATE INDEX idx_positions_wallet ON positions(wallet_id);
CREATE INDEX idx_positions_token ON positions(token_id);
CREATE INDEX idx_positions_open ON positions(is_open);
CREATE INDEX idx_positions_opened_at ON positions(opened_at);

-- Risk assessments indexes
CREATE INDEX idx_risk_token ON risk_assessments(token_id);
CREATE INDEX idx_risk_score ON risk_assessments(overall_score);
CREATE INDEX idx_risk_honeypot ON risk_assessments(honeypot_detected);
CREATE INDEX idx_risk_assessed_at ON risk_assessments(assessed_at);

-- Monitoring indexes
CREATE INDEX idx_health_component ON health_checks(component_name);
CREATE INDEX idx_health_status ON health_checks(status);
CREATE INDEX idx_health_checked_at ON health_checks(checked_at);

CREATE INDEX idx_metrics_name ON metrics(metric_name);
CREATE INDEX idx_metrics_session ON metrics(session_id);
CREATE INDEX idx_metrics_recorded_at ON metrics(recorded_at);

CREATE INDEX idx_error_logs_type ON error_logs(error_type);
CREATE INDEX idx_error_logs_component ON error_logs(component);
CREATE INDEX idx_error_logs_severity ON error_logs(severity);
CREATE INDEX idx_error_logs_resolved ON error_logs(resolved);
CREATE INDEX idx_error_logs_occurred_at ON error_logs(occurred_at);

CREATE INDEX idx_audit_logs_user ON audit_logs(user_id);
CREATE INDEX idx_audit_logs_action ON audit_logs(action);
CREATE INDEX idx_audit_logs_resource ON audit_logs(resource_type, resource_id);
CREATE INDEX idx_audit_logs_performed_at ON audit_logs(performed_at);

-- Telegram indexes
CREATE INDEX idx_telegram_users_telegram_id ON telegram_users(telegram_id);
CREATE INDEX idx_telegram_users_role ON telegram_users(role);
CREATE INDEX idx_telegram_users_active ON telegram_users(is_active);

CREATE INDEX idx_telegram_commands_user ON telegram_commands(user_id);
CREATE INDEX idx_telegram_commands_command ON telegram_commands(command);
CREATE INDEX idx_telegram_commands_executed_at ON telegram_commands(executed_at);

-- ============================================================================
-- FUNCTIONS AND TRIGGERS
-- ============================================================================

-- Function to update the updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
RETURN NEW;
END;
$$ language 'plpgsql';

-- Triggers for updated_at columns
CREATE TRIGGER update_sessions_updated_at BEFORE UPDATE ON sessions FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_wallets_updated_at BEFORE UPDATE ON wallets FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_tokens_updated_at BEFORE UPDATE ON tokens FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_trades_updated_at BEFORE UPDATE ON trades FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_positions_updated_at BEFORE UPDATE ON positions FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
CREATE TRIGGER update_telegram_users_updated_at BEFORE UPDATE ON telegram_users FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Function to calculate position PnL
CREATE OR REPLACE FUNCTION calculate_position_pnl(
    p_position_id UUID,
    p_current_price DECIMAL(20, 12)
)
RETURNS DECIMAL(20, 8) AS $$
DECLARE
v_entry_price DECIMAL(20, 12);
    v_quantity DECIMAL(30, 8);
    v_pnl DECIMAL(20, 8);
BEGIN
SELECT entry_price, quantity INTO v_entry_price, v_quantity
FROM positions
WHERE id = p_position_id;

IF v_entry_price IS NULL THEN
        RETURN 0;
END IF;

    v_pnl := v_quantity * (p_current_price - v_entry_price);

UPDATE positions
SET
    current_price = p_current_price,
    unrealized_pnl_sol = v_pnl,
    updated_at = NOW()
WHERE id = p_position_id;

RETURN v_pnl;
END;
$$ LANGUAGE plpgsql;

-- Function to update session statistics
CREATE OR REPLACE FUNCTION update_session_stats(p_session_id UUID)
RETURNS VOID AS $$
DECLARE
v_total_trades INTEGER;
    v_profitable_trades INTEGER;
    v_total_pnl DECIMAL(20, 8);
    v_max_drawdown DECIMAL(10, 4);
BEGIN
    -- Calculate total trades
SELECT COUNT(*) INTO v_total_trades
FROM trades
WHERE session_id = p_session_id AND status = 'executed';

-- Calculate profitable trades
SELECT COUNT(*) INTO v_profitable_trades
FROM trades
WHERE session_id = p_session_id AND status = 'executed' AND pnl_sol > 0;

-- Calculate total PnL
SELECT COALESCE(SUM(pnl_sol), 0) INTO v_total_pnl
FROM trades
WHERE session_id = p_session_id AND status = 'executed';

-- Calculate max drawdown (simplified)
SELECT COALESCE(MIN(pnl_percent), 0) INTO v_max_drawdown
FROM trades
WHERE session_id = p_session_id AND status = 'executed';

-- Update session
UPDATE sessions
SET
    total_trades = v_total_trades,
    profitable_trades = v_profitable_trades,
    total_pnl = v_total_pnl,
    max_drawdown = ABS(v_max_drawdown),
    updated_at = NOW()
WHERE id = p_session_id;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- VIEWS FOR COMMON QUERIES
-- ============================================================================

-- Active sessions view
CREATE VIEW active_sessions AS
SELECT
    s.*,
    (s.profitable_trades::DECIMAL / NULLIF(s.total_trades, 0) * 100) AS win_rate_percent,
    w.address as wallet_address
FROM sessions s
         LEFT JOIN wallets w ON w.is_active = true
WHERE s.is_active = true;

-- Open positions with current PnL view
CREATE VIEW open_positions AS
SELECT
    p.*,
    t.symbol,
    t.name as token_name,
    w.address as wallet_address,
    s.name as session_name
FROM positions p
         JOIN tokens t ON p.token_id = t.id
         JOIN wallets w ON p.wallet_id = w.id
         JOIN sessions s ON p.session_id = s.id
WHERE p.is_open = true;

-- Recent trades view
CREATE VIEW recent_trades AS
SELECT
    tr.*,
    t.symbol,
    t.name as token_name,
    w.address as wallet_address,
    s.name as session_name
FROM trades tr
         JOIN tokens t ON tr.token_id = t.id
         JOIN wallets w ON tr.wallet_id = w.id
         JOIN sessions s ON tr.session_id = s.id
ORDER BY tr.executed_at DESC;

-- Daily performance view
CREATE VIEW daily_performance AS
SELECT
        DATE(executed_at) as trade_date,
        session_id,
        COUNT(*) as total_trades,
        COUNT(CASE WHEN pnl_sol > 0 THEN 1 END) as profitable_trades,
        SUM(pnl_sol) as daily_pnl,
        AVG(execution_time_ms) as avg_execution_time,
        MAX(execution_time_ms) as max_execution_time
        FROM trades
        WHERE status = 'executed' AND executed_at >= CURRENT_DATE - INTERVAL '30 days'
        GROUP BY DATE(executed_at), session_id
        ORDER BY trade_date DESC;

-- Component health status view
CREATE VIEW component_health AS
SELECT
    component_name,
    status,
    message,
    response_time_ms,
    checked_at,
    ROW_NUMBER() OVER (PARTITION BY component_name ORDER BY checked_at DESC) as rn
FROM health_checks
WHERE checked_at >= NOW() - INTERVAL '1 hour';

-- ============================================================================
-- INITIAL DATA
-- ============================================================================

-- Insert default health check components
INSERT INTO health_checks (component_name, status, message) VALUES
                                                                ('database', 'healthy', 'Database connection established'),
                                                                ('redis', 'starting', 'Redis connection initializing'),
                                                                ('solana_rpc', 'starting', 'Solana RPC connection initializing'),
                                                                ('helius_api', 'starting', 'Helius API connection initializing'),
                                                                ('birdeye_api', 'starting', 'Birdeye API connection initializing'),
                                                                ('telegram_bot', 'starting', 'Telegram bot initializing'),
                                                                ('metrics', 'starting', 'Metrics system initializing');

-- ============================================================================
-- GRANTS AND PERMISSIONS
-- ============================================================================

-- Grant permissions to application user (if different from owner)
-- GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO sniper_app;
-- GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO sniper_app;
-- GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA public TO sniper_app;

-- Comments for documentation
COMMENT ON TABLE sessions IS 'Trading sessions with configuration snapshots and performance metrics';
COMMENT ON TABLE wallets IS 'Encrypted wallet storage with balance tracking';
COMMENT ON TABLE tokens IS 'Discovered tokens with metadata and risk indicators';
COMMENT ON TABLE trades IS 'Complete trade execution history with performance metrics';
COMMENT ON TABLE positions IS 'Open and closed position tracking with PnL calculations';
COMMENT ON TABLE risk_assessments IS 'Token risk analysis results from various evaluation rules';
COMMENT ON TABLE health_checks IS 'System component health monitoring';
COMMENT ON TABLE metrics IS 'Performance and business metrics storage';
COMMENT ON TABLE error_logs IS 'Application error tracking and resolution status';
COMMENT ON TABLE audit_logs IS 'Security and compliance audit trail';
COMMENT ON TABLE telegram_users IS 'Telegram bot user management and preferences';
COMMENT ON TABLE telegram_commands IS 'Telegram command execution history and performance';

-- Migration completion marker
INSERT INTO metrics (metric_name, metric_value, labels) VALUES
    ('migration_completed', 1, '{"version": "20240602000001", "phase": "3"}');