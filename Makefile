# Solana Sniper Bot - Professional Makefile
# Version: 1.0.0
# Author: Solana Sniper Team

# Go parameters
GOCMD=go
GOBUILD=$(GOCMD) build
GOCLEAN=$(GOCMD) clean
GOTEST=$(GOCMD) test
GOGET=$(GOCMD) get
GOMOD=$(GOCMD) mod
BINARY_NAME=sniper-bot
BINARY_UNIX=$(BINARY_NAME)_unix
MAIN_PATH=./cmd/main.go

# Docker parameters
DOCKER_IMAGE=solana-sniper-bot
DOCKER_TAG=latest
DOCKER_REGISTRY=registry.hub.docker.com

# Build information
VERSION := $(shell git describe --tags --always --dirty)
BUILD_TIME := $(shell date +%Y-%m-%d\ %H:%M)
GIT_COMMIT := $(shell git rev-parse HEAD)
LDFLAGS := -X 'main.Version=$(VERSION)' -X 'main.BuildTime=$(BUILD_TIME)' -X 'main.GitCommit=$(GIT_COMMIT)'

# Colors for terminal output
RED=\033[0;31m
GREEN=\033[0;32m
YELLOW=\033[1;33m
BLUE=\033[0;34m
NC=\033[0m # No Color

.PHONY: all build clean test coverage deps docker help install lint security audit performance

# Default target
all: deps build test

## Build Commands
build: ## Build the application
	@echo "$(GREEN)🔨 Building Solana Sniper Bot...$(NC)"
	CGO_ENABLED=0 GOOS=linux $(GOBUILD) -ldflags "$(LDFLAGS)" -a -installsuffix cgo -o $(BINARY_NAME) $(MAIN_PATH)
	@echo "$(GREEN)✅ Build completed successfully!$(NC)"

build-windows: ## Build for Windows
	@echo "$(BLUE)🔨 Building for Windows...$(NC)"
	CGO_ENABLED=0 GOOS=windows $(GOBUILD) -ldflags "$(LDFLAGS)" -a -installsuffix cgo -o $(BINARY_NAME).exe $(MAIN_PATH)

build-mac: ## Build for macOS
	@echo "$(BLUE)🔨 Building for macOS...$(NC)"
	CGO_ENABLED=0 GOOS=darwin $(GOBUILD) -ldflags "$(LDFLAGS)" -a -installsuffix cgo -o $(BINARY_NAME)_mac $(MAIN_PATH)

build-all: build build-windows build-mac ## Build for all platforms
	@echo "$(GREEN)✅ Multi-platform build completed!$(NC)"

## Development Commands
run: ## Run the application in development mode
	@echo "$(YELLOW)🚀 Starting Solana Sniper Bot in development mode...$(NC)"
	$(GOCMD) run $(MAIN_PATH)

run-prod: build ## Run the application in production mode
	@echo "$(GREEN)🚀 Starting Solana Sniper Bot in production mode...$(NC)"
	./$(BINARY_NAME)

watch: ## Run with auto-reload (requires air)
	@echo "$(YELLOW)👀 Starting with auto-reload...$(NC)"
	air

## Testing Commands
test: ## Run tests
	@echo "$(BLUE)🧪 Running tests...$(NC)"
	$(GOTEST) -v ./...

test-race: ## Run tests with race detection
	@echo "$(BLUE)🏁 Running tests with race detection...$(NC)"
	$(GOTEST) -race -v ./...

test-coverage: ## Run tests with coverage
	@echo "$(BLUE)📊 Running tests with coverage...$(NC)"
	$(GOTEST) -coverprofile=coverage.out ./...
	$(GOCMD) tool cover -html=coverage.out -o coverage.html
	@echo "$(GREEN)📋 Coverage report generated: coverage.html$(NC)"

benchmark: ## Run benchmarks
	@echo "$(BLUE)⚡ Running benchmarks...$(NC)"
	$(GOTEST) -bench=. -benchmem ./...

## Quality Commands
lint: ## Run linter
	@echo "$(YELLOW)🔍 Running linter...$(NC)"
	golangci-lint run

fmt: ## Format code
	@echo "$(BLUE)💄 Formatting code...$(NC)"
	$(GOCMD) fmt ./...

vet: ## Run go vet
	@echo "$(BLUE)🔍 Running go vet...$(NC)"
	$(GOCMD) vet ./...

security: ## Run security scan
	@echo "$(RED)🔒 Running security scan...$(NC)"
	gosec ./...

audit: ## Run dependency audit
	@echo "$(YELLOW)🔍 Running dependency audit...$(NC)"
	$(GOMOD) tidy
	nancy sleuth

## Dependencies
deps: ## Download dependencies
	@echo "$(BLUE)📦 Downloading dependencies...$(NC)"
	$(GOMOD) download
	$(GOMOD) tidy

deps-update: ## Update dependencies
	@echo "$(YELLOW)🔄 Updating dependencies...$(NC)"
	$(GOMOD) get -u ./...
	$(GOMOD) tidy

deps-vendor: ## Vendor dependencies
	@echo "$(BLUE)📦 Vendoring dependencies...$(NC)"
	$(GOMOD) vendor

## Docker Commands
docker-build: ## Build Docker image
	@echo "$(BLUE)🐳 Building Docker image...$(NC)"
	docker build -t $(DOCKER_IMAGE):$(DOCKER_TAG) .
	@echo "$(GREEN)✅ Docker image built successfully!$(NC)"

docker-build-prod: ## Build production Docker image
	@echo "$(BLUE)🐳 Building production Docker image...$(NC)"
	docker build -f deployments/docker/Dockerfile.prod -t $(DOCKER_IMAGE):prod .

docker-run: ## Run Docker container
	@echo "$(GREEN)🐳 Running Docker container...$(NC)"
	docker run -p 8080:8080 --env-file .env $(DOCKER_IMAGE):$(DOCKER_TAG)

docker-compose-up: ## Start all services with docker-compose
	@echo "$(BLUE)🐳 Starting services with docker-compose...$(NC)"
	docker-compose up -d

docker-compose-down: ## Stop all services
	@echo "$(YELLOW)🐳 Stopping services...$(NC)"
	docker-compose down

docker-push: ## Push Docker image to registry
	@echo "$(BLUE)🚀 Pushing Docker image to registry...$(NC)"
	docker tag $(DOCKER_IMAGE):$(DOCKER_TAG) $(DOCKER_REGISTRY)/$(DOCKER_IMAGE):$(DOCKER_TAG)
	docker push $(DOCKER_REGISTRY)/$(DOCKER_IMAGE):$(DOCKER_TAG)

## Database Commands
db-migrate: ## Run database migrations
	@echo "$(BLUE)🗄️  Running database migrations...$(NC)"
	migrate -path ./migrations -database "${DATABASE_URL}" up

db-rollback: ## Rollback database migrations
	@echo "$(YELLOW)🗄️  Rolling back database migrations...$(NC)"
	migrate -path ./migrations -database "${DATABASE_URL}" down 1

db-seed: ## Seed database with test data
	@echo "$(BLUE)🌱 Seeding database...$(NC)"
	$(GOCMD) run scripts/seed.go

## Performance Commands
profile-cpu: ## Profile CPU usage
	@echo "$(BLUE)⚡ Profiling CPU usage...$(NC)"
	$(GOCMD) test -cpuprofile=cpu.prof -bench=. ./...
	$(GOCMD) tool pprof cpu.prof

profile-memory: ## Profile memory usage
	@echo "$(BLUE)💾 Profiling memory usage...$(NC)"
	$(GOCMD) test -memprofile=mem.prof -bench=. ./...
	$(GOCMD) tool pprof mem.prof

## Cleanup Commands
clean: ## Clean build artifacts
	@echo "$(YELLOW)🧹 Cleaning build artifacts...$(NC)"
	$(GOCLEAN)
	rm -f $(BINARY_NAME)
	rm -f $(BINARY_NAME).exe
	rm -f $(BINARY_NAME)_mac
	rm -f $(BINARY_UNIX)
	rm -f coverage.out coverage.html
	rm -f cpu.prof mem.prof

clean-docker: ## Clean Docker images and containers
	@echo "$(YELLOW)🐳 Cleaning Docker resources...$(NC)"
	docker system prune -f

clean-all: clean clean-docker ## Clean everything
	@echo "$(GREEN)✅ Everything cleaned!$(NC)"

## Installation Commands
install: ## Install the application
	@echo "$(GREEN)📦 Installing Solana Sniper Bot...$(NC)"
	$(GOBUILD) -ldflags "$(LDFLAGS)" -o $(GOPATH)/bin/$(BINARY_NAME) $(MAIN_PATH)

install-tools: ## Install development tools
	@echo "$(BLUE)🔧 Installing development tools...$(NC)"
	$(GOGET) -u github.com/cosmtrek/air
	$(GOGET) -u github.com/golangci/golangci-lint/cmd/golangci-lint
	$(GOGET) -u github.com/securecodewarrior/gosec/v2/cmd/gosec
	$(GOGET) -u github.com/sonatypecommunity/nancy

## Release Commands
release: clean build-all test ## Create a release build
	@echo "$(GREEN)🎉 Release build completed!$(NC)"
	@echo "$(BLUE)Version: $(VERSION)$(NC)"
	@echo "$(BLUE)Build Time: $(BUILD_TIME)$(NC)"
	@echo "$(BLUE)Git Commit: $(GIT_COMMIT)$(NC)"

## Development Environment
dev-setup: install-tools deps ## Setup development environment
	@echo "$(GREEN)🛠️  Development environment setup completed!$(NC)"
	@echo "$(YELLOW)Don't forget to copy .env.example to .env and configure your settings!$(NC)"

dev-start: docker-compose-up ## Start development environment
	@echo "$(GREEN)🚀 Development environment started!$(NC)"

dev-stop: docker-compose-down ## Stop development environment
	@echo "$(YELLOW)🛑 Development environment stopped!$(NC)"

## Information Commands
version: ## Show version information
	@echo "$(BLUE)Solana Sniper Bot$(NC)"
	@echo "$(BLUE)Version: $(VERSION)$(NC)"
	@echo "$(BLUE)Build Time: $(BUILD_TIME)$(NC)"
	@echo "$(BLUE)Git Commit: $(GIT_COMMIT)$(NC)"

status: ## Show project status
	@echo "$(BLUE)📊 Project Status:$(NC)"
	@echo "$(GREEN)✅ Go version: $(shell go version)$(NC)"
	@echo "$(GREEN)✅ Docker: $(shell docker --version 2>/dev/null || echo 'Not installed')$(NC)"
	@echo "$(GREEN)✅ Make: $(shell make --version | head -1)$(NC)"

## Help
help: ## Show this help message
	@echo "$(BLUE)🚀 Solana Sniper Bot - Makefile Commands$(NC)"
	@echo ""
	@awk 'BEGIN {FS = ":.*##"} /^[a-zA-Z_-]+:.*##/ { printf "  $(GREEN)%-20s$(NC) %s\n", $$1, $$2 }' $(MAKEFILE_LIST)
	@echo ""
	@echo "$(YELLOW)📖 Usage Examples:$(NC)"
	@echo "  make build          # Build the application"
	@echo "  make test           # Run tests"
	@echo "  make docker-build   # Build Docker image"
	@echo "  make dev-setup      # Set up development environment"
	@echo "  make help           # Show this help"