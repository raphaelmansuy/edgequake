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

.PHONY: help install dev dev-bg dev-memory stop clean build test lint format \
        backend-dev backend-db backend-memory backend-bg backend-build backend-test backend-run \
        frontend-dev frontend-build frontend-test frontend-lint \
        db-start db-stop db-wait db-logs db-shell \
        docker-build docker-up docker-down docker-logs \
        check-deps status

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
	@echo "  $(GREEN)make dev$(RESET)          Start full development stack (PostgreSQL)"
	@echo "  $(GREEN)make dev-bg$(RESET)       Start full stack in BACKGROUND (for agents)"
	@echo "  $(GREEN)make dev-memory$(RESET)   Start with in-memory storage (for testing)"
	@echo "  $(GREEN)make stop$(RESET)         Stop all services"
	@echo "  $(GREEN)make status$(RESET)       Check status of all services"
	@echo ""
	@echo "$(BOLD)$(BLUE)🔧 Backend (Rust)$(RESET)"
	@echo "  $(GREEN)make backend-dev$(RESET)  Run backend with PostgreSQL (DEFAULT)"
	@echo "  $(GREEN)make backend-db$(RESET)   Run backend with PostgreSQL (explicit)"
	@echo "  $(GREEN)make backend-memory$(RESET) Run backend with in-memory (testing)"
	@echo "  $(GREEN)make backend-bg$(RESET)   Run backend in background"
	@echo "  $(GREEN)make backend-build$(RESET) Build backend release"
	@echo "  $(GREEN)make backend-test$(RESET) Run backend tests"
	@echo ""
	@echo "$(BOLD)$(BLUE)🎨 Frontend (Next.js)$(RESET)"
	@echo "  $(GREEN)make frontend-dev$(RESET)  Start frontend dev server"
	@echo "  $(GREEN)make frontend-build$(RESET) Build frontend for production"
	@echo "  $(GREEN)make frontend-lint$(RESET) Lint frontend code"
	@echo ""
	@echo "$(BOLD)$(BLUE)🗄️  Database$(RESET)"
	@echo "  $(GREEN)make db-start$(RESET)     Start PostgreSQL container"
	@echo "  $(GREEN)make db-stop$(RESET)      Stop PostgreSQL container"
	@echo "  $(GREEN)make db-wait$(RESET)      Wait for database to be ready"
	@echo "  $(GREEN)make db-logs$(RESET)      View database logs"
	@echo "  $(GREEN)make db-shell$(RESET)     Open psql shell"
	@echo "  $(GREEN)make db-clean$(RESET)     Clean all data (non-interactive)"
	@echo "  $(GREEN)make db-clean-force$(RESET) Destroy and recreate DB container"
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

dev: check-deps ## Start full development stack (DB + Backend + Frontend) with Ollama
	@echo ""
	@echo "$(BOLD)$(BLUE)🚀 Starting EdgeQuake Development Stack$(RESET)"
	@echo "$(BOLD)$(YELLOW)📝 Using Ollama as default LLM provider$(RESET)"
	@echo ""
	@echo "$(YELLOW)→ Stopping any existing services...$(RESET)"
	@$(MAKE) stop --no-print-directory 2>/dev/null || true
	@sleep 2
	@echo ""
	@echo "$(YELLOW)→ Starting PostgreSQL...$(RESET)"
	@$(MAKE) db-start --no-print-directory
	@echo ""
	@echo "$(YELLOW)→ Starting services in parallel...$(RESET)"
	@echo "  $(BLUE)Backend$(RESET):  http://localhost:8080"
	@echo "  $(BLUE)Frontend$(RESET): http://localhost:3000"
	@echo "  $(BLUE)Swagger$(RESET):  http://localhost:8080/swagger-ui"
	@echo "  $(BLUE)Provider$(RESET): Ollama (http://localhost:11434)"
	@echo ""
	@echo "$(GREEN)✓ Services starting...$(RESET)"
	@echo "$(YELLOW)Press Ctrl+C to stop all services$(RESET)"
	@echo ""
	@trap 'echo ""; echo "$(YELLOW)Stopping services...$(RESET)"; $(MAKE) stop --no-print-directory; exit 0' INT; \
	(cd $(BACKEND_DIR) && \
		DATABASE_URL="postgresql://edgequake:edgequake_secret@localhost:5432/edgequake" \
		LLM_PROVIDER="ollama" \
		LLM_MODEL="qwen2.5:7b" \
		LLM_BASE_URL="http://localhost:11434" \
		EMBEDDING_PROVIDER="ollama" \
		EMBEDDING_MODEL="nomic-embed-text" \
		EMBEDDING_BASE_URL="http://localhost:11434" \
		cargo run 2>&1 | sed 's/^/[backend] /') & \
	BACKEND_PID=$$!; \
	(sleep 5 && cd $(FRONTEND_DIR) && (bun run dev 2>/dev/null || npm run dev) 2>&1 | sed 's/^/[frontend] /') & \
	FRONTEND_PID=$$!; \
	echo "$(GREEN)✓ Backend PID: $$BACKEND_PID, Frontend PID: $$FRONTEND_PID$(RESET)"; \
	wait

dev-frontend: ## Start only frontend dev server
	@$(MAKE) frontend-dev --no-print-directory

dev-backend: ## Start only backend dev server (with database)
	@$(MAKE) db-start --no-print-directory
	@$(MAKE) backend-dev --no-print-directory

dev-memory: check-deps ## Start development with in-memory storage (for testing)
	@echo ""
	@echo "$(BOLD)$(YELLOW)⚠️  Starting EdgeQuake with IN-MEMORY Storage$(RESET)"
	@echo "$(YELLOW)Data will NOT persist across restarts!$(RESET)"
	@echo ""
	@trap 'echo ""; echo "$(YELLOW)Stopping services...$(RESET)"; $(MAKE) stop --no-print-directory; exit 0' INT; \
	(cd $(BACKEND_DIR) && cargo run 2>&1 | sed 's/^/[backend] /') & \
	BACKEND_PID=$$!; \
	(sleep 5 && cd $(FRONTEND_DIR) && (bun run dev 2>/dev/null || npm run dev) 2>&1 | sed 's/^/[frontend] /') & \
	FRONTEND_PID=$$!; \
	echo "$(GREEN)✓ Backend PID: $$BACKEND_PID, Frontend PID: $$FRONTEND_PID$(RESET)"; \
	wait

dev-bg: check-deps ## Start full development stack in BACKGROUND (agentic mode) with Ollama
	@echo ""
	@echo "$(BOLD)$(BLUE)🤖 Starting EdgeQuake in Background Mode (Agentic)$(RESET)"
	@echo "$(BOLD)$(YELLOW)📝 Using Ollama as default LLM provider$(RESET)"
	@echo ""
	@$(MAKE) stop --no-print-directory 2>/dev/null || true
	@sleep 1
	@echo "$(YELLOW)→ Starting PostgreSQL...$(RESET)"
	@$(MAKE) db-start --no-print-directory
	@echo ""
	@echo "$(YELLOW)→ Waiting for database...$(RESET)"
	@for i in 1 2 3 4 5 6 7 8 9 10; do \
		docker exec edgequake-postgres pg_isready -U edgequake -d edgequake 2>/dev/null && break || sleep 2; \
	done
	@echo ""
	@echo "$(YELLOW)→ Starting backend in background...$(RESET)"
	@cd $(BACKEND_DIR) && \
		DATABASE_URL="$(DATABASE_URL)" \
		LLM_PROVIDER="ollama" \
		LLM_MODEL="qwen2.5:7b" \
		LLM_BASE_URL="http://localhost:11434" \
		EMBEDDING_PROVIDER="ollama" \
		EMBEDDING_MODEL="nomic-embed-text" \
		EMBEDDING_BASE_URL="http://localhost:11434" \
		nohup cargo run > /tmp/edgequake-backend.log 2>&1 &
	@echo "$(GREEN)✓ Backend starting (log: /tmp/edgequake-backend.log)$(RESET)"
	@echo ""
	@echo "$(YELLOW)→ Waiting for backend to start...$(RESET)"
	@sleep 8
	@echo ""
	@echo "$(YELLOW)→ Starting frontend in background...$(RESET)"
	@cd $(FRONTEND_DIR) && nohup sh -c 'bun run dev 2>/dev/null || npm run dev' > /tmp/edgequake-frontend.log 2>&1 &
	@echo "$(GREEN)✓ Frontend starting (log: /tmp/edgequake-frontend.log)$(RESET)"
	@echo ""
	@sleep 3
	@echo "$(BOLD)$(GREEN)✅ EdgeQuake Background Stack Started$(RESET)"
	@echo ""
	@echo "  $(BLUE)Backend$(RESET):  http://localhost:8080"
	@echo "  $(BLUE)Frontend$(RESET): http://localhost:3000"
	@echo "  $(BLUE)Swagger$(RESET):  http://localhost:8080/swagger-ui"
	@echo "  $(BLUE)Provider$(RESET): Ollama (http://localhost:11434)"
	@echo ""
	@echo "  Use $(BOLD)make status$(RESET) to check service health"
	@echo "  Use $(BOLD)make stop$(RESET) to stop all services"
	@echo ""

stop: ## Stop all development services
	@echo "$(YELLOW)Stopping services...$(RESET)"
	@echo "$(BLUE)→ Stopping backend processes...$(RESET)"
	@-pkill -f "cargo run" 2>/dev/null || true
	@-pkill -9 -f "edgequake-api" 2>/dev/null || true
	@echo "$(BLUE)→ Stopping frontend processes...$(RESET)"
	@-pkill -f "next dev" 2>/dev/null || true
	@-pkill -f "node.*edgequake_webui" 2>/dev/null || true
	@-pkill -9 -f "bun.*dev" 2>/dev/null || true
	@echo "$(BLUE)→ Stopping database...$(RESET)"
	@$(MAKE) db-stop --no-print-directory 2>/dev/null || true
	@sleep 1
	@echo "$(GREEN)✓ All services stopped$(RESET)"

# ============================================================================
# Backend
# ============================================================================

# Database URL for PostgreSQL mode
DATABASE_URL := postgresql://edgequake:edgequake_secret@localhost:5432/edgequake

backend-dev: db-wait ## Run backend in development mode with PostgreSQL + Ollama (DEFAULT)
	@echo "$(BLUE)Starting backend with PostgreSQL storage + Ollama...$(RESET)"
	@cd $(BACKEND_DIR) && \
		DATABASE_URL="$(DATABASE_URL)" \
		LLM_PROVIDER="ollama" \
		LLM_MODEL="qwen2.5:7b" \
		LLM_BASE_URL="http://localhost:11434" \
		EMBEDDING_PROVIDER="ollama" \
		EMBEDDING_MODEL="nomic-embed-text" \
		EMBEDDING_BASE_URL="http://localhost:11434" \
		cargo run

backend-db: db-wait ## Run backend with PostgreSQL storage + Ollama (explicit)
	@echo "$(BLUE)Starting backend with PostgreSQL storage + Ollama (explicit)...$(RESET)"
	@cd $(BACKEND_DIR) && \
		DATABASE_URL="$(DATABASE_URL)" \
		LLM_PROVIDER="ollama" \
		LLM_MODEL="qwen2.5:7b" \
		LLM_BASE_URL="http://localhost:11434" \
		EMBEDDING_PROVIDER="ollama" \
		EMBEDDING_MODEL="nomic-embed-text" \
		EMBEDDING_BASE_URL="http://localhost:11434" \
		cargo run

backend-memory: ## Run backend with in-memory storage (for testing only)
	@echo "$(YELLOW)⚠️  Starting backend with IN-MEMORY storage (data will not persist)$(RESET)"
	@cd $(BACKEND_DIR) && cargo run

backend-bg: db-wait ## Run backend in background with PostgreSQL + Ollama
	@echo "$(BLUE)Starting backend in background with Ollama...$(RESET)"
	@cd $(BACKEND_DIR) && \
		DATABASE_URL="$(DATABASE_URL)" \
		LLM_PROVIDER="ollama" \
		LLM_MODEL="qwen2.5:7b" \
		LLM_BASE_URL="http://localhost:11434" \
		EMBEDDING_PROVIDER="ollama" \
		EMBEDDING_MODEL="nomic-embed-text" \
		EMBEDDING_BASE_URL="http://localhost:11434" \
		nohup cargo run > /tmp/edgequake-backend.log 2>&1 &
	@echo "$(GREEN)✓ Backend starting in background. Log: /tmp/edgequake-backend.log$(RESET)"

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

db-wait: db-start ## Wait for database to be ready (used by other targets)
	@echo "$(YELLOW)Waiting for database to be ready...$(RESET)"
	@for i in 1 2 3 4 5 6 7 8 9 10; do \
		docker exec edgequake-postgres pg_isready -U edgequake -d edgequake 2>/dev/null && break || sleep 2; \
	done
	@docker exec edgequake-postgres pg_isready -U edgequake -d edgequake 2>/dev/null && \
		echo "$(GREEN)✓ Database is ready$(RESET)" || \
		(echo "$(RED)✗ Database failed to start$(RESET)" && exit 1)

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

db-clean: ## Clean all data from database (non-interactive, for testing/CI)
	@echo "$(YELLOW)Cleaning all data from database...$(RESET)"
	@docker exec edgequake-postgres psql -U edgequake -d edgequake -c "\
		TRUNCATE TABLE documents CASCADE; \
		TRUNCATE TABLE chunks CASCADE; \
		TRUNCATE TABLE entities CASCADE; \
		TRUNCATE TABLE relationships CASCADE; \
		TRUNCATE TABLE tasks CASCADE; \
		TRUNCATE TABLE conversations CASCADE; \
		TRUNCATE TABLE messages CASCADE; \
		TRUNCATE TABLE folders CASCADE; \
		TRUNCATE TABLE tenants CASCADE; \
		TRUNCATE TABLE workspaces CASCADE; \
	" 2>/dev/null || echo "$(YELLOW)Some tables may not exist yet$(RESET)"
	@echo "$(GREEN)✓ Database cleaned$(RESET)"

db-clean-force: ## Force clean database by destroying and recreating container
	@echo "$(RED)Force cleaning database - destroying container...$(RESET)"
	@cd $(DOCKER_DIR) && docker compose down -v postgres 2>/dev/null || true
	@sleep 2
	@echo "$(YELLOW)→ Recreating database container...$(RESET)"
	@cd $(DOCKER_DIR) && docker compose up -d postgres
	@echo "$(YELLOW)→ Waiting for database to be ready...$(RESET)"
	@sleep 5
	@for i in 1 2 3 4 5 6 7 8 9 10; do \
		docker exec edgequake-postgres pg_isready -U edgequake -d edgequake 2>/dev/null && break || sleep 2; \
	done
	@echo "$(GREEN)✓ Database force cleaned and ready$(RESET)"

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
# PostgreSQL Integration Tests
# ============================================================================

test-postgres-start: ## Start PostgreSQL test containers
	@echo "$(BLUE)Starting PostgreSQL test containers...$(RESET)"
	@cd $(DOCKER_DIR) && docker compose -f docker-compose.test.yml up -d
	@echo "$(YELLOW)Waiting for databases to be ready...$(RESET)"
	@for i in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15; do \
		(docker exec edgequake-postgres-test pg_isready -U edgequake_test -d edgequake_test 2>/dev/null) && break || sleep 2; \
	done
	@echo "$(GREEN)✓ PostgreSQL test containers ready$(RESET)"

test-postgres-stop: ## Stop PostgreSQL test containers
	@echo "$(BLUE)Stopping PostgreSQL test containers...$(RESET)"
	@cd $(DOCKER_DIR) && docker compose -f docker-compose.test.yml down -v
	@echo "$(GREEN)✓ PostgreSQL test containers stopped$(RESET)"

test-postgres-storage: test-postgres-start ## Run PostgreSQL storage integration tests
	@echo "$(BLUE)Running PostgreSQL storage integration tests...$(RESET)"
	@cd $(BACKEND_DIR) && \
		POSTGRES_HOST=localhost \
		POSTGRES_PORT=5433 \
		POSTGRES_DB=edgequake_test \
		POSTGRES_USER=edgequake_test \
		POSTGRES_PASSWORD=test_password_123 \
		DATABASE_URL="postgresql://edgequake_test:test_password_123@localhost:5433/edgequake_test" \
		cargo test --package edgequake-storage --test postgres_integration --features postgres -- --test-threads=1
	@echo "$(GREEN)✓ PostgreSQL storage tests complete$(RESET)"

test-postgres-conversation: test-postgres-start ## Run PostgreSQL conversation integration tests
	@echo "$(BLUE)Running PostgreSQL conversation integration tests...$(RESET)"
	@cd $(BACKEND_DIR) && \
		POSTGRES_HOST=localhost \
		POSTGRES_PORT=5433 \
		POSTGRES_DB=edgequake_test \
		POSTGRES_USER=edgequake_test \
		POSTGRES_PASSWORD=test_password_123 \
		DATABASE_URL="postgresql://edgequake_test:test_password_123@localhost:5433/edgequake_test" \
		cargo test --package edgequake-storage --test postgres_conversation_integration --features postgres -- --test-threads=1
	@echo "$(GREEN)✓ PostgreSQL conversation tests complete$(RESET)"

test-postgres-workspace: test-postgres-start ## Run PostgreSQL workspace service tests
	@echo "$(BLUE)Running PostgreSQL workspace service tests...$(RESET)"
	@cd $(BACKEND_DIR) && \
		POSTGRES_HOST=localhost \
		POSTGRES_PORT=5433 \
		POSTGRES_DB=edgequake_test \
		POSTGRES_USER=edgequake_test \
		POSTGRES_PASSWORD=test_password_123 \
		DATABASE_URL="postgresql://edgequake_test:test_password_123@localhost:5433/edgequake_test" \
		cargo test --package edgequake-api --test e2e_postgres_workspace --features postgres -- --test-threads=1
	@echo "$(GREEN)✓ PostgreSQL workspace tests complete$(RESET)"

test-postgres-tasks: test-postgres-start ## Run PostgreSQL task storage tests
	@echo "$(BLUE)Running PostgreSQL task storage tests...$(RESET)"
	@cd $(BACKEND_DIR) && \
		POSTGRES_HOST=localhost \
		POSTGRES_PORT=5433 \
		POSTGRES_DB=edgequake_test \
		POSTGRES_USER=edgequake_test \
		POSTGRES_PASSWORD=test_password_123 \
		DATABASE_URL="postgresql://edgequake_test:test_password_123@localhost:5433/edgequake_test" \
		cargo test --package edgequake-tasks --features postgres -- --test-threads=1
	@echo "$(GREEN)✓ PostgreSQL task tests complete$(RESET)"

test-postgres-rls: test-postgres-start ## Run PostgreSQL RLS (Row Level Security) tests
	@echo "$(BLUE)Running PostgreSQL RLS tests...$(RESET)"
	@cd $(BACKEND_DIR) && \
		TEST_DATABASE_URL="postgresql://app_user:app_password_123@localhost:5433/edgequake_test" \
		ADMIN_DATABASE_URL="postgresql://edgequake_test:test_password_123@localhost:5433/edgequake_test" \
		cargo test --package edgequake-api --test e2e_postgres_rls --features postgres -- --ignored --test-threads=1
	@echo "$(GREEN)✓ PostgreSQL RLS tests complete$(RESET)"

test-postgres-all: test-postgres-start ## Run ALL PostgreSQL integration tests
	@echo "$(BOLD)$(BLUE)🧪 Running ALL PostgreSQL Integration Tests$(RESET)"
	@echo ""
	@$(MAKE) test-postgres-storage --no-print-directory || true
	@$(MAKE) test-postgres-conversation --no-print-directory || true
	@$(MAKE) test-postgres-workspace --no-print-directory || true
	@$(MAKE) test-postgres-tasks --no-print-directory || true
	@$(MAKE) test-postgres-rls --no-print-directory || true
	@echo ""
	@echo "$(GREEN)✓ All PostgreSQL integration tests completed$(RESET)"

test-postgres-ci: ## Run PostgreSQL tests in CI mode (starts containers, runs tests, stops containers)
	@echo "$(BOLD)$(BLUE)🤖 Running PostgreSQL CI Tests$(RESET)"
	@$(MAKE) test-postgres-start --no-print-directory
	@$(MAKE) test-postgres-all --no-print-directory
	@$(MAKE) test-postgres-stop --no-print-directory
	@echo "$(GREEN)✓ PostgreSQL CI tests complete$(RESET)"

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

rebuild: ## Full rebuild: stop + clean + dev (ensures latest code is running)
	@echo ""
	@echo "$(BOLD)$(BLUE)🔄 Full Rebuild - Ensuring Latest Code$(RESET)"
	@echo ""
	@$(MAKE) stop --no-print-directory 2>/dev/null || true
	@echo "$(YELLOW)→ Killing any stale processes...$(RESET)"
	@-pkill -9 -f "target/debug/edgequake" 2>/dev/null || true
	@-pkill -9 -f "target/release/edgequake" 2>/dev/null || true
	@-lsof -ti:8080 | xargs kill -9 2>/dev/null || true
	@-lsof -ti:3000 | xargs kill -9 2>/dev/null || true
	@sleep 2
	@echo "$(YELLOW)→ Cleaning build artifacts...$(RESET)"
	@$(MAKE) clean --no-print-directory
	@echo "$(YELLOW)→ Starting fresh development environment...$(RESET)"
	@$(MAKE) dev --no-print-directory

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
