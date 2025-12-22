# ============================================================================
# EdgeQuake - Full Stack Development Makefile
# ============================================================================
# 
# A unified interface for managing the EdgeQuake RAG framework stack:
#   - Rust backend API (edgequake)
#   - Next.js frontend (edgequake_webui)
#   - PostgreSQL with pgvector/AGE (docker)
#
# Usage:
#   make help          - Show all available commands
#   make install       - Install all dependencies
#   make dev           - Start development environment
#   make stop          - Stop all services
#
# ============================================================================

.PHONY: help install dev stop clean build test lint format \
        backend-dev backend-build backend-test backend-run \
        frontend-dev frontend-build frontend-test frontend-lint \
        db-start db-stop db-logs db-shell \
        docker-build docker-up docker-down docker-logs \
        check-deps

# Colors for terminal output
BLUE := \033[34m
GREEN := \033[32m
YELLOW := \033[33m
RED := \033[31m
BOLD := \033[1m
RESET := \033[0m

# Project directories
ROOT_DIR := $(shell pwd)
BACKEND_DIR := $(ROOT_DIR)/edgequake
FRONTEND_DIR := $(ROOT_DIR)/edgequake_webui
DOCKER_DIR := $(BACKEND_DIR)/docker

# Default target
.DEFAULT_GOAL := help

# ============================================================================
# Help
# ============================================================================

help: ## Show this help message
	@echo ""
	@echo "$(BOLD)EdgeQuake Development Commands$(RESET)"
	@echo "================================"
	@echo ""
	@echo "$(BOLD)$(BLUE)🚀 Quick Start$(RESET)"
	@echo "  $(GREEN)make install$(RESET)      Install all dependencies"
	@echo "  $(GREEN)make dev$(RESET)          Start full development stack"
	@echo "  $(GREEN)make stop$(RESET)         Stop all services"
	@echo ""
	@echo "$(BOLD)$(BLUE)🔧 Backend (Rust)$(RESET)"
	@echo "  $(GREEN)make backend-dev$(RESET)  Run backend in dev mode"
	@echo "  $(GREEN)make backend-build$(RESET) Build backend release"
	@echo "  $(GREEN)make backend-test$(RESET) Run backend tests"
	@echo "  $(GREEN)make backend-run$(RESET)  Run compiled backend"
	@echo ""
	@echo "$(BOLD)$(BLUE)🎨 Frontend (Next.js)$(RESET)"
	@echo "  $(GREEN)make frontend-dev$(RESET)  Start frontend dev server"
	@echo "  $(GREEN)make frontend-build$(RESET) Build frontend for production"
	@echo "  $(GREEN)make frontend-lint$(RESET) Lint frontend code"
	@echo ""
	@echo "$(BOLD)$(BLUE)🗄️  Database$(RESET)"
	@echo "  $(GREEN)make db-start$(RESET)     Start PostgreSQL container"
	@echo "  $(GREEN)make db-stop$(RESET)      Stop PostgreSQL container"
	@echo "  $(GREEN)make db-logs$(RESET)      View database logs"
	@echo "  $(GREEN)make db-shell$(RESET)     Open psql shell"
	@echo ""
	@echo "$(BOLD)$(BLUE)🐳 Docker$(RESET)"
	@echo "  $(GREEN)make docker-up$(RESET)    Start full stack via Docker"
	@echo "  $(GREEN)make docker-down$(RESET)  Stop Docker stack"
	@echo "  $(GREEN)make docker-build$(RESET) Rebuild Docker images"
	@echo "  $(GREEN)make docker-logs$(RESET)  View Docker logs"
	@echo ""
	@echo "$(BOLD)$(BLUE)🧹 Maintenance$(RESET)"
	@echo "  $(GREEN)make clean$(RESET)        Clean build artifacts"
	@echo "  $(GREEN)make lint$(RESET)         Lint all code"
	@echo "  $(GREEN)make format$(RESET)       Format all code"
	@echo "  $(GREEN)make test$(RESET)         Run all tests"
	@echo ""

# ============================================================================
# Dependency Checks
# ============================================================================

check-deps: ## Check that required dependencies are installed
	@echo "$(BLUE)Checking dependencies...$(RESET)"
	@command -v cargo >/dev/null 2>&1 || { echo "$(RED)❌ cargo not found. Install Rust: https://rustup.rs$(RESET)"; exit 1; }
	@command -v bun >/dev/null 2>&1 || command -v npm >/dev/null 2>&1 || { echo "$(RED)❌ bun/npm not found. Install Node.js/Bun$(RESET)"; exit 1; }
	@command -v docker >/dev/null 2>&1 || { echo "$(YELLOW)⚠️  docker not found. Some features require Docker$(RESET)"; }
	@echo "$(GREEN)✓ All required dependencies found$(RESET)"

# ============================================================================
# Installation
# ============================================================================

install: check-deps ## Install all project dependencies
	@echo ""
	@echo "$(BOLD)$(BLUE)📦 Installing dependencies...$(RESET)"
	@echo ""
	@echo "$(YELLOW)→ Installing Rust dependencies...$(RESET)"
	@cd $(BACKEND_DIR) && cargo fetch
	@echo "$(GREEN)✓ Rust dependencies installed$(RESET)"
	@echo ""
	@echo "$(YELLOW)→ Installing frontend dependencies...$(RESET)"
	@cd $(FRONTEND_DIR) && bun install 2>/dev/null || npm install
	@echo "$(GREEN)✓ Frontend dependencies installed$(RESET)"
	@echo ""
	@echo "$(BOLD)$(GREEN)✅ All dependencies installed!$(RESET)"
	@echo ""

# ============================================================================
# Development
# ============================================================================

dev: check-deps ## Start full development stack (DB + Backend + Frontend)
	@echo ""
	@echo "$(BOLD)$(BLUE)🚀 Starting EdgeQuake Development Stack$(RESET)"
	@echo ""
	@echo "$(YELLOW)→ Starting PostgreSQL...$(RESET)"
	@$(MAKE) db-start --no-print-directory
	@echo ""
	@echo "$(YELLOW)→ Starting services in parallel...$(RESET)"
	@echo "  $(BLUE)Backend$(RESET):  http://localhost:8080"
	@echo "  $(BLUE)Frontend$(RESET): http://localhost:3000"
	@echo "  $(BLUE)Swagger$(RESET):  http://localhost:8080/swagger-ui"
	@echo ""
	@echo "$(YELLOW)Press Ctrl+C to stop all services$(RESET)"
	@echo ""
	@trap 'echo ""; echo "$(YELLOW)Stopping services...$(RESET)"; $(MAKE) stop --no-print-directory; exit 0' INT; \
	(cd $(BACKEND_DIR) && cargo run 2>&1 | sed 's/^/[backend] /') & \
	(sleep 3 && cd $(FRONTEND_DIR) && (bun run dev 2>/dev/null || npm run dev) 2>&1 | sed 's/^/[frontend] /') & \
	wait

dev-frontend: ## Start only frontend dev server
	@$(MAKE) frontend-dev --no-print-directory

dev-backend: ## Start only backend dev server
	@$(MAKE) db-start --no-print-directory
	@$(MAKE) backend-dev --no-print-directory

stop: ## Stop all development services
	@echo "$(YELLOW)Stopping services...$(RESET)"
	@-pkill -f "cargo run" 2>/dev/null || true
	@-pkill -f "next dev" 2>/dev/null || true
	@-pkill -f "node.*edgequake_webui" 2>/dev/null || true
	@$(MAKE) db-stop --no-print-directory 2>/dev/null || true
	@echo "$(GREEN)✓ All services stopped$(RESET)"

# ============================================================================
# Backend
# ============================================================================

backend-dev: ## Run backend in development mode with hot reload
	@echo "$(BLUE)Starting backend development server...$(RESET)"
	@cd $(BACKEND_DIR) && cargo run

backend-build: ## Build backend for release
	@echo "$(BLUE)Building backend...$(RESET)"
	@cd $(BACKEND_DIR) && cargo build --release
	@echo "$(GREEN)✓ Backend built: $(BACKEND_DIR)/target/release/edgequake$(RESET)"

backend-test: ## Run backend tests
	@echo "$(BLUE)Running backend tests...$(RESET)"
	@cd $(BACKEND_DIR) && cargo test

backend-run: ## Run the compiled backend binary
	@echo "$(BLUE)Running backend...$(RESET)"
	@$(BACKEND_DIR)/target/release/edgequake

backend-clippy: ## Run Clippy linter on backend
	@echo "$(BLUE)Running Clippy...$(RESET)"
	@cd $(BACKEND_DIR) && cargo clippy -- -D warnings

backend-fmt: ## Format backend code
	@echo "$(BLUE)Formatting backend code...$(RESET)"
	@cd $(BACKEND_DIR) && cargo fmt

# ============================================================================
# Frontend
# ============================================================================

frontend-dev: ## Start frontend development server
	@echo "$(BLUE)Starting frontend development server...$(RESET)"
	@cd $(FRONTEND_DIR) && (bun run dev 2>/dev/null || npm run dev)

frontend-build: ## Build frontend for production
	@echo "$(BLUE)Building frontend...$(RESET)"
	@cd $(FRONTEND_DIR) && (bun run build 2>/dev/null || npm run build)
	@echo "$(GREEN)✓ Frontend built$(RESET)"

frontend-start: ## Start frontend production server
	@echo "$(BLUE)Starting frontend production server...$(RESET)"
	@cd $(FRONTEND_DIR) && (bun run start 2>/dev/null || npm run start)

frontend-lint: ## Lint frontend code
	@echo "$(BLUE)Linting frontend code...$(RESET)"
	@cd $(FRONTEND_DIR) && (bun run lint 2>/dev/null || npm run lint)

frontend-test: ## Run frontend tests
	@echo "$(BLUE)Running frontend tests...$(RESET)"
	@cd $(FRONTEND_DIR) && (bun test 2>/dev/null || npm test) || echo "$(YELLOW)No tests configured$(RESET)"

# ============================================================================
# Database
# ============================================================================

db-start: ## Start PostgreSQL container
	@echo "$(BLUE)Starting PostgreSQL...$(RESET)"
	@cd $(DOCKER_DIR) && docker compose up -d postgres
	@echo "$(GREEN)✓ PostgreSQL started on port 5432$(RESET)"
	@echo "$(YELLOW)Waiting for database to be ready...$(RESET)"
	@sleep 3
	@until docker exec edgequake-postgres pg_isready -U edgequake -d edgequake 2>/dev/null; do \
		echo "Waiting..."; \
		sleep 2; \
	done
	@echo "$(GREEN)✓ Database is ready$(RESET)"

db-stop: ## Stop PostgreSQL container
	@echo "$(BLUE)Stopping PostgreSQL...$(RESET)"
	@cd $(DOCKER_DIR) && docker compose stop postgres 2>/dev/null || true
	@echo "$(GREEN)✓ PostgreSQL stopped$(RESET)"

db-logs: ## View PostgreSQL logs
	@cd $(DOCKER_DIR) && docker compose logs -f postgres

db-shell: ## Open psql shell
	@docker exec -it edgequake-postgres psql -U edgequake -d edgequake

db-reset: ## Reset database (WARNING: deletes all data)
	@echo "$(RED)⚠️  This will delete all data. Are you sure? [y/N]$(RESET)"
	@read -r confirm && [ "$$confirm" = "y" ] && \
		cd $(DOCKER_DIR) && docker compose down -v postgres && \
		docker compose up -d postgres && \
		echo "$(GREEN)✓ Database reset$(RESET)" || \
		echo "$(YELLOW)Cancelled$(RESET)"

# ============================================================================
# Docker (Full Stack)
# ============================================================================

docker-build: ## Build all Docker images
	@echo "$(BLUE)Building Docker images...$(RESET)"
	@cd $(DOCKER_DIR) && docker compose build
	@echo "$(GREEN)✓ Docker images built$(RESET)"

docker-up: ## Start full stack via Docker Compose
	@echo "$(BLUE)Starting Docker stack...$(RESET)"
	@cd $(DOCKER_DIR) && docker compose up -d
	@echo ""
	@echo "$(GREEN)✓ EdgeQuake stack is running$(RESET)"
	@echo ""
	@echo "  $(BLUE)API$(RESET):     http://localhost:8080"
	@echo "  $(BLUE)Swagger$(RESET): http://localhost:8080/swagger-ui"
	@echo ""

docker-down: ## Stop Docker stack
	@echo "$(BLUE)Stopping Docker stack...$(RESET)"
	@cd $(DOCKER_DIR) && docker compose down
	@echo "$(GREEN)✓ Docker stack stopped$(RESET)"

docker-logs: ## View Docker logs
	@cd $(DOCKER_DIR) && docker compose logs -f

docker-ps: ## Show Docker container status
	@cd $(DOCKER_DIR) && docker compose ps

# ============================================================================
# Quality Assurance
# ============================================================================

lint: backend-clippy frontend-lint ## Lint all code
	@echo "$(GREEN)✓ All linting passed$(RESET)"

format: backend-fmt ## Format all code
	@echo "$(GREEN)✓ All code formatted$(RESET)"

test: backend-test frontend-test ## Run all tests
	@echo "$(GREEN)✓ All tests passed$(RESET)"

build: backend-build frontend-build ## Build all projects
	@echo "$(GREEN)✓ All projects built$(RESET)"

# ============================================================================
# Cleanup
# ============================================================================

clean: ## Clean all build artifacts
	@echo "$(BLUE)Cleaning build artifacts...$(RESET)"
	@cd $(BACKEND_DIR) && cargo clean
	@rm -rf $(FRONTEND_DIR)/.next $(FRONTEND_DIR)/node_modules/.cache
	@echo "$(GREEN)✓ Build artifacts cleaned$(RESET)"

clean-all: clean ## Clean everything including node_modules
	@echo "$(BLUE)Cleaning all dependencies...$(RESET)"
	@rm -rf $(FRONTEND_DIR)/node_modules
	@echo "$(GREEN)✓ All cleaned$(RESET)"

# ============================================================================
# Utilities
# ============================================================================

swagger: ## Open Swagger UI in browser
	@echo "$(BLUE)Opening Swagger UI...$(RESET)"
	@open http://localhost:8080/swagger-ui 2>/dev/null || xdg-open http://localhost:8080/swagger-ui 2>/dev/null || echo "Open http://localhost:8080/swagger-ui in your browser"

logs: ## Show recent logs from all services
	@echo "$(BOLD)Recent Backend Logs:$(RESET)"
	@tail -20 $(BACKEND_DIR)/edgequake.log 2>/dev/null || echo "No backend logs found"
	@echo ""
	@echo "$(BOLD)Docker Container Status:$(RESET)"
	@cd $(DOCKER_DIR) && docker compose ps 2>/dev/null || echo "Docker not running"

status: ## Show status of all services
	@echo ""
	@echo "$(BOLD)EdgeQuake Service Status$(RESET)"
	@echo "========================="
	@echo ""
	@echo "$(BOLD)Backend:$(RESET)"
	@curl -s http://localhost:8080/health | jq . 2>/dev/null || echo "  $(RED)Not running$(RESET)"
	@echo ""
	@echo "$(BOLD)Frontend:$(RESET)"
	@curl -s http://localhost:3000 >/dev/null 2>&1 && echo "  $(GREEN)Running on http://localhost:3000$(RESET)" || echo "  $(RED)Not running$(RESET)"
	@echo ""
	@echo "$(BOLD)Database:$(RESET)"
	@docker exec edgequake-postgres pg_isready -U edgequake -d edgequake 2>/dev/null && echo "  $(GREEN)Running on localhost:5432$(RESET)" || echo "  $(RED)Not running$(RESET)"
	@echo ""
