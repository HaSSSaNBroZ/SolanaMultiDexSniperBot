// Package main is the entry point for the Solana Sniper Bot
// This is a professional-grade trading bot designed for ultra-fast token sniping
// on the Solana blockchain with advanced risk management and Telegram integration.
//
// Author: Solana Sniper Team
// Version: 1.0.0
// License: MIT
package main

import (
	"context"
	"fmt"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/gin-gonic/gin"
	"github.com/solana-sniper-bot/core/internal/config"
	"github.com/solana-sniper-bot/core/internal/utils"
)

// Build information (injected at compile time)
var (
	Version   = "dev"
	BuildTime = "unknown"
	GitCommit = "unknown"
)

// Application holds the main application components
type Application struct {
	Config *config.Config
	Logger *utils.Logger
	Server *http.Server
}

// main is the entry point of the Solana Sniper Bot application
func main() {
	// Print banner
	printBanner()

	// Initialize application
	app, err := initializeApplication()
	if err != nil {
		fmt.Printf("❌ Failed to initialize application: %v\n", err)
		os.Exit(1)
	}

	// Start application
	if err := app.Start(); err != nil {
		app.Logger.Error("Failed to start application", "error", err)
		os.Exit(1)
	}
}

// initializeApplication sets up and configures the application
func initializeApplication() (*Application, error) {
	// Load configuration
	cfg, err := config.Load()
	if err != nil {
		return nil, fmt.Errorf("failed to load configuration: %w", err)
	}

	// Initialize logger
	logger := utils.NewLogger(cfg.Log.Level, cfg.Log.Format)
	logger.Info("🚀 Initializing Solana Sniper Bot",
		"version", Version,
		"build_time", BuildTime,
		"git_commit", GitCommit,
	)

	// Validate configuration
	if err := utils.ValidateConfig(cfg); err != nil {
		return nil, fmt.Errorf("configuration validation failed: %w", err)
	}

	// Set Gin mode based on environment
	if cfg.Environment == "production" {
		gin.SetMode(gin.ReleaseMode)
	}

	// Create HTTP server
	server := &http.Server{
		Addr:           fmt.Sprintf(":%d", cfg.Server.Port),
		Handler:        setupRouter(cfg, logger),
		ReadTimeout:    time.Duration(cfg.Server.ReadTimeout) * time.Second,
		WriteTimeout:   time.Duration(cfg.Server.WriteTimeout) * time.Second,
		IdleTimeout:    time.Duration(cfg.Server.IdleTimeout) * time.Second,
		MaxHeaderBytes: cfg.Server.MaxHeaderBytes,
	}

	return &Application{
		Config: cfg,
		Logger: logger,
		Server: server,
	}, nil
}

// Start starts the application with graceful shutdown
func (app *Application) Start() error {
	// Create context for graceful shutdown
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	// Setup signal handling for graceful shutdown
	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, syscall.SIGINT, syscall.SIGTERM)

	// Start HTTP server in goroutine
	serverErrChan := make(chan error, 1)
	go func() {
		app.Logger.Info("🌐 Starting HTTP server",
			"address", app.Server.Addr,
			"environment", app.Config.Environment,
		)

		if err := app.Server.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			serverErrChan <- fmt.Errorf("HTTP server failed: %w", err)
		}
	}()

	// Wait for shutdown signal or server error
	select {
	case sig := <-sigChan:
		app.Logger.Info("🛑 Received shutdown signal", "signal", sig.String())
		return app.gracefulShutdown(ctx)
	case err := <-serverErrChan:
		app.Logger.Error("❌ Server error", "error", err)
		return err
	}
}

// gracefulShutdown performs graceful shutdown of the application
func (app *Application) gracefulShutdown(ctx context.Context) error {
	app.Logger.Info("🔄 Starting graceful shutdown...")

	// Create shutdown context with timeout
	shutdownCtx, cancel := context.WithTimeout(ctx, 30*time.Second)
	defer cancel()

	// Shutdown HTTP server
	if err := app.Server.Shutdown(shutdownCtx); err != nil {
		app.Logger.Error("❌ Failed to shutdown HTTP server", "error", err)
		return err
	}

	app.Logger.Info("✅ Graceful shutdown completed successfully")
	return nil
}

// setupRouter configures and returns the HTTP router
func setupRouter(cfg *config.Config, logger *utils.Logger) *gin.Engine {
	router := gin.New()

	// Add middleware
	router.Use(gin.Recovery())
	router.Use(corsMiddleware())
	router.Use(loggingMiddleware(logger))
	router.Use(securityMiddleware())

	// Health check endpoint
	router.GET("/health", healthCheckHandler(cfg))
	router.GET("/ready", readinessHandler())
	router.GET("/version", versionHandler())

	// API v1 routes
	v1 := router.Group("/api/v1")
	{
		v1.GET("/status", statusHandler(cfg))
		// More endpoints will be added in future phases
	}

	return router
}

// HTTP Handlers

// healthCheckHandler returns the health status of the application
func healthCheckHandler(cfg *config.Config) gin.HandlerFunc {
	return func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{
			"status":      "healthy",
			"timestamp":   time.Now().UTC(),
			"version":     Version,
			"environment": cfg.Environment,
		})
	}
}

// readinessHandler checks if the application is ready to serve traffic
func readinessHandler() gin.HandlerFunc {
	return func(c *gin.Context) {
		// TODO: Add checks for database, Redis, Solana RPC, etc.
		c.JSON(http.StatusOK, gin.H{
			"status":    "ready",
			"timestamp": time.Now().UTC(),
			"checks": gin.H{
				"database": "ok", // Will be implemented in Phase 2
				"solana":   "ok", // Will be implemented in Phase 3
				"redis":    "ok", // Will be implemented in Phase 2
			},
		})
	}
}

// versionHandler returns version information
func versionHandler() gin.HandlerFunc {
	return func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{
			"version":    Version,
			"build_time": BuildTime,
			"git_commit": GitCommit,
			"go_version": fmt.Sprintf("%s", os.Getenv("GO_VERSION")),
		})
	}
}

// statusHandler returns the current status of the bot
func statusHandler(cfg *config.Config) gin.HandlerFunc {
	return func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{
			"bot_status":    "initializing", // Will be dynamic in later phases
			"trading_mode":  cfg.Trading.Mode,
			"auto_trading":  cfg.Trading.AutoTrading,
			"risk_level":    cfg.Risk.Level,
			"max_positions": cfg.Trading.MaxPositions,
			"uptime":        time.Since(time.Now()).String(), // Will be calculated properly
		})
	}
}

// Middleware

// corsMiddleware adds CORS headers
func corsMiddleware() gin.HandlerFunc {
	return func(c *gin.Context) {
		c.Header("Access-Control-Allow-Origin", "*")
		c.Header("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS")
		c.Header("Access-Control-Allow-Headers", "Origin, Content-Type, Content-Length, Accept-Encoding, X-CSRF-Token, Authorization")

		if c.Request.Method == "OPTIONS" {
			c.AbortWithStatus(http.StatusNoContent)
			return
		}

		c.Next()
	}
}

// loggingMiddleware logs HTTP requests
func loggingMiddleware(logger *utils.Logger) gin.HandlerFunc {
	return func(c *gin.Context) {
		start := time.Now()
		path := c.Request.URL.Path
		raw := c.Request.URL.RawQuery

		// Process request
		c.Next()

		// Calculate latency
		latency := time.Since(start)

		if raw != "" {
			path = path + "?" + raw
		}

		logger.Info("HTTP Request",
			"status", c.Writer.Status(),
			"method", c.Request.Method,
			"path", path,
			"ip", c.ClientIP(),
			"user_agent", c.Request.UserAgent(),
			"latency", latency,
			"size", c.Writer.Size(),
		)
	}
}

// securityMiddleware adds security headers
func securityMiddleware() gin.HandlerFunc {
	return func(c *gin.Context) {
		c.Header("X-Content-Type-Options", "nosniff")
		c.Header("X-Frame-Options", "DENY")
		c.Header("X-XSS-Protection", "1; mode=block")
		c.Header("Strict-Transport-Security", "max-age=31536000; includeSubDomains")
		c.Header("Content-Security-Policy", "default-src 'self'")
		c.Next()
	}
}

// printBanner prints the application banner
func printBanner() {
	banner := `
╔══════════════════════════════════════════════════════════════╗
║                    🚀 SOLANA SNIPER BOT 🚀                    ║
║                                                              ║
║              Ultra-Fast Token Sniping System                 ║
║                Professional Trading Bot                      ║
║                                                              ║
║  Version: %-10s  Build: %-20s    ║
║  Commit:  %-50s    ║
║                                                              ║
║  🎯 Target: <50ms execution time                             ║
║  ⚡ Speed: Lightning-fast token detection                    ║
║  🛡️  Security: Advanced risk management                      ║
║  📱 Interface: Professional Telegram bot                    ║
║                                                              ║
║              Created by Solana Sniper Team                   ║
╚══════════════════════════════════════════════════════════════╝
	`

	fmt.Printf(banner, Version, BuildTime, GitCommit)
	fmt.Println()
}
