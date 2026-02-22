.PHONY: sdk-rust-build sdk-rust-publish sdk-rust-version
.PHONY: sdk-python-build sdk-python-publish sdk-python-version
.PHONY: sdk-typescript-build sdk-typescript-publish sdk-typescript-version
.PHONY: sdk-java-build sdk-java-publish sdk-java-version
.PHONY: sdk-kotlin-build sdk-kotlin-publish sdk-kotlin-version

sdk-rust-version: ## Update the version of the Rust SDK (sdks/rust). Usage: make sdk-rust-version VERSION=0.2.0
	@if [ -z "$(VERSION)" ]; then \
		echo "Usage: make sdk-rust-version VERSION=<new_version>"; \
		exit 1; \
	fi
	sed -i '' -E 's/^version = ".*"/version = "$(VERSION)"/' sdks/rust/Cargo.toml
	@echo "$(GREEN)✓ Updated Rust SDK version to $(VERSION) in sdks/rust/Cargo.toml$(RESET)"

# Python SDK targets
.PHONY: sdk-python-build sdk-python-publish sdk-python-version

sdk-python-build: ## Build the Python SDK (sdks/python)
	@echo "$(BOLD)$(BLUE)🔨 Building Python SDK (sdks/python)$(RESET)"
	cd sdks/python && rm -rf dist build && python3 -m pip install --upgrade build > /dev/null && python3 -m build

sdk-python-publish: ## Publish the Python SDK (sdks/python) to PyPI
	@echo "$(BOLD)$(BLUE)🚀 Publishing Python SDK (sdks/python) to PyPI$(RESET)"
	cd sdks/python && python3 -m pip install --upgrade twine > /dev/null && python3 -m twine upload dist/*

sdk-python-version: ## Update the version of the Python SDK (sdks/python). Usage: make sdk-python-version VERSION=0.2.0
	@if [ -z "$(VERSION)" ]; then \
		echo "Usage: make sdk-python-version VERSION=<new_version>"; \
		exit 1; \
	fi
	sed -i '' -E 's/^version = ".*"/version = "$(VERSION)"/' sdks/python/pyproject.toml
	@echo "$(GREEN)✓ Updated Python SDK version to $(VERSION) in sdks/python/pyproject.toml$(RESET)"

# TypeScript SDK targets
sdk-typescript-build: ## Build the TypeScript SDK (sdks/typescript)
	@echo "$(BOLD)$(BLUE)🔨 Building TypeScript SDK (sdks/typescript)$(RESET)"
	cd sdks/typescript && npm run build

sdk-typescript-publish: ## Publish the TypeScript SDK (sdks/typescript) to npm
	@echo "$(BOLD)$(BLUE)🚀 Publishing TypeScript SDK (sdks/typescript) to npm$(RESET)"
	cd sdks/typescript && npm publish

sdk-typescript-version: ## Update the version of the TypeScript SDK (sdks/typescript). Usage: make sdk-typescript-version VERSION=0.2.0
	@if [ -z "$(VERSION)" ]; then \
		echo "Usage: make sdk-typescript-version VERSION=<new_version>"; \
		exit 1; \
	fi
	sed -i '' -E 's/"version": ".*"/"version": "$(VERSION)"/' sdks/typescript/package.json
	@echo "$(GREEN)✓ Updated TypeScript SDK version to $(VERSION) in sdks/typescript/package.json$(RESET)"

# Java SDK targets
sdk-java-build: ## Build the Java SDK (sdks/java)
	@echo "$(BOLD)$(BLUE)🔨 Building Java SDK (sdks/java)$(RESET)"
	cd sdks/java && JAVA_HOME=$$(java_home -v 17) mvn clean package -DskipTests

sdk-java-publish: ## Publish the Java SDK (sdks/java) to Maven Central
	@echo "$(BOLD)$(BLUE)🚀 Publishing Java SDK (sdks/java) to Maven Central$(RESET)"
	cd sdks/java && JAVA_HOME=$$(java_home -v 17) mvn clean deploy -P ossrh

sdk-java-version: ## Update the version of the Java SDK (sdks/java). Usage: make sdk-java-version VERSION=0.2.0
	@if [ -z "$(VERSION)" ]; then \
		echo "Usage: make sdk-java-version VERSION=<new_version>"; \
		exit 1; \
	fi
	sed -i '' -E 's/<version>.*<\/version>/<version>$(VERSION)<\/version>/' sdks/java/pom.xml
	@echo "$(GREEN)✓ Updated Java SDK version to $(VERSION) in sdks/java/pom.xml$(RESET)"

# Kotlin SDK targets
sdk-kotlin-build: ## Build the Kotlin SDK (sdks/kotlin)
	@echo "$(BOLD)$(BLUE)🔨 Building Kotlin SDK (sdks/kotlin)$(RESET)"
	cd sdks/kotlin && JAVA_HOME=$$(java_home -v 17) mvn clean package -DskipTests

sdk-kotlin-publish: ## Publish the Kotlin SDK (sdks/kotlin) to Maven Central
	@echo "$(BOLD)$(BLUE)🚀 Publishing Kotlin SDK (sdks/kotlin) to Maven Central$(RESET)"
	cd sdks/kotlin && JAVA_HOME=$$(java_home -v 17) mvn clean deploy -P ossrh

sdk-kotlin-version: ## Update the version of the Kotlin SDK (sdks/kotlin). Usage: make sdk-kotlin-version VERSION=0.2.0
	@if [ -z "$(VERSION)" ]; then \
		echo "Usage: make sdk-kotlin-version VERSION=<new_version>"; \
		exit 1; \
	fi
	sed -i '' -E 's/<version>.*<\/version>/<version>$(VERSION)<\/version>/' sdks/kotlin/pom.xml
	@echo "$(GREEN)✓ Updated Kotlin SDK version to $(VERSION) in sdks/kotlin/pom.xml$(RESET)"

 
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
# =========================================================================
# Cargo Release Automation
# =========================================================================

.PHONY: install-cargo-release release

install-cargo-release: ## Install cargo-release tool for workspace version management
	cargo install cargo-release

# Usage: make release VERSION=0.2.2 [LEVEL=patch|minor|major]
release: ## Bump all crate versions and tag release using cargo-release (uses VERSION file if VERSION is unset)
	@if ! command -v cargo-release >/dev/null 2>&1; then \
		echo "cargo-release not found. Installing..."; \
		cargo install cargo-release; \
	fi
	@if [ -z "$(VERSION)" ]; then \
		if [ -f VERSION ]; then \
			VERSION_FILE=$$(cat VERSION | tr -d '\n'); \
			if [ -z "$$VERSION_FILE" ]; then \
				echo "VERSION file is empty. Please set a version."; \
				exit 1; \
			fi; \
			VERSION=$$VERSION_FILE; \
		else \
			echo "VERSION variable not set and VERSION file not found."; \
			exit 1; \
		fi; \
	fi; \
	cd edgequake && cargo release $$VERSION --workspace --no-publish --execute


.PHONY: help install dev dev-bg dev-memory stop clean build test lint format \
        backend-dev backend-db backend-memory backend-bg backend-build backend-build-online backend-sqlx-prepare backend-test backend-run \
        frontend-dev frontend-build frontend-test frontend-lint \
        db-start db-stop db-wait db-logs db-shell \
        docker-build docker-up docker-down docker-logs \
        check-deps status \
        test-quality test-invariants test-timing test-count test-flaky \
        test-e2e-critical test-e2e-full test-stability-report

# ============================================================================
# Version Management
# ============================================================================

.PHONY: version-bump version-tag

# Bump version in VERSION, Cargo.toml, and package.json
version-bump:
	@if [ -z "$(VERSION)" ]; then \
	  echo "Usage: make version-bump VERSION=<new_version>"; \
	  exit 1; \
	fi
	bash scripts/bump-version.sh $(VERSION)

# Tag and push release
version-tag:
	@if [ -z "$(VERSION)" ]; then \
	  echo "Set VERSION=<new_version> make version-bump version-tag"; \
	  exit 1; \
	fi
	git commit -am "Bump version to $(VERSION)"
	git tag v$(VERSION)
	git push && git push --tags

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

# Load environment variables from .env file if it exists
-include $(ROOT_DIR)/.env
export

# Environment variables (can be overridden from shell)
OPENAI_API_KEY ?= $(shell echo $$OPENAI_API_KEY)

# OODA-09: Auto-configure providers based on OPENAI_API_KEY presence.
# WHY: User sets OPENAI_API_KEY but system still uses Ollama defaults.
# This ensures correct provider selection when API key is available.
ifdef OPENAI_API_KEY
  # Use OpenAI as default when API key is set
  EDGEQUAKE_DEFAULT_LLM_PROVIDER ?= openai
  EDGEQUAKE_DEFAULT_LLM_MODEL ?= gpt-5-nano
  EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER ?= openai
  EDGEQUAKE_DEFAULT_EMBEDDING_MODEL ?= text-embedding-3-small
  EDGEQUAKE_DEFAULT_EMBEDDING_DIMENSION ?= 1536
else
  # Fall back to Ollama when no API key
  EDGEQUAKE_DEFAULT_LLM_PROVIDER ?= ollama
  EDGEQUAKE_DEFAULT_LLM_MODEL ?= gemma3:12b
  EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER ?= ollama
  EDGEQUAKE_DEFAULT_EMBEDDING_MODEL ?= embeddinggemma
  EDGEQUAKE_DEFAULT_EMBEDDING_DIMENSION ?= 768
endif

# SPEC-040: Vision/VLM provider defaults for PDF-to-Markdown conversion
# Vision always uses OpenAI by default (requires vision-capable model)
EDGEQUAKE_VISION_PROVIDER ?= openai
EDGEQUAKE_VISION_MODEL ?= gpt-4.1-nano

# Default target
.DEFAULT_GOAL := help

# ============================================================================
# Help
# ============================================================================

help: ## Show this help message
	@echo ""
	@echo "$(BOLD)EdgeQuake Development Commands$(RESET)"
	@echo "  $(GREEN)make install-cargo-release$(RESET)  Install cargo-release for version management"
	@echo "  $(GREEN)make release VERSION=0.2.2$(RESET)  Bump all crate versions and tag release"
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
	@echo "  $(GREEN)make backend-build$(RESET) Build backend release (offline mode)"
	@echo "  $(GREEN)make backend-build-online$(RESET) Build with live DB verification"
	@echo "  $(GREEN)make backend-sqlx-prepare$(RESET) Generate SQLx metadata for offline builds"
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
	@echo "  $(GREEN)make docker-up$(RESET)    Start full stack via Docker (frontend + backend + DB)"
	@echo "  $(GREEN)make docker-down$(RESET)  Stop Docker stack"
	@echo "  $(GREEN)make docker-build$(RESET) Rebuild Docker images"
	@echo "  $(GREEN)make docker-logs$(RESET)  View Docker logs"
	@echo "  $(GREEN)make docker-ps$(RESET)    Show Docker container status"
	@echo ""
	@echo "$(BOLD)$(BLUE)📦 SDKs$(RESET)"
	@echo "  $(GREEN)make sdk-rust-build$(RESET)    Build Rust SDK (sdks/rust)"
	@echo "  $(GREEN)make sdk-rust-publish$(RESET)  Publish Rust SDK (sdks/rust) to crates.io"
	@echo "  $(GREEN)make sdk-rust-version$(RESET)  Update Rust SDK version (VERSION=...)"
	@echo "  $(GREEN)make sdk-python-build$(RESET)    Build Python SDK (sdks/python)"
	@echo "  $(GREEN)make sdk-python-publish$(RESET)  Publish Python SDK (sdks/python) to PyPI"
	@echo "  $(GREEN)make sdk-python-version$(RESET)  Update Python SDK version (VERSION=...)"
	@echo "  $(GREEN)make sdk-typescript-build$(RESET)    Build TypeScript SDK (sdks/typescript)"
	@echo "  $(GREEN)make sdk-typescript-publish$(RESET)  Publish TypeScript SDK (sdks/typescript) to npm"
	@echo "  $(GREEN)make sdk-typescript-version$(RESET)  Update TypeScript SDK version (VERSION=...)"
	@echo "  $(GREEN)make sdk-java-build$(RESET)         Build Java SDK (sdks/java)"
	@echo "  $(GREEN)make sdk-java-publish$(RESET)       Publish Java SDK (sdks/java) to Maven Central"
	@echo "  $(GREEN)make sdk-java-version$(RESET)       Update Java SDK version (VERSION=...)"
	@echo "  $(GREEN)make sdk-kotlin-build$(RESET)       Build Kotlin SDK (sdks/kotlin)"
	@echo "  $(GREEN)make sdk-kotlin-publish$(RESET)     Publish Kotlin SDK (sdks/kotlin) to Maven Central"
	@echo "  $(GREEN)make sdk-kotlin-version$(RESET)     Update Kotlin SDK version (VERSION=...)"
	@echo ""
	@echo "$(BOLD)$(BLUE)🧹 Maintenance$(RESET)"
	@echo "  $(GREEN)make clean$(RESET)        Clean build artifacts"
	@echo "  $(GREEN)make lint$(RESET)         Lint all code"
	@echo "  $(GREEN)make format$(RESET)       Format all code"
	@echo "  $(GREEN)make test$(RESET)         Run all tests"
	@echo ""
	@echo "$(BOLD)$(BLUE)🛡️  Test Quality Gates (OODA-286+)$(RESET)"
	@echo "  $(GREEN)make test-quality$(RESET)     Run all quality gates"
	@echo "  $(GREEN)make test-invariants$(RESET)  Run invariant tests (INV-001 to INV-010)"
	@echo "  $(GREEN)make test-timing$(RESET)      Check test timing (<30s)"
	@echo "  $(GREEN)make test-count$(RESET)       Verify test count (>=2600)"
	@echo "  $(GREEN)make test-flaky$(RESET)       Detect flaky tests"
	@echo "  $(GREEN)make test-e2e-critical$(RESET) Run E2E critical path"
	@echo "  $(GREEN)make test-e2e-full$(RESET)    Run full E2E suite"
	@echo ""

# ============================================================================
# Dependency Checks
# ============================================================================

# ============================================================================
# SDKs (Language-specific)
# ============================================================================

.PHONY: sdk-rust-build sdk-rust-publish

sdk-rust-build: ## Build the Rust SDK (sdks/rust)
	@echo "$(BOLD)$(BLUE)🔨 Building Rust SDK (sdks/rust)$(RESET)"
	cd sdks/rust && cargo build --release

sdk-rust-publish: ## Publish the Rust SDK (sdks/rust) to crates.io
	@echo "$(BOLD)$(BLUE)🚀 Publishing Rust SDK (sdks/rust) to crates.io$(RESET)"
	cd sdks/rust && cargo publish



check-deps: ## Check that required dependencies are installed
	@echo "$(BLUE)Checking dependencies...$(RESET)"
	@command -v cargo >/dev/null 2>&1 || { echo "$(RED)❌ cargo not found. Install Rust: https://rustup.rs$(RESET)"; exit 1; }
	@command -v bun >/dev/null 2>&1 || command -v npm >/dev/null 2>&1 || { echo "$(RED)❌ bun/npm not found. Install Node.js/Bun$(RESET)"; exit 1; }
	@command -v docker >/dev/null 2>&1 || { echo "$(YELLOW)⚠️  docker not found. Some features require Docker$(RESET)"; }
	@echo "$(GREEN)✓ All required dependencies found$(RESET)"

check-ports: ## Check and clear ports 8080 and 3000 if in use
	@echo "$(BLUE)Checking ports 8080 and 3000...$(RESET)"
	@PORT_8080=$$(lsof -ti:8080 2>/dev/null || true); \
	PORT_3000=$$(lsof -ti:3000 2>/dev/null || true); \
	if [ -n "$$PORT_8080" ]; then \
		echo "$(YELLOW)⚠️  Port 8080 in use by PID $$PORT_8080 - killing...$(RESET)"; \
		kill -9 $$PORT_8080 2>/dev/null || true; \
		sleep 1; \
	fi; \
	if [ -n "$$PORT_3000" ]; then \
		echo "$(YELLOW)⚠️  Port 3000 in use by PID $$PORT_3000 - killing...$(RESET)"; \
		kill -9 $$PORT_3000 2>/dev/null || true; \
		sleep 1; \
	fi
	@echo "$(GREEN)✓ Ports 8080 and 3000 are available$(RESET)"

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

dev: check-deps check-ports ## Start full development stack (DB + Backend + Frontend) with Ollama
	@echo ""
	@echo "$(BOLD)$(BLUE)🚀 Starting EdgeQuake Development Stack$(RESET)"
	@# OODA-09: Dynamically select provider based on OPENAI_API_KEY
	@if [ -n "$(OPENAI_API_KEY)" ]; then \
		echo "$(BOLD)$(YELLOW)📝 Using OpenAI provider (OPENAI_API_KEY detected)$(RESET)"; \
	else \
		echo "$(BOLD)$(YELLOW)📝 Using Ollama as default LLM provider$(RESET)"; \
	fi
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
	@if [ -n "$(OPENAI_API_KEY)" ]; then \
		echo "  $(BLUE)Provider$(RESET): OpenAI"; \
	else \
		echo "  $(BLUE)Provider$(RESET): Ollama (http://localhost:11434)"; \
	fi
	@echo ""
	@echo "$(GREEN)✓ Services starting...$(RESET)"
	@echo "$(YELLOW)Press Ctrl+C to stop all services$(RESET)"
	@echo ""
	@trap 'echo ""; echo "$(YELLOW)Stopping services...$(RESET)"; $(MAKE) stop --no-print-directory; exit 0' INT; \
	if [ -n "$(OPENAI_API_KEY)" ]; then \
		(cd $(BACKEND_DIR) && \
			DATABASE_URL="postgresql://edgequake:edgequake_secret@localhost:5432/edgequake" \
			OPENAI_API_KEY="$(OPENAI_API_KEY)" \
			cargo run 2>&1 | sed 's/^/[backend] /') & \
		BACKEND_PID=$$!; \
	else \
		(cd $(BACKEND_DIR) && \
			DATABASE_URL="postgresql://edgequake:edgequake_secret@localhost:5432/edgequake" \
			OLLAMA_HOST="http://localhost:11434" \
			OLLAMA_MODEL="gemma3:latest" \
			OLLAMA_EMBEDDING_MODEL="nomic-embed-text" \
			cargo run 2>&1 | sed 's/^/[backend] /') & \
		BACKEND_PID=$$!; \
	fi; \
	(sleep 5 && cd $(FRONTEND_DIR) && (bun run dev 2>/dev/null || npm run dev) 2>&1 | sed 's/^/[frontend] /') & \
	FRONTEND_PID=$$!; \
	echo "$(GREEN)✓ Backend PID: $$BACKEND_PID, Frontend PID: $$FRONTEND_PID$(RESET)"; \
	wait

dev-frontend: ## Start only frontend dev server
	@$(MAKE) frontend-dev --no-print-directory

dev-backend: ## Start only backend dev server (with database)
	@$(MAKE) db-start --no-print-directory
	@$(MAKE) backend-dev --no-print-directory

dev-memory: check-deps check-ports ## Start development with in-memory storage (for testing)
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

dev-bg: check-deps check-ports ## Start full development stack in BACKGROUND (agentic mode)
	@echo ""
	@echo "$(BOLD)$(BLUE)🤖 Starting EdgeQuake in Background Mode (Agentic)$(RESET)"
	@if [ -n "$(OPENAI_API_KEY)" ]; then \
		echo "$(BOLD)$(YELLOW)📝 Using OpenAI provider$(RESET)"; \
	else \
		echo "$(BOLD)$(YELLOW)📝 Using Ollama as default LLM provider$(RESET)"; \
	fi
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
	@# Use EDGEQUAKE_LLM_PROVIDER (not _DEFAULT_) to explicitly select provider
	@# When OPENAI_API_KEY is set, use OpenAI. Otherwise use Ollama.
	@if [ -n "$(OPENAI_API_KEY)" ]; then \
		cd $(BACKEND_DIR) && \
			DATABASE_URL="$(DATABASE_URL)" \
			OPENAI_API_KEY="$(OPENAI_API_KEY)" \
			EDGEQUAKE_LLM_PROVIDER="openai" \
			nohup cargo run > /tmp/edgequake-backend.log 2>&1 & \
	else \
		cd $(BACKEND_DIR) && \
			DATABASE_URL="$(DATABASE_URL)" \
			EDGEQUAKE_LLM_PROVIDER="ollama" \
			OLLAMA_HOST="http://localhost:11434" \
			OLLAMA_MODEL="gemma3:latest" \
			OLLAMA_EMBEDDING_MODEL="nomic-embed-text" \
			nohup cargo run > /tmp/edgequake-backend.log 2>&1 & \
	fi
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
	@if [ -n "$(OPENAI_API_KEY)" ]; then \
		echo "  $(BLUE)LLM Provider$(RESET): openai (gpt-5-nano)"; \
		echo "  $(BLUE)Embedding$(RESET): openai (text-embedding-3-small, 1536d)"; \
	else \
		echo "  $(BLUE)LLM Provider$(RESET): ollama (gemma3:latest)"; \
		echo "  $(BLUE)Embedding$(RESET): ollama (nomic-embed-text, 768d)"; \
	fi
	@echo ""
	@echo "  Use $(BOLD)make status$(RESET) to check service health"
	@echo "  Use $(BOLD)make stop$(RESET) to stop all services"
	@echo ""

stop: ## Stop all development services
	@echo "$(YELLOW)Stopping services...$(RESET)"
	@echo "$(BLUE)→ Stopping backend processes...$(RESET)"
	@-pkill -f "cargo run" 2>/dev/null || true
	@-pkill -9 -f "edgequake-api" 2>/dev/null || true
	@-pkill -9 -f "target/debug/edgequake" 2>/dev/null || true
	@-pkill -9 -f "target/release/edgequake" 2>/dev/null || true
	@# OODA-256: Force-kill any process on port 8080
	@-lsof -ti:8080 | xargs -r kill -9 2>/dev/null || true
	@echo "$(BLUE)→ Stopping frontend processes...$(RESET)"
	@-pkill -f "next dev" 2>/dev/null || true
	@-pkill -f "node.*edgequake_webui" 2>/dev/null || true
	@-pkill -9 -f "bun.*dev" 2>/dev/null || true
	@-pkill -9 -f "next-server" 2>/dev/null || true
	@# OODA-256: Force-kill any process on port 3000
	@-lsof -ti:3000 | xargs -r kill -9 2>/dev/null || true
	@echo "$(BLUE)→ Stopping database...$(RESET)"
	@$(MAKE) db-stop --no-print-directory 2>/dev/null || true
	@sleep 1
	@echo "$(GREEN)✓ All services stopped$(RESET)"

# ============================================================================
# Backend
# ============================================================================

# Database URL for PostgreSQL mode
DATABASE_URL := postgresql://edgequake:edgequake_secret@localhost:5432/edgequake

# SPEC-040 v0.4.1: pdfium is now EMBEDDED in the edgequake-pdf2md 0.4.1 binary
# via pdfium-auto at compile time. No external libpdfium.dylib, no env vars needed.

backend-dev: db-wait ## Run backend in development mode with PostgreSQL (uses .env configuration)
	@echo "$(BLUE)Starting backend with PostgreSQL storage...$(RESET)"
	@if [ -n "$(EDGEQUAKE_DEFAULT_LLM_PROVIDER)" ]; then \
		echo "$(GREEN)✓ LLM Provider: $(EDGEQUAKE_DEFAULT_LLM_PROVIDER) ($(EDGEQUAKE_DEFAULT_LLM_MODEL))$(RESET)"; \
	fi
	@cd $(BACKEND_DIR) && \
		DATABASE_URL="$(DATABASE_URL)" \
		OPENAI_API_KEY="$(OPENAI_API_KEY)" \
		EDGEQUAKE_DEFAULT_LLM_PROVIDER="$(EDGEQUAKE_DEFAULT_LLM_PROVIDER)" \
		EDGEQUAKE_DEFAULT_LLM_MODEL="$(EDGEQUAKE_DEFAULT_LLM_MODEL)" \
		EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER="$(EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER)" \
		EDGEQUAKE_DEFAULT_EMBEDDING_MODEL="$(EDGEQUAKE_DEFAULT_EMBEDDING_MODEL)" \
		EDGEQUAKE_DEFAULT_EMBEDDING_DIMENSION="$(EDGEQUAKE_DEFAULT_EMBEDDING_DIMENSION)" \
		EDGEQUAKE_VISION_PROVIDER="$(EDGEQUAKE_VISION_PROVIDER)" \
		EDGEQUAKE_VISION_MODEL="$(EDGEQUAKE_VISION_MODEL)" \
		OLLAMA_HOST="http://localhost:11434" \
		OLLAMA_MODEL="gemma3:latest" \
		OLLAMA_EMBEDDING_MODEL="nomic-embed-text" \
		cargo run

backend-db: db-wait ## Run backend with PostgreSQL storage (uses .env configuration)
	@echo "$(BLUE)Starting backend with PostgreSQL storage (explicit)...$(RESET)"
	@if [ -n "$(EDGEQUAKE_DEFAULT_LLM_PROVIDER)" ]; then \
		echo "$(GREEN)✓ LLM Provider: $(EDGEQUAKE_DEFAULT_LLM_PROVIDER) ($(EDGEQUAKE_DEFAULT_LLM_MODEL))$(RESET)"; \
	fi
	@cd $(BACKEND_DIR) && \
		DATABASE_URL="$(DATABASE_URL)" \
		OPENAI_API_KEY="$(OPENAI_API_KEY)" \
		EDGEQUAKE_DEFAULT_LLM_PROVIDER="$(EDGEQUAKE_DEFAULT_LLM_PROVIDER)" \
		EDGEQUAKE_DEFAULT_LLM_MODEL="$(EDGEQUAKE_DEFAULT_LLM_MODEL)" \
		EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER="$(EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER)" \
		EDGEQUAKE_DEFAULT_EMBEDDING_MODEL="$(EDGEQUAKE_DEFAULT_EMBEDDING_MODEL)" \
		EDGEQUAKE_DEFAULT_EMBEDDING_DIMENSION="$(EDGEQUAKE_DEFAULT_EMBEDDING_DIMENSION)" \
		EDGEQUAKE_VISION_PROVIDER="$(EDGEQUAKE_VISION_PROVIDER)" \
		EDGEQUAKE_VISION_MODEL="$(EDGEQUAKE_VISION_MODEL)" \
		OLLAMA_HOST="http://localhost:11434" \
		OLLAMA_MODEL="gemma3:latest" \
		OLLAMA_EMBEDDING_MODEL="nomic-embed-text" \
		cargo run

# OODA-03: In-memory storage has been REMOVED for production consistency.
# This target now fails with guidance to use PostgreSQL instead.
backend-memory: ## DEPRECATED - In-memory storage removed, use backend-dev with PostgreSQL
	@echo "$(RED)╔══════════════════════════════════════════════════════════════════╗$(RESET)"
	@echo "$(RED)║  ❌  ERROR: In-memory storage has been REMOVED                   ║$(RESET)"
	@echo "$(RED)║                                                                  ║$(RESET)"
	@echo "$(RED)║  The mission directive requires PostgreSQL for all operations.  ║$(RESET)"
	@echo "$(RED)║  Please use one of these alternatives:                          ║$(RESET)"
	@echo "$(RED)║                                                                  ║$(RESET)"
	@echo "$(RED)║    make dev          # Full stack with PostgreSQL               ║$(RESET)"
	@echo "$(RED)║    make backend-dev  # Backend only with PostgreSQL             ║$(RESET)"
	@echo "$(RED)║                                                                  ║$(RESET)"
	@echo "$(RED)╚══════════════════════════════════════════════════════════════════╝$(RESET)"
	@exit 1

backend-bg: db-wait ## Run backend in background with PostgreSQL (respects OPENAI_API_KEY if set)
	@echo "$(BLUE)Starting backend in background...$(RESET)"
	@if [ -n "$(OPENAI_API_KEY)" ]; then \
		echo "$(YELLOW)→ OPENAI_API_KEY detected - using OpenAI as default provider$(RESET)"; \
		printf '%s\n' "#!/bin/bash" > /tmp/edgequake-start.sh; \
		printf '%s\n' "export DATABASE_URL=\"$(DATABASE_URL)\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export OPENAI_API_KEY=\"$(OPENAI_API_KEY)\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export EDGEQUAKE_LLM_PROVIDER=\"openai\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "cd $(BACKEND_DIR) && exec cargo run" >> /tmp/edgequake-start.sh; \
		chmod +x /tmp/edgequake-start.sh; \
		nohup /tmp/edgequake-start.sh > /tmp/edgequake-backend.log 2>&1 & \
	else \
		echo "$(YELLOW)→ No OPENAI_API_KEY, using Ollama provider$(RESET)"; \
		printf '%s\n' "#!/bin/bash" > /tmp/edgequake-start.sh; \
		printf '%s\n' "export DATABASE_URL=\"$(DATABASE_URL)\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export EDGEQUAKE_LLM_PROVIDER=\"ollama\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export OLLAMA_HOST=\"http://localhost:11434\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export OLLAMA_MODEL=\"gemma3:latest\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export OLLAMA_EMBEDDING_MODEL=\"nomic-embed-text\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "cd $(BACKEND_DIR) && exec cargo run" >> /tmp/edgequake-start.sh; \
		chmod +x /tmp/edgequake-start.sh; \
		nohup /tmp/edgequake-start.sh > /tmp/edgequake-backend.log 2>&1 & \
	fi
	@echo "$(GREEN)✓ Backend starting in background. Log: /tmp/edgequake-backend.log$(RESET)"

backend-build: ## Build backend for release (offline mode)
	@echo "$(BLUE)Building backend in offline mode...$(RESET)"
	@cd $(BACKEND_DIR) && SQLX_OFFLINE=true cargo build --release
	@echo "$(GREEN)✓ Backend built: $(BACKEND_DIR)/target/release/edgequake$(RESET)"

backend-build-online: db-start ## Build backend with live database verification
	@echo "$(BLUE)Building backend with live DB verification...$(RESET)"
	@cd $(BACKEND_DIR) && \
		DATABASE_URL="postgresql://edgequake:edgequake_secret@localhost:5432/edgequake" \
		cargo build --release
	@echo "$(GREEN)✓ Backend built with DB verification$(RESET)"

backend-sqlx-prepare: db-start ## Generate SQLx metadata for offline builds
	@echo "$(BLUE)Preparing SQLx metadata from database...$(RESET)"
	@cd $(BACKEND_DIR) && \
		DATABASE_URL="postgresql://edgequake:edgequake_secret@localhost:5432/edgequake" \
		cargo sqlx prepare --workspace
	@echo "$(GREEN)✓ SQLx metadata prepared in .sqlx/$(RESET)"

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
	@echo ""
	@echo "$(BOLD)$(BLUE)🐳 Starting EdgeQuake Full Stack via Docker$(RESET)"
	@echo ""
	@echo "$(YELLOW)→ Building and starting services...$(RESET)"
	@echo ""
	@cd $(DOCKER_DIR) && docker compose up -d
	@echo ""
	@echo "$(YELLOW)→ Waiting for services to be ready...$(RESET)"
	@sleep 5
	@echo ""
	@echo "$(BOLD)$(GREEN)✅ EdgeQuake Docker Stack is Running$(RESET)"
	@echo ""
	@echo "$(BOLD)📍 Access Points:$(RESET)"
	@echo ""
	@echo "  $(BLUE)Frontend (Web UI)$(RESET)"
	@echo "    🌐 URL: $(BOLD)http://localhost:3000$(RESET)"
	@echo "    📝 Navigate here to upload documents and interact with the knowledge graph"
	@echo ""
	@echo "  $(BLUE)Backend API$(RESET)"
	@echo "    🔗 URL: $(BOLD)http://localhost:8080$(RESET)"
	@echo "    📚 Swagger UI: $(BOLD)http://localhost:8080/swagger-ui$(RESET)"
	@echo "    🏥 Health: $(BOLD)http://localhost:8080/health$(RESET)"
	@echo ""
	@echo "  $(BLUE)Database$(RESET)"
	@echo "    🗄️  PostgreSQL on port 5432"
	@echo "    👤 User: edgequake"
	@echo ""
	@echo "$(YELLOW)→ First Time:$(RESET)"
	@echo "  1. Open http://localhost:3000 in your browser"
	@echo "  2. Upload a PDF document from the File menu"
	@echo "  3. Wait for entity extraction to complete"
	@echo "  4. View the knowledge graph and extracted entities"
	@echo ""
	@echo "$(YELLOW)→ Management:$(RESET)"
	@echo "  $(BOLD)See logs:$(RESET) make docker-logs"
	@echo "  $(BOLD)Stop stack:$(RESET) make docker-down"
	@echo "  $(BOLD)Check status:$(RESET) make docker-ps"
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
# Test Quality Gates (OODA-286+)
# ============================================================================

test-quality: test-invariants test-timing test-count ## Run all quality gate checks
	@echo "$(GREEN)✓ All quality gates passed$(RESET)"

test-invariants: ## Run critical invariant tests (INV-001 to INV-010)
	@echo "$(BLUE)Running critical invariant tests...$(RESET)"
	@cd $(BACKEND_DIR) && cargo test --package edgequake-core --test inviolable_invariants 2>&1 | tee /tmp/invariant_results.txt
	@cd $(BACKEND_DIR) && cargo test --package edgequake-core --test edge_case_invariants 2>&1 | tee -a /tmp/invariant_results.txt
	@cd $(BACKEND_DIR) && cargo test --package edgequake-api --test integration_invariants 2>&1 | tee -a /tmp/invariant_results.txt
	@if grep -q "FAILED" /tmp/invariant_results.txt; then \
		echo "$(RED)CRITICAL: Invariant tests failed!$(RESET)"; \
		exit 1; \
	fi
	@echo "$(GREEN)✓ All invariant tests passed$(RESET)"

test-timing: ## Check test suite timing (Target: <30s for unit tests)
	@echo "$(BLUE)Running timing check...$(RESET)"
	@START=$$(date +%s); \
	cd $(BACKEND_DIR) && cargo test --lib --all --quiet 2>&1 > /dev/null; \
	END=$$(date +%s); \
	DURATION=$$((END - START)); \
	echo "Unit tests completed in $${DURATION}s"; \
	if [ $$DURATION -gt 30 ]; then \
		echo "$(YELLOW)Warning: Unit tests exceeded 30s threshold$(RESET)"; \
	else \
		echo "$(GREEN)✓ Timing target met ($${DURATION}s < 30s)$(RESET)"; \
	fi

test-count: ## Verify minimum test count (Target: >=2600)
	@echo "$(BLUE)Counting tests...$(RESET)"
	@cd $(BACKEND_DIR) && cargo test --all 2>&1 | grep "test result:" | awk '{sum += $$4} END {print "Total passed:", sum}' | tee /tmp/test_count.txt
	@TOTAL=$$(cat /tmp/test_count.txt | grep -oE '[0-9]+' | head -1); \
	if [ "$$TOTAL" -lt 2600 ]; then \
		echo "$(RED)CRITICAL: Test count below 2600 threshold (got: $$TOTAL)$(RESET)"; \
		exit 1; \
	fi
	@echo "$(GREEN)✓ Test count gate passed$(RESET)"

test-flaky: ## Run flaky test detection (3 iterations)
	@echo "$(BLUE)Running flaky test detection...$(RESET)"
	@./scripts/detect_flaky_tests.sh 3 all

test-e2e-critical: ## Run E2E critical path tests
	@echo "$(BLUE)Running E2E critical path tests...$(RESET)"
	@cd $(FRONTEND_DIR) && PLAYWRIGHT_BASE_URL=http://localhost:3000 \
		pnpm exec playwright test ooda-228-critical-path.spec.ts --reporter=line

test-e2e-full: ## Run full E2E test suite
	@echo "$(BLUE)Running full E2E suite...$(RESET)"
	@cd $(FRONTEND_DIR) && PLAYWRIGHT_BASE_URL=http://localhost:3000 \
		pnpm exec playwright test --reporter=line

test-stability-report: ## Generate test stability report
	@echo "$(BLUE)Generating stability report...$(RESET)"
	@cd $(BACKEND_DIR) && cargo test --all 2>&1 | tee /tmp/full_test_output.txt
	@echo "Test results saved to /tmp/full_test_output.txt"
	@echo "$(GREEN)✓ See docs/TEST_STABILITY_REPORT.md for detailed analysis$(RESET)"

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
