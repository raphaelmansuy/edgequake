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


.PHONY: help install dev dev-auth dev-bg dev-auth-bg dev-memory stop clean build test lint format \
        backend-dev backend-db backend-memory backend-bg backend-build backend-build-online backend-sqlx-prepare backend-test backend-run \
        frontend-dev frontend-bg frontend-build frontend-test frontend-lint \
        db-start db-stop db-wait db-logs db-shell docker-network-diagnose stop-docker-services \
        docker-build docker-up docker-prebuilt docker-prebuilt-down docker-prebuilt-logs docker-ps-prebuilt docker-api-only docker-down docker-logs \
        stack stack-down stack-logs stack-status stack-restart stack-pull \
        check-deps status \
        test-quality test-invariants test-timing test-count test-flaky \
        test-e2e-critical test-e2e-full test-stability-report \
        sdk-e2e sdk-e2e-with-stack sdk-csharp-test-unit

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

# Local development ports.
# WHY: Local EdgeQuake and the published Docker stack both document the Web UI
# on localhost:3000. Keep that as the primary development default, then shift to
# the next safe free port only when 3000 is already occupied.
DEFAULT_BACKEND_PORT ?= 8080
DEFAULT_FRONTEND_PORT ?= 3000
PORT_SCAN_WINDOW ?= 20
ifndef BACKEND_PORT
BACKEND_PORT := $(shell python3 $(ROOT_DIR)/scripts/select_edgequake_port.py backend $(DEFAULT_BACKEND_PORT) $(PORT_SCAN_WINDOW))
endif
ifndef FRONTEND_PORT
FRONTEND_PORT := $(shell python3 $(ROOT_DIR)/scripts/select_edgequake_port.py frontend $(DEFAULT_FRONTEND_PORT) $(PORT_SCAN_WINDOW))
endif
BACKEND_URL := http://localhost:$(BACKEND_PORT)
FRONTEND_URL := http://localhost:$(FRONTEND_PORT)

# WHY: A fixed Compose project name keeps the local Docker network/container
# namespace stable across repeated invocations and different working directories.
# This reduces needless network churn and makes startup behavior more deterministic.
COMPOSE_PROJECT_NAME ?= edgequake-dev
export COMPOSE_PROJECT_NAME

# Load environment variables from .env file if it exists
-include $(ROOT_DIR)/.env
export

# Environment variables (can be overridden from shell)
OPENAI_API_KEY ?= $(shell echo $$OPENAI_API_KEY)
DEV_AUTH_ENABLED ?= false
DEV_DISABLE_DEMO_LOGIN ?= false

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
  EDGEQUAKE_DEFAULT_LLM_MODEL ?= gemma4:latest
  EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER ?= ollama
  EDGEQUAKE_DEFAULT_EMBEDDING_MODEL ?= embeddinggemma:latest
  EDGEQUAKE_DEFAULT_EMBEDDING_DIMENSION ?= 768
endif

# SPEC-040: Vision/VLM provider defaults for PDF-to-Markdown conversion
# WHY: Vision provider MUST inherit from the resolved DEFAULT_LLM values (set above,
# potentially overridden by .env).  Previous code had a separate ifdef that could
# produce a provider/model mismatch (e.g. .env → ollama but vision → gpt-4.1-nano).
# First Principle: ONE source of truth for "which provider am I using?"
EDGEQUAKE_VISION_PROVIDER ?= $(EDGEQUAKE_DEFAULT_LLM_PROVIDER)
EDGEQUAKE_VISION_MODEL    ?= $(EDGEQUAKE_DEFAULT_LLM_MODEL)

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
	@echo "  $(GREEN)make dev$(RESET)          Start full development stack without authentication (default local mode)"
	@echo "  $(GREEN)make dev-auth$(RESET)     Start full development stack with authentication enabled"
	@echo "  $(GREEN)make dev-bg$(RESET)       Start full stack in BACKGROUND without authentication"
	@echo "  $(GREEN)make dev-auth-bg$(RESET)  Start full stack in BACKGROUND with authentication enabled"
	@echo "  $(GREEN)make dev-memory$(RESET)   Start with in-memory storage (for testing)"
	@echo "  $(GREEN)make stop$(RESET)         Stop all services"
	@echo "  $(GREEN)make status$(RESET)       Check status of all services"
	@echo ""
	@echo "$(BOLD)$(BLUE)⚡ One-Command Docker Stack (no build needed)$(RESET)"
	@echo "  $(GREEN)make stack$(RESET)        Pull GHCR images and start API+UI+DB  (~30s)"
	@echo "  $(GREEN)make stack-down$(RESET)   Stop and remove stack containers"
	@echo "  $(GREEN)make stack-logs$(RESET)   Tail logs from all stack containers"
	@echo "  $(GREEN)make stack-status$(RESET) Show container status"
	@echo "  $(GREEN)make stack-pull$(RESET)   Pull latest images without starting"
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
	@echo "  $(GREEN)make docker-up$(RESET)               Start full stack via Docker (build from source)"
	@echo "  $(GREEN)make docker-prebuilt$(RESET)         Start full stack using prebuilt GHCR images (fastest, no build)"
	@echo "  $(GREEN)make docker-prebuilt-down$(RESET)    Stop prebuilt stack"
	@echo "  $(GREEN)make docker-prebuilt-logs$(RESET)    View prebuilt stack logs"
	@echo "  $(GREEN)make docker-ps-prebuilt$(RESET)      Show prebuilt stack container status"
	@echo "  $(GREEN)make docker-api-only$(RESET)         Start API only (bring your own PostgreSQL)"
	@echo "  $(GREEN)make docker-down$(RESET)             Stop Docker stack (build-from-source)"
	@echo "  $(GREEN)make docker-build$(RESET)            Rebuild Docker images"
	@echo "  $(GREEN)make docker-logs$(RESET)             View Docker logs"
	@echo "  $(GREEN)make docker-ps$(RESET)               Show Docker container status"
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
	@echo "  $(GREEN)make sdk-e2e$(RESET)          Run Rust/Python/TS SDK E2E vs SDK_E2E_URL (needs healthy API)"
	@echo "  $(GREEN)make sdk-e2e-with-stack$(RESET)  $(GREEN)make stack$(RESET) then SDK E2E (Docker quickstart)"
	@echo "  $(GREEN)make sdk-csharp-test-unit$(RESET)  C# SDK unit tests only (excludes E2E trait)"
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
	@command -v pnpm >/dev/null 2>&1 || command -v bun >/dev/null 2>&1 || { echo "$(RED)❌ pnpm/bun not found. Install pnpm or Bun$(RESET)"; exit 1; }
	@command -v docker >/dev/null 2>&1 || { echo "$(YELLOW)⚠️  docker not found. Some features require Docker$(RESET)"; }
	@echo "$(GREEN)✓ All required dependencies found$(RESET)"

check-ports: ## Validate configured ports without killing unrelated processes
	@echo "$(BLUE)Checking selected ports $(BACKEND_PORT) and $(FRONTEND_PORT)...$(RESET)"
	@if [ "$(BACKEND_PORT)" != "$(DEFAULT_BACKEND_PORT)" ]; then \
		echo "$(YELLOW)→ Preferred backend port $(DEFAULT_BACKEND_PORT) is busy; using $(BACKEND_PORT) to avoid interference$(RESET)"; \
	fi
	@if [ "$(FRONTEND_PORT)" != "$(DEFAULT_FRONTEND_PORT)" ]; then \
		echo "$(YELLOW)→ Preferred frontend port $(DEFAULT_FRONTEND_PORT) is busy; using $(FRONTEND_PORT) instead$(RESET)"; \
		echo "$(YELLOW)  Open $(FRONTEND_URL) in your browser for this session$(RESET)"; \
	fi
	@for port in $(BACKEND_PORT) $(FRONTEND_PORT); do \
		PID=$$(lsof -nP -iTCP:$$port -sTCP:LISTEN -t 2>/dev/null | head -n 1 || true); \
		if [ -z "$$PID" ]; then \
			continue; \
		fi; \
		CMD=$$(ps -p "$$PID" -o command= 2>/dev/null || true); \
		if [ "$$port" = "$(BACKEND_PORT)" ] && curl -fsS "$(BACKEND_URL)/health" 2>/dev/null | grep -q '"status"'; then \
			echo "$(YELLOW)→ Port $(BACKEND_PORT) is already serving EdgeQuake; reusing it$(RESET)"; \
			continue; \
		fi; \
		if [ "$$port" = "$(FRONTEND_PORT)" ] && curl -fsS "$(FRONTEND_URL)" 2>/dev/null | grep -qi 'EdgeQuake'; then \
			echo "$(YELLOW)→ Port $(FRONTEND_PORT) is already serving the EdgeQuake UI; reusing it$(RESET)"; \
			continue; \
		fi; \
		echo "$(RED)✗ Selected port $$port is already bound by another application$(RESET)"; \
		echo "  PID: $$PID"; \
		echo "  CMD: $$CMD"; \
		echo "  Hint: EdgeQuake auto-selects safe ports, but you can also override BACKEND_PORT or FRONTEND_PORT explicitly."; \
		exit 1; \
	done
	@echo "$(GREEN)✓ Port check complete$(RESET)"

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
	@cd $(FRONTEND_DIR) && pnpm install 2>/dev/null || bun install
	@echo "$(GREEN)✓ Frontend dependencies installed$(RESET)"
	@echo ""
	@echo "$(BOLD)$(GREEN)✅ All dependencies installed!$(RESET)"
	@echo ""

# ============================================================================
# Development
# ============================================================================

dev: check-deps check-ports ## Start full development stack without authentication
	@echo ""
	@echo "$(BOLD)$(BLUE)🚀 Starting EdgeQuake Development Stack$(RESET)"
	@echo "$(YELLOW)→ Incremental startup: healthy services are reused; nothing is killed blindly$(RESET)"
	@# OODA-09: Dynamically select provider based on OPENAI_API_KEY
	@if [ -n "$(OPENAI_API_KEY)" ]; then \
		echo "$(BOLD)$(YELLOW)📝 Using OpenAI provider (OPENAI_API_KEY detected)$(RESET)"; \
	else \
		echo "$(BOLD)$(YELLOW)📝 Using Ollama as default LLM provider$(RESET)"; \
	fi
	@echo ""
	@if curl -fsS "$(BACKEND_URL)/health" >/dev/null 2>&1 && curl -fsS "$(FRONTEND_URL)" 2>/dev/null | grep -qi 'EdgeQuake'; then \
		echo "$(YELLOW)→ Existing EdgeQuake services detected; continuing with reuse checks$(RESET)"; \
	fi
	@echo "$(YELLOW)→ Ensuring PostgreSQL availability...$(RESET)"
	@$(MAKE) db-start --no-print-directory
	@echo ""
	@echo "  $(BLUE)Backend$(RESET):  $(BACKEND_URL)"
	@echo "  $(BLUE)Frontend$(RESET): $(FRONTEND_URL)"
	@echo "  $(BLUE)Swagger$(RESET):  $(BACKEND_URL)/swagger-ui"
	@if [ "$(DEV_AUTH_ENABLED)" = "true" ]; then \
		echo "  $(BLUE)Auth$(RESET):     enabled"; \
	else \
		echo "  $(BLUE)Auth$(RESET):     disabled (default local mode)"; \
	fi
	@if [ -n "$(OPENAI_API_KEY)" ]; then \
		echo "  $(BLUE)Provider$(RESET): OpenAI"; \
	else \
		echo "  $(BLUE)Provider$(RESET): Ollama (http://localhost:11434)"; \
	fi
	@echo ""
	@trap 'echo ""; echo "$(YELLOW)Stopping only the processes started by this make dev session...$(RESET)"; [ -n "$$BACKEND_PID" ] && kill "$$BACKEND_PID" 2>/dev/null || true; [ -n "$$FRONTEND_PID" ] && kill "$$FRONTEND_PID" 2>/dev/null || true; echo "$(GREEN)✓ App processes stopped. PostgreSQL is left running for faster restarts.$(RESET)"; exit 0' INT; \
	BACKEND_PID=""; \
	FRONTEND_PID=""; \
	if curl -fsS "$(BACKEND_URL)/health" >/dev/null 2>&1; then \
		echo "$(GREEN)✓ Reusing running backend on port $(BACKEND_PORT)$(RESET)"; \
	else \
		echo "$(YELLOW)→ Starting backend...$(RESET)"; \
		if [ -n "$(OPENAI_API_KEY)" ]; then \
			(cd $(BACKEND_DIR) && \
				PORT="$(BACKEND_PORT)" \
				DATABASE_URL="$(DATABASE_URL)" \
				OPENAI_API_KEY="$(OPENAI_API_KEY)" \
				EDGEQUAKE_AUTH_ENABLED="$(DEV_AUTH_ENABLED)" \
				AUTH_ENABLED="$(DEV_AUTH_ENABLED)" \
				cargo run 2>&1 | sed 's/^/[backend] /') & \
			BACKEND_PID=$$!; \
		else \
			(cd $(BACKEND_DIR) && \
				PORT="$(BACKEND_PORT)" \
				DATABASE_URL="$(DATABASE_URL)" \
				EDGEQUAKE_AUTH_ENABLED="$(DEV_AUTH_ENABLED)" \
				AUTH_ENABLED="$(DEV_AUTH_ENABLED)" \
				OLLAMA_HOST="http://localhost:11434" \
				OLLAMA_MODEL="gemma4:latest" \
				OLLAMA_EMBEDDING_MODEL="embeddinggemma:latest" \
				cargo run 2>&1 | sed 's/^/[backend] /') & \
			BACKEND_PID=$$!; \
		fi; \
	fi; \
	if curl -fsS "$(FRONTEND_URL)" 2>/dev/null | grep -qi 'EdgeQuake'; then \
		echo "$(GREEN)✓ Reusing running frontend on port $(FRONTEND_PORT)$(RESET)"; \
	else \
		echo "$(YELLOW)→ Starting frontend on port $(FRONTEND_PORT)...$(RESET)"; \
		(sleep 2 && cd $(FRONTEND_DIR) && PORT="$(FRONTEND_PORT)" NEXT_PUBLIC_API_URL="$(BACKEND_URL)" NEXT_PUBLIC_AUTH_ENABLED="$(DEV_AUTH_ENABLED)" NEXT_PUBLIC_DISABLE_DEMO_LOGIN="$(DEV_DISABLE_DEMO_LOGIN)" sh -c '(pnpm run dev 2>/dev/null || bun run dev)' 2>&1 | sed 's/^/[frontend] /') & \
		FRONTEND_PID=$$!; \
	fi; \
	if [ -z "$$BACKEND_PID$$FRONTEND_PID" ]; then \
		echo "$(GREEN)✓ Stack already running; nothing new to start$(RESET)"; \
		exit 0; \
	fi; \
	echo "$(GREEN)✓ Startup in progress$(RESET)"; \
	echo "$(YELLOW)Press Ctrl+C to stop only this session's app processes$(RESET)"; \
	wait

dev-auth: ## Start full development stack with authentication enabled
	@$(MAKE) dev --no-print-directory DEV_AUTH_ENABLED=true DEV_DISABLE_DEMO_LOGIN=true

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
	(sleep 5 && cd $(FRONTEND_DIR) && (pnpm run dev 2>/dev/null || bun run dev) 2>&1 | sed 's/^/[frontend] /') & \
	FRONTEND_PID=$$!; \
	echo "$(GREEN)✓ Backend PID: $$BACKEND_PID, Frontend PID: $$FRONTEND_PID$(RESET)"; \
	wait

dev-bg: check-deps check-ports ## Start full development stack in BACKGROUND without authentication
	@echo ""
	@echo "$(BOLD)$(BLUE)🤖 Starting EdgeQuake in Background Mode (Agentic)$(RESET)"
	@echo "$(YELLOW)→ Incremental startup: healthy services are reused; Docker is touched only when needed$(RESET)"
	@if [ -n "$(OPENAI_API_KEY)" ]; then \
		echo "$(BOLD)$(YELLOW)📝 Using OpenAI provider$(RESET)"; \
	else \
		echo "$(BOLD)$(YELLOW)📝 Using Ollama as default LLM provider$(RESET)"; \
	fi
	@echo ""
	@if curl -fsS "$(BACKEND_URL)/health" >/dev/null 2>&1 && curl -fsS "$(FRONTEND_URL)" 2>/dev/null | grep -qi 'EdgeQuake'; then \
		echo "$(YELLOW)→ Existing EdgeQuake services detected; continuing with reuse checks$(RESET)"; \
	fi
	@echo "$(YELLOW)→ Ensuring PostgreSQL availability...$(RESET)"
	@$(MAKE) db-start --no-print-directory
	@echo ""
	@echo "$(YELLOW)→ Waiting for database...$(RESET)"
	@DB_READY_CMD='pg_isready -h localhost -p 5432'; \
	if ! printf '%s' "$(DATABASE_URL)" | grep -Eiq '@(localhost|127\.0\.0\.1)(:|/)|://(localhost|127\.0\.0\.1)(:|/)'; then \
		DB_READY_CMD='pg_isready -d "$(DATABASE_URL)"'; \
	fi; \
	for i in 1 2 3 4 5 6 7 8 9 10; do \
		eval "$$DB_READY_CMD" >/dev/null 2>&1 && break || sleep 2; \
	done
	@echo ""
	@if curl -fsS "$(BACKEND_URL)/health" >/dev/null 2>&1; then \
		echo "$(GREEN)✓ Backend already healthy on port $(BACKEND_PORT)$(RESET)"; \
	else \
		echo "$(YELLOW)→ Starting backend in background...$(RESET)"; \
		$(MAKE) backend-bg --no-print-directory DEV_AUTH_ENABLED="$(DEV_AUTH_ENABLED)"; \
	fi
	@echo ""
	@echo "$(YELLOW)→ Waiting for backend to start...$(RESET)"
	@BACKEND_OK=""; \
	for i in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30; do \
		if curl -fsS "$(BACKEND_URL)/health" >/dev/null 2>&1; then \
			BACKEND_OK=1; \
			break; \
		fi; \
		if [ -f /tmp/edgequake-backend.pid ] && ! kill -0 "$$(cat /tmp/edgequake-backend.pid)" 2>/dev/null; then \
			echo "$(RED)✗ Backend exited during startup$(RESET)"; \
			tail -n 100 /tmp/edgequake-backend.log; \
			exit 1; \
		fi; \
		sleep 2; \
	done; \
	if [ -z "$$BACKEND_OK" ]; then \
		echo "$(RED)✗ Backend did not become healthy in time$(RESET)"; \
		tail -n 100 /tmp/edgequake-backend.log; \
		exit 1; \
	fi
	@echo ""
	@if curl -fsS "$(FRONTEND_URL)" 2>/dev/null | grep -qi 'EdgeQuake'; then \
		echo "$(GREEN)✓ Frontend already reachable on port $(FRONTEND_PORT)$(RESET)"; \
	else \
		echo "$(YELLOW)→ Starting frontend in background...$(RESET)"; \
		$(MAKE) frontend-bg --no-print-directory DEV_AUTH_ENABLED="$(DEV_AUTH_ENABLED)" DEV_DISABLE_DEMO_LOGIN="$(DEV_DISABLE_DEMO_LOGIN)"; \
	fi
	@echo ""
	@FRONTEND_OK=""; \
	for i in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15; do \
		if curl -fsS "$(FRONTEND_URL)" 2>/dev/null | grep -qi 'EdgeQuake'; then \
			FRONTEND_OK=1; \
			break; \
		fi; \
		if [ -f /tmp/edgequake-frontend.pid ] && ! kill -0 "$$(cat /tmp/edgequake-frontend.pid)" 2>/dev/null; then \
			echo "$(RED)✗ Frontend exited during startup$(RESET)"; \
			tail -n 100 /tmp/edgequake-frontend.log; \
			exit 1; \
		fi; \
		sleep 2; \
	done; \
	if [ -z "$$FRONTEND_OK" ]; then \
		echo "$(RED)✗ Frontend did not become healthy in time$(RESET)"; \
		tail -n 100 /tmp/edgequake-frontend.log; \
		exit 1; \
	fi
	@echo "$(BOLD)$(GREEN)✅ EdgeQuake Background Stack Started$(RESET)"
	@echo ""
	@echo "  $(BLUE)Backend$(RESET):  $(BACKEND_URL)"
	@echo "  $(BLUE)Frontend$(RESET): $(FRONTEND_URL)"
	@echo "  $(BLUE)Swagger$(RESET):  $(BACKEND_URL)/swagger-ui"
	@if [ "$(DEV_AUTH_ENABLED)" = "true" ]; then \
		echo "  $(BLUE)Auth$(RESET): enabled"; \
	else \
		echo "  $(BLUE)Auth$(RESET): disabled (default local mode)"; \
	fi
	@if [ -n "$(OPENAI_API_KEY)" ]; then \
		echo "  $(BLUE)LLM Provider$(RESET): openai (gpt-5-nano)"; \
		echo "  $(BLUE)Embedding$(RESET): openai (text-embedding-3-small, 1536d)"; \
	elif [ -n "$(MISTRAL_API_KEY)" ]; then \
		echo "  $(BLUE)LLM Provider$(RESET): mistral (mistral-small-latest)"; \
		echo "  $(BLUE)Embedding$(RESET): mistral (mistral-embed, 1024d)"; \
		echo "  $(BLUE)Vision$(RESET): mistral (pixtral-large-latest)"; \
	else \
		echo "  $(BLUE)LLM Provider$(RESET): ollama (gemma4:latest)"; \
		echo "  $(BLUE)Embedding$(RESET): ollama (embeddinggemma:latest, 768d)"; \
	fi
	@echo ""
	@echo "  Use $(BOLD)make status$(RESET) to check service health"
	@echo "  Use $(BOLD)make stop$(RESET) to stop all services"
	@echo ""

dev-auth-bg: ## Start full development stack in BACKGROUND with authentication enabled
	@$(MAKE) dev-bg --no-print-directory DEV_AUTH_ENABLED=true DEV_DISABLE_DEMO_LOGIN=true

stop-docker-services: ## Stop Docker/OrbStack-backed EdgeQuake containers if they are running
	@if command -v docker >/dev/null 2>&1 && docker info >/dev/null 2>&1; then \
		echo "$(BLUE)→ Stopping Docker/OrbStack EdgeQuake containers...$(RESET)"; \
		cd $(DOCKER_DIR) && docker compose down --remove-orphans 2>/dev/null || true; \
		cd $(DOCKER_DIR) && docker compose -f docker-compose.prebuilt.yml down --remove-orphans 2>/dev/null || true; \
		docker compose -f $(QUICKSTART_COMPOSE) down --remove-orphans 2>/dev/null || true; \
		docker stop edgequake-api edgequake-frontend edgequake-postgres 2>/dev/null || true; \
	else \
		echo "$(YELLOW)→ Docker daemon unavailable; skipping container stop$(RESET)"; \
	fi

stop: ## Stop all development services
	@echo "$(YELLOW)Stopping services...$(RESET)"
	@echo "$(BLUE)→ Stopping backend processes started by this workspace...$(RESET)"
	@-if [ -f /tmp/edgequake-backend.pid ]; then kill -9 $$(cat /tmp/edgequake-backend.pid) 2>/dev/null || true; fi
	@-pkill -9 -f "target/debug/edgequake" 2>/dev/null || true
	@-pkill -9 -f "target/release/edgequake" 2>/dev/null || true
	@-rm -f /tmp/edgequake-backend.pid /tmp/edgequake-start.sh
	@echo "$(BLUE)→ Stopping frontend processes started by this workspace...$(RESET)"
	@-if [ -f /tmp/edgequake-frontend.pid ]; then kill -9 $$(cat /tmp/edgequake-frontend.pid) 2>/dev/null || true; fi
	@-pkill -f "node.*edgequake_webui" 2>/dev/null || true
	@-rm -f /tmp/edgequake-frontend.pid /tmp/edgequake-frontend-start.sh
	@$(MAKE) stop-docker-services --no-print-directory 2>/dev/null || true
	@BACKEND_STILL_UP=0; FRONTEND_STILL_UP=0; DB_STILL_UP=0; \
	if curl -fsS "$(BACKEND_URL)/health" >/dev/null 2>&1; then BACKEND_STILL_UP=1; fi; \
	if curl -fsS "$(FRONTEND_URL)" >/dev/null 2>&1; then FRONTEND_STILL_UP=1; fi; \
	if pg_isready -h localhost -p 5432 >/dev/null 2>&1; then DB_STILL_UP=1; fi; \
	if [ "$$BACKEND_STILL_UP$$FRONTEND_STILL_UP$$DB_STILL_UP" = "000" ]; then \
		echo "$(GREEN)✓ All services stopped$(RESET)"; \
	else \
		echo "$(YELLOW)⚠ Some EdgeQuake services are still reachable; check 'make status' for details$(RESET)"; \
	fi

# ============================================================================
# Backend
# ============================================================================

# Database URL for PostgreSQL mode.
# WHY: Some shells / .env setups export DATABASE_URL as an empty string, which
# causes the backend to panic with `RelativeUrlWithoutBase`. Treat empty as
# unset and fall back to the local development PostgreSQL container, while
# still respecting any explicit external DATABASE_URL provided by the user.
# WHY ?options=-c%20search_path%3Dpublic: The edgequake schema is created by
# migration 001. PostgreSQL's default search_path "$user",public resolves
# "$user"=edgequake to that schema on subsequent connections. Without forcing
# search_path=public at connection time, sqlx-cli creates _sqlx_migrations in
# the edgequake schema (empty), then migration 001 switches the session path to
# public, and subsequent tracking writes collide with public._sqlx_migrations.
DEFAULT_DATABASE_URL := postgresql://edgequake:edgequake_secret@localhost:5432/edgequake?options=-c%20search_path%3Dpublic
ENV_DATABASE_URL := $(strip $(shell printf '%s' "$$DATABASE_URL"))
ifneq ($(ENV_DATABASE_URL),)
  DATABASE_URL := $(ENV_DATABASE_URL)
endif
ifeq ($(strip $(DATABASE_URL)),)
  DATABASE_URL := $(DEFAULT_DATABASE_URL)
endif
export DATABASE_URL

# SPEC-040 v0.4.1: pdfium is now EMBEDDED in the edgequake-pdf2md 0.4.1 binary
# via pdfium-auto at compile time. No external libpdfium.dylib, no env vars needed.

backend-dev: db-wait ## Run backend in development mode with PostgreSQL (uses .env configuration)
	@echo "$(BLUE)Starting backend with PostgreSQL storage...$(RESET)"
	@if [ -n "$(EDGEQUAKE_DEFAULT_LLM_PROVIDER)" ]; then \
		echo "$(GREEN)✓ LLM Provider: $(EDGEQUAKE_DEFAULT_LLM_PROVIDER) ($(EDGEQUAKE_DEFAULT_LLM_MODEL))$(RESET)"; \
	fi
	@cd $(BACKEND_DIR) && \
		PORT="$(BACKEND_PORT)" \
		DATABASE_URL="$(DATABASE_URL)" \
		OPENAI_API_KEY="$(OPENAI_API_KEY)" \
		EDGEQUAKE_AUTH_ENABLED="$(DEV_AUTH_ENABLED)" \
		AUTH_ENABLED="$(DEV_AUTH_ENABLED)" \
		EDGEQUAKE_DEFAULT_LLM_PROVIDER="$(EDGEQUAKE_DEFAULT_LLM_PROVIDER)" \
		EDGEQUAKE_DEFAULT_LLM_MODEL="$(EDGEQUAKE_DEFAULT_LLM_MODEL)" \
		EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER="$(EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER)" \
		EDGEQUAKE_DEFAULT_EMBEDDING_MODEL="$(EDGEQUAKE_DEFAULT_EMBEDDING_MODEL)" \
		EDGEQUAKE_DEFAULT_EMBEDDING_DIMENSION="$(EDGEQUAKE_DEFAULT_EMBEDDING_DIMENSION)" \
		EDGEQUAKE_VISION_PROVIDER="$(EDGEQUAKE_VISION_PROVIDER)" \
		EDGEQUAKE_VISION_MODEL="$(EDGEQUAKE_VISION_MODEL)" \
		OLLAMA_HOST="http://localhost:11434" \
		OLLAMA_MODEL="gemma4:latest" \
		OLLAMA_EMBEDDING_MODEL="embeddinggemma:latest" \
		cargo run

backend-db: db-wait ## Run backend with PostgreSQL storage (uses .env configuration)
	@echo "$(BLUE)Starting backend with PostgreSQL storage (explicit)...$(RESET)"
	@if [ -n "$(EDGEQUAKE_DEFAULT_LLM_PROVIDER)" ]; then \
		echo "$(GREEN)✓ LLM Provider: $(EDGEQUAKE_DEFAULT_LLM_PROVIDER) ($(EDGEQUAKE_DEFAULT_LLM_MODEL))$(RESET)"; \
	fi
	@cd $(BACKEND_DIR) && \
		PORT="$(BACKEND_PORT)" \
		DATABASE_URL="$(DATABASE_URL)" \
		OPENAI_API_KEY="$(OPENAI_API_KEY)" \
		EDGEQUAKE_AUTH_ENABLED="$(DEV_AUTH_ENABLED)" \
		AUTH_ENABLED="$(DEV_AUTH_ENABLED)" \
		EDGEQUAKE_DEFAULT_LLM_PROVIDER="$(EDGEQUAKE_DEFAULT_LLM_PROVIDER)" \
		EDGEQUAKE_DEFAULT_LLM_MODEL="$(EDGEQUAKE_DEFAULT_LLM_MODEL)" \
		EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER="$(EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER)" \
		EDGEQUAKE_DEFAULT_EMBEDDING_MODEL="$(EDGEQUAKE_DEFAULT_EMBEDDING_MODEL)" \
		EDGEQUAKE_DEFAULT_EMBEDDING_DIMENSION="$(EDGEQUAKE_DEFAULT_EMBEDDING_DIMENSION)" \
		EDGEQUAKE_VISION_PROVIDER="$(EDGEQUAKE_VISION_PROVIDER)" \
		EDGEQUAKE_VISION_MODEL="$(EDGEQUAKE_VISION_MODEL)" \
		OLLAMA_HOST="http://localhost:11434" \
		OLLAMA_MODEL="gemma4:latest" \
		OLLAMA_EMBEDDING_MODEL="embeddinggemma:latest" \
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

backend-bg: db-wait ## Run backend in background with PostgreSQL (respects MISTRAL_API_KEY, OPENAI_API_KEY if set)
	@if curl -fsS "$(BACKEND_URL)/health" >/dev/null 2>&1; then \
		echo "$(GREEN)✓ Backend already healthy on port $(BACKEND_PORT)$(RESET)"; \
		exit 0; \
	fi
	@echo "$(BLUE)Starting backend in background...$(RESET)"
	@if [ -n "$$MISTRAL_API_KEY" ] || [ -n "$(MISTRAL_API_KEY)" ]; then \
		_MISTRAL_KEY="$${MISTRAL_API_KEY:-$(MISTRAL_API_KEY)}"; \
		echo "$(YELLOW)→ MISTRAL_API_KEY detected - using Mistral as default provider$(RESET)"; \
		printf '%s\n' "#!/bin/bash" > /tmp/edgequake-start.sh; \
		printf '%s\n' "export PORT=\"$(BACKEND_PORT)\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export DATABASE_URL=\"$(DATABASE_URL)\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export MISTRAL_API_KEY=\"$$_MISTRAL_KEY\"" >> /tmp/edgequake-start.sh; \
		[ -n "$(OPENAI_API_KEY)" ] && printf '%s\n' "export OPENAI_API_KEY=\"$(OPENAI_API_KEY)\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export EDGEQUAKE_AUTH_ENABLED=\"$(DEV_AUTH_ENABLED)\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export AUTH_ENABLED=\"$(DEV_AUTH_ENABLED)\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export EDGEQUAKE_LLM_PROVIDER=\"mistral\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export EDGEQUAKE_EMBEDDING_PROVIDER=\"mistral\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export MISTRAL_EMBEDDING_MODEL=\"mistral-embed\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export EDGEQUAKE_VISION_PROVIDER=\"mistral\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export EDGEQUAKE_VISION_MODEL=\"pixtral-large-latest\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export EDGEQUAKE_EMBEDDING_BATCH_SIZE=\"16\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "cd $(BACKEND_DIR) && exec cargo run" >> /tmp/edgequake-start.sh; \
		chmod +x /tmp/edgequake-start.sh; \
		/bin/bash -lc 'nohup /tmp/edgequake-start.sh > /tmp/edgequake-backend.log 2>&1 < /dev/null & backend_pid=$$!; disown "$$backend_pid"; printf "%s\n" "$$backend_pid" > /tmp/edgequake-backend.pid'; \
	elif [ -n "$(OPENAI_API_KEY)" ]; then \
		echo "$(YELLOW)→ OPENAI_API_KEY detected - using OpenAI as default provider$(RESET)"; \
		printf '%s\n' "#!/bin/bash" > /tmp/edgequake-start.sh; \
		printf '%s\n' "export PORT=\"$(BACKEND_PORT)\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export DATABASE_URL=\"$(DATABASE_URL)\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export OPENAI_API_KEY=\"$(OPENAI_API_KEY)\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export EDGEQUAKE_AUTH_ENABLED=\"$(DEV_AUTH_ENABLED)\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export AUTH_ENABLED=\"$(DEV_AUTH_ENABLED)\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export EDGEQUAKE_LLM_PROVIDER=\"openai\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "cd $(BACKEND_DIR) && exec cargo run" >> /tmp/edgequake-start.sh; \
		chmod +x /tmp/edgequake-start.sh; \
		/bin/bash -lc 'nohup /tmp/edgequake-start.sh > /tmp/edgequake-backend.log 2>&1 < /dev/null & backend_pid=$$!; disown "$$backend_pid"; printf "%s\n" "$$backend_pid" > /tmp/edgequake-backend.pid'; \
	else \
		echo "$(YELLOW)→ No OPENAI_API_KEY, using Ollama provider$(RESET)"; \
		printf '%s\n' "#!/bin/bash" > /tmp/edgequake-start.sh; \
		printf '%s\n' "export PORT=\"$(BACKEND_PORT)\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export DATABASE_URL=\"$(DATABASE_URL)\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export EDGEQUAKE_AUTH_ENABLED=\"$(DEV_AUTH_ENABLED)\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export AUTH_ENABLED=\"$(DEV_AUTH_ENABLED)\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export EDGEQUAKE_LLM_PROVIDER=\"ollama\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export OLLAMA_HOST=\"http://localhost:11434\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export OLLAMA_MODEL=\"gemma4:latest\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export OLLAMA_EMBEDDING_MODEL=\"embeddinggemma:latest\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "cd $(BACKEND_DIR) && exec cargo run" >> /tmp/edgequake-start.sh; \
		chmod +x /tmp/edgequake-start.sh; \
		/bin/bash -lc 'nohup /tmp/edgequake-start.sh > /tmp/edgequake-backend.log 2>&1 < /dev/null & backend_pid=$$!; disown "$$backend_pid"; printf "%s\n" "$$backend_pid" > /tmp/edgequake-backend.pid'; \
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
	@echo "$(BLUE)Starting frontend development server on port $(FRONTEND_PORT)...$(RESET)"
	@cd $(FRONTEND_DIR) && PORT="$(FRONTEND_PORT)" NEXT_PUBLIC_API_URL="$(BACKEND_URL)" NEXT_PUBLIC_AUTH_ENABLED="$(DEV_AUTH_ENABLED)" NEXT_PUBLIC_DISABLE_DEMO_LOGIN="$(DEV_DISABLE_DEMO_LOGIN)" sh -c '(pnpm run dev 2>/dev/null || bun run dev)'

frontend-bg: ## Start frontend development server in background
	@if curl -fsS "$(FRONTEND_URL)" 2>/dev/null | grep -qi 'EdgeQuake'; then \
		echo "$(GREEN)✓ Frontend already reachable on port $(FRONTEND_PORT)$(RESET)"; \
		exit 0; \
	fi
	@echo "$(BLUE)Starting frontend in background...$(RESET)"
	@printf '%s\n' "#!/bin/bash" > /tmp/edgequake-frontend-start.sh
	@printf '%s\n' "cd $(FRONTEND_DIR)" >> /tmp/edgequake-frontend-start.sh
	@printf '%s\n' "export PORT=\"$(FRONTEND_PORT)\"" >> /tmp/edgequake-frontend-start.sh
	@printf '%s\n' "export NEXT_PUBLIC_API_URL=\"$(BACKEND_URL)\"" >> /tmp/edgequake-frontend-start.sh
	@printf '%s\n' "export NEXT_PUBLIC_AUTH_ENABLED=\"$(DEV_AUTH_ENABLED)\"" >> /tmp/edgequake-frontend-start.sh
	@printf '%s\n' "export NEXT_PUBLIC_DISABLE_DEMO_LOGIN=\"$(DEV_DISABLE_DEMO_LOGIN)\"" >> /tmp/edgequake-frontend-start.sh
	@printf '%s\n' "if command -v pnpm >/dev/null 2>&1; then" >> /tmp/edgequake-frontend-start.sh
	@printf '%s\n' "  exec pnpm run dev" >> /tmp/edgequake-frontend-start.sh
	@printf '%s\n' "fi" >> /tmp/edgequake-frontend-start.sh
	@printf '%s\n' "exec bun run dev" >> /tmp/edgequake-frontend-start.sh
	@chmod +x /tmp/edgequake-frontend-start.sh
	@/bin/bash -lc 'nohup /tmp/edgequake-frontend-start.sh > /tmp/edgequake-frontend.log 2>&1 < /dev/null & frontend_pid=$$!; disown "$$frontend_pid"; printf "%s\n" "$$frontend_pid" > /tmp/edgequake-frontend.pid'
	@echo "$(GREEN)✓ Frontend starting in background. Log: /tmp/edgequake-frontend.log$(RESET)"

frontend-build: ## Build frontend for production
	@echo "$(BLUE)Building frontend...$(RESET)"
	@cd $(FRONTEND_DIR) && (pnpm run build 2>/dev/null || bun run build)
	@echo "$(GREEN)✓ Frontend built$(RESET)"

frontend-start: ## Start frontend production server
	@echo "$(BLUE)Starting frontend production server...$(RESET)"
	@cd $(FRONTEND_DIR) && (pnpm run start 2>/dev/null || bun run start)

frontend-lint: ## Lint frontend code
	@echo "$(BLUE)Linting frontend code...$(RESET)"
	@cd $(FRONTEND_DIR) && (pnpm run lint 2>/dev/null || bun run lint)

frontend-test: ## Run frontend tests
	@echo "$(BLUE)Running frontend tests...$(RESET)"
	@cd $(FRONTEND_DIR) && (pnpm test 2>/dev/null || bun test) || echo "$(YELLOW)No tests configured$(RESET)"

# ============================================================================
# Database
# ============================================================================

db-wait: db-start ## Wait for database to be ready (used by other targets)
	@echo "$(YELLOW)Waiting for database to be ready...$(RESET)"
	@DB_READY_CMD='pg_isready -h localhost -p 5432'; \
	if ! printf '%s' "$(DATABASE_URL)" | grep -Eiq '@(localhost|127\.0\.0\.1)(:|/)|://(localhost|127\.0\.0\.1)(:|/)'; then \
		DB_READY_CMD='pg_isready -d "$(DATABASE_URL)"'; \
	fi; \
	for i in 1 2 3 4 5 6 7 8 9 10; do \
		eval "$$DB_READY_CMD" >/dev/null 2>&1 && break || sleep 2; \
	done; \
	eval "$$DB_READY_CMD" >/dev/null 2>&1 && \
		echo "$(GREEN)✓ Database is ready$(RESET)" || \
		(echo "$(RED)✗ Database failed to start$(RESET)" && exit 1)

docker-network-diagnose: ## Diagnose common OrbStack/Docker network route conflicts
	@ROUTES=$$(netstat -rn 2>/dev/null | egrep '(^10[[:space:]]|^172\.16/12|^192\.168\.0/16)' || true); \
	if [ -n "$$ROUTES" ]; then \
		echo "$(YELLOW)→ Detected broad private-network routes on this host:$(RESET)"; \
		echo "$$ROUTES"; \
		echo "$(YELLOW)  WHY this matters: OrbStack/Docker bridge networks also use private ranges.$(RESET)"; \
		echo "$(YELLOW)  If those ranges are already claimed by VPN/Homebridge/router software, Docker may fail with 'failed to add network' or 'conflict with existing route'.$(RESET)"; \
	else \
		echo "$(GREEN)✓ No broad private-network route collision detected from the local route table$(RESET)"; \
	fi


db-start: ## Start PostgreSQL container
	@echo "$(BLUE)Starting PostgreSQL...$(RESET)"
	@# WHY: Prefer a single lightweight probe before touching Docker. When
	@# OrbStack/Docker is down, repeated docker exec/compose retries can amplify
	@# the failure and make the local environment feel unstable.
	@LOCAL_DB_PATTERN='@(localhost|127\.0\.0\.1)(:|/)|://(localhost|127\.0\.0\.1)(:|/)'; \
	if ! printf '%s' "$(DATABASE_URL)" | grep -Eiq "$$LOCAL_DB_PATTERN"; then \
		echo "$(GREEN)✓ Using external PostgreSQL from DATABASE_URL; skipping Docker startup$(RESET)"; \
		exit 0; \
	fi; \
	if pg_isready -h localhost -p 5432 >/dev/null 2>&1; then \
		echo "$(GREEN)✓ PostgreSQL already reachable on port 5432$(RESET)"; \
		exit 0; \
	fi; \
	if command -v docker >/dev/null 2>&1 && docker ps --format '{{.Names}}' 2>/dev/null | grep -qx 'edgequake-postgres'; then \
		for i in 1 2 3 4 5; do \
			if pg_isready -h localhost -p 5432 >/dev/null 2>&1; then \
				echo "$(GREEN)✓ Existing edgequake-postgres container is already running and reachable$(RESET)"; \
				exit 0; \
			fi; \
			sleep 2; \
		done; \
		echo "$(YELLOW)→ Existing edgequake-postgres container is running but not reachable on localhost:5432; recreating it$(RESET)"; \
		docker rm -f edgequake-postgres >/dev/null 2>&1 || true; \
	fi; \
	if command -v docker >/dev/null 2>&1 && docker ps -a --format '{{.Names}}' 2>/dev/null | grep -qx 'edgequake-postgres'; then \
		echo "$(YELLOW)→ Starting existing edgequake-postgres container...$(RESET)"; \
		docker start edgequake-postgres >/dev/null 2>&1 || true; \
		for i in 1 2 3 4 5; do \
			if pg_isready -h localhost -p 5432 >/dev/null 2>&1; then \
				echo "$(GREEN)✓ Existing edgequake-postgres container is ready$(RESET)"; \
				exit 0; \
			fi; \
			sleep 2; \
		done; \
		echo "$(YELLOW)→ Existing edgequake-postgres container is not reachable on localhost:5432; recreating it with the current compose settings$(RESET)"; \
		docker rm -f edgequake-postgres >/dev/null 2>&1 || true; \
	fi; \
	if ! command -v docker >/dev/null 2>&1; then \
		echo "$(RED)✗ Docker is not installed; cannot start the PostgreSQL container$(RESET)"; \
		exit 1; \
	fi; \
	if ! docker info >/dev/null 2>&1; then \
		echo "$(YELLOW)⚠️  Docker daemon is unavailable; EdgeQuake will not retry aggressively to avoid destabilizing OrbStack$(RESET)"; \
		$(MAKE) docker-network-diagnose --no-print-directory || true; \
		echo "$(RED)✗ PostgreSQL is not reachable and Docker cannot currently start it$(RESET)"; \
		echo "$(YELLOW)  Common root cause on OrbStack: a VPN / Homebridge / host route already claims the private subnet range that Docker wants for its bridge network.$(RESET)"; \
		echo "$(YELLOW)  Recovery: stop the conflicting network tool, restart OrbStack, then rerun 'make dev' or 'make dev-bg'.$(RESET)"; \
		exit 1; \
	fi; \
	TMP_LOG=$$(mktemp); \
	if cd $(DOCKER_DIR) && docker compose up -d postgres >"$$TMP_LOG" 2>&1; then \
		cat "$$TMP_LOG"; \
		rm -f "$$TMP_LOG"; \
	else \
		cat "$$TMP_LOG"; \
		echo "$(RED)✗ Failed to start PostgreSQL container$(RESET)"; \
		if grep -Eiq 'failed to add network|conflict with existing route|invalid IP Prefix' "$$TMP_LOG"; then \
			echo "$(YELLOW)→ Detected a Docker/OrbStack bridge-network conflict rather than an EdgeQuake application error$(RESET)"; \
			$(MAKE) docker-network-diagnose --no-print-directory || true; \
		fi; \
		rm -f "$$TMP_LOG"; \
		exit 1; \
	fi; \
	echo "$(GREEN)✓ PostgreSQL container started on port 5432$(RESET)"; \
	echo "$(YELLOW)Waiting for database to be ready...$(RESET)"; \
	for i in 1 2 3 4 5 6 7 8 9 10; do \
		pg_isready -h localhost -p 5432 >/dev/null 2>&1 && break || { echo "Waiting..."; sleep 2; }; \
	done; \
	pg_isready -h localhost -p 5432 >/dev/null 2>&1 && echo "$(GREEN)✓ Database is ready$(RESET)" || { echo "$(RED)✗ Database failed to start$(RESET)"; exit 1; }

db-stop: ## Stop PostgreSQL container
	@echo "$(BLUE)Stopping PostgreSQL...$(RESET)"
	@if command -v docker >/dev/null 2>&1 && docker info >/dev/null 2>&1; then \
		cd $(DOCKER_DIR) && docker compose stop postgres 2>/dev/null || true; \
		cd $(DOCKER_DIR) && docker compose -f docker-compose.prebuilt.yml stop postgres 2>/dev/null || true; \
		docker compose -f $(QUICKSTART_COMPOSE) stop postgres 2>/dev/null || true; \
		docker stop edgequake-postgres 2>/dev/null || true; \
	else \
		echo "$(YELLOW)→ Docker daemon unavailable; nothing to stop$(RESET)"; \
	fi
	@echo "$(GREEN)✓ PostgreSQL stop check complete$(RESET)"

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

docker-prebuilt: ## Start full stack (API + Web UI + DB) from latest published GHCR images — no build needed
	@echo ""
	@echo "$(BOLD)$(BLUE)🐳 Starting EdgeQuake Full Stack (latest published GHCR images)$(RESET)"
	@echo ""
	@if [ ! -f "$(DOCKER_DIR)/.env" ]; then \
		echo "$(YELLOW)→ Creating $(DOCKER_DIR)/.env from .env.example$(RESET)"; \
		cp $(DOCKER_DIR)/.env.example $(DOCKER_DIR)/.env; \
		echo "$(YELLOW)  Edit $(DOCKER_DIR)/.env to set your LLM provider + API key$(RESET)"; \
	fi
	@echo "$(YELLOW)→ Pulling latest images from GHCR...$(RESET)"
	@cd $(DOCKER_DIR) && docker compose -f docker-compose.prebuilt.yml pull --ignore-pull-failures edgequake frontend 2>/dev/null || true
	@echo "$(YELLOW)→ Starting services (API + Web UI + PostgreSQL)...$(RESET)"
	@cd $(DOCKER_DIR) && docker compose -f docker-compose.prebuilt.yml up -d
	@echo "$(YELLOW)→ Waiting for API to be healthy...$(RESET)"
	@for i in $$(seq 1 30); do \
		if curl -sf http://localhost:8080/health > /dev/null 2>&1; then \
			echo "$(GREEN)✓ API is healthy$(RESET)"; break; \
		fi; \
		sleep 2; \
	done
	@echo ""
	@echo "$(BOLD)$(GREEN)✅ EdgeQuake Full Stack is Running$(RESET)"
	@echo ""
	@echo "$(BOLD)📍 Access Points:$(RESET)"
	@echo ""
	@echo "  $(BLUE)Frontend (Web UI)$(RESET)"
	@echo "    🌐 URL: $(BOLD)http://localhost:3000$(RESET)"
	@echo ""
	@echo "  $(BLUE)Backend API$(RESET)"
	@echo "    🔗 URL: $(BOLD)http://localhost:8080$(RESET)"
	@echo "    📚 Swagger: $(BOLD)http://localhost:8080/swagger-ui$(RESET)"
	@echo "    🏥 Health:  $(BOLD)http://localhost:8080/health$(RESET)"
	@echo ""
	@echo "$(YELLOW)→ Management:$(RESET)"
	@echo "  $(BOLD)Logs:$(RESET)   make docker-prebuilt-logs"
	@echo "  $(BOLD)Status:$(RESET) make docker-ps-prebuilt"
	@echo "  $(BOLD)Stop:$(RESET)   make docker-prebuilt-down"
	@echo ""

docker-prebuilt-down: ## Stop prebuilt stack
	@echo "$(BLUE)Stopping prebuilt Docker stack...$(RESET)"
	@cd $(DOCKER_DIR) && docker compose -f docker-compose.prebuilt.yml down
	@echo "$(GREEN)✓ Prebuilt stack stopped$(RESET)"

docker-prebuilt-logs: ## View logs from prebuilt stack
	@cd $(DOCKER_DIR) && docker compose -f docker-compose.prebuilt.yml logs -f

docker-ps-prebuilt: ## Show container status for prebuilt stack
	@cd $(DOCKER_DIR) && docker compose -f docker-compose.prebuilt.yml ps

docker-api-only: ## Start API only using prebuilt GHCR image (bring your own PostgreSQL)
	@echo "$(YELLOW)Reminder: set DATABASE_URL in $(DOCKER_DIR)/.env first$(RESET)"
	@cd $(DOCKER_DIR) && docker compose -f docker-compose.api-only.yml up -d
	@echo "$(GREEN)✓ EdgeQuake API started (http://localhost:8080/health)$(RESET)"

# ============================================================================
# Stack — One-Command Quickstart (pulls all images from GHCR, no local build)
# ============================================================================
#
# These targets use docker-compose.quickstart.yml at the repo root.
# All three images (API, frontend, PostgreSQL) are pulled from GHCR so
# the entire stack starts from scratch in under 30 seconds after caching.
#
# Usage:
#   make stack                # pull images and start all services
#   make stack-down           # stop and remove containers
#   make stack-logs           # tail all logs
#   make stack-status         # show container status
#   make stack-restart        # stop then start
#
# Override LLM provider at runtime:
#   EDGEQUAKE_LLM_PROVIDER=openai OPENAI_API_KEY=sk-... make stack
# Pin to a specific version:
#   EDGEQUAKE_VERSION=0.9.4 make stack

QUICKSTART_COMPOSE := $(ROOT_DIR)/docker-compose.quickstart.yml

.PHONY: stack stack-down stack-logs stack-status stack-restart stack-pull

stack: ## ⚡ One command: pull all GHCR images and start API + Web UI + DB  (<30s)
	@echo ""
	@echo "$(BOLD)$(BLUE)⚡ EdgeQuake Quickstart — One Command Stack$(RESET)"
	@echo ""
	@echo "  No Rust toolchain, no Node.js, no local build needed."
	@echo "  Pulling prebuilt images from GitHub Container Registry..."
	@echo ""
	@if [ -n "$(OPENAI_API_KEY)" ]; then \
		echo "  $(GREEN)OPENAI_API_KEY detected → using OpenAI provider$(RESET)"; \
	else \
		echo "  $(YELLOW)No API key → using Ollama (ensure Ollama runs on port 11434)$(RESET)"; \
	fi
	@echo ""
	@echo "$(YELLOW)→ Pulling images...$(RESET)"
	@EDGEQUAKE_LLM_PROVIDER=$${EDGEQUAKE_LLM_PROVIDER:-$$([ -n "$(OPENAI_API_KEY)" ] && echo "openai" || echo "ollama")} \
	OPENAI_API_KEY="$(OPENAI_API_KEY)" \
	EDGEQUAKE_VERSION=$${EDGEQUAKE_VERSION:-latest} \
	docker compose -f $(QUICKSTART_COMPOSE) pull
	@echo ""
	@echo "$(YELLOW)→ Starting services...$(RESET)"
	@EDGEQUAKE_LLM_PROVIDER=$${EDGEQUAKE_LLM_PROVIDER:-$$([ -n "$(OPENAI_API_KEY)" ] && echo "openai" || echo "ollama")} \
	OPENAI_API_KEY="$(OPENAI_API_KEY)" \
	EDGEQUAKE_VERSION=$${EDGEQUAKE_VERSION:-latest} \
	docker compose -f $(QUICKSTART_COMPOSE) up -d
	@echo ""
	@echo "$(YELLOW)→ Waiting for API to be healthy (up to 60s)...$(RESET)"
	@for i in $$(seq 1 30); do \
		if curl -sf http://localhost:8080/health > /dev/null 2>&1; then \
			echo "$(GREEN)✓ API is healthy$(RESET)"; break; \
		fi; \
		printf "."; sleep 2; \
	done
	@echo ""
	@echo "$(BOLD)$(GREEN)✅ EdgeQuake Stack is Running$(RESET)"
	@echo ""
	@echo "$(BOLD)📍 Access Points:$(RESET)"
	@echo "  🌐 Web UI:  $(BOLD)http://localhost:3000$(RESET)"
	@echo "  🔗 API:     $(BOLD)http://localhost:8080$(RESET)"
	@echo "  📚 Swagger: $(BOLD)http://localhost:8080/swagger-ui$(RESET)"
	@echo "  🏥 Health:  $(BOLD)http://localhost:8080/health$(RESET)"
	@echo ""
	@echo "$(BOLD)Next steps:$(RESET)"
	@echo "  1. Open $(BOLD)http://localhost:3000$(RESET) in your browser"
	@echo "  2. Upload a PDF or paste text to build your knowledge graph"
	@echo "  3. Ask questions — EdgeQuake will retrieve graph-aware answers"
	@echo ""
	@echo "$(YELLOW)Management:$(RESET)"
	@echo "  $(BOLD)make stack-logs$(RESET)    tail logs"
	@echo "  $(BOLD)make stack-status$(RESET)  check containers"
	@echo "  $(BOLD)make stack-down$(RESET)    stop and remove containers"
	@echo ""

stack-down: ## Stop and remove all quickstart containers
	@echo "$(YELLOW)Stopping EdgeQuake quickstart stack...$(RESET)"
	@docker compose -f $(QUICKSTART_COMPOSE) down
	@echo "$(GREEN)✓ Stack stopped$(RESET)"

stack-logs: ## Tail logs from all quickstart stack containers
	@docker compose -f $(QUICKSTART_COMPOSE) logs -f

stack-status: ## Show container status for quickstart stack
	@docker compose -f $(QUICKSTART_COMPOSE) ps

stack-restart: stack-down stack ## Restart the quickstart stack (pull fresh images)
	@echo "$(GREEN)✓ Stack restarted$(RESET)"

stack-pull: ## Pull latest GHCR images without starting
	@echo "$(YELLOW)Pulling latest EdgeQuake images from GHCR...$(RESET)"
	@docker compose -f $(QUICKSTART_COMPOSE) pull
	@echo "$(GREEN)✓ Images updated$(RESET)"



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

# ============================================================================
# SDK E2E — Rust, Python, TypeScript against a live API (Docker Compose stack)
# ============================================================================
#
# Prerequisites: API healthy at SDK_E2E_URL (default http://localhost:8080).
#   make stack              # root quickstart (GHCR images)
#   make docker-prebuilt    # edgequake/docker/docker-compose.prebuilt.yml
#   make docker-up          # build-from-source full stack
#
# Override:  make sdk-e2e SDK_E2E_URL=http://127.0.0.1:9090

SDK_E2E_URL ?= http://localhost:8080

sdk-e2e: ## Run SDK E2E suites (Rust --features e2e, Python test_e2e, TS tests/e2e)
	@echo "$(BOLD)$(BLUE)SDK E2E → $(SDK_E2E_URL)$(RESET)"
	@curl -sf "$(SDK_E2E_URL)/health" >/dev/null || { \
		echo "$(RED)✗ API not healthy at $(SDK_E2E_URL)$(RESET)"; \
		echo "  Start: $(GREEN)make stack$(RESET) or $(GREEN)make docker-prebuilt$(RESET) or $(GREEN)make docker-up$(RESET)"; \
		exit 1; \
	}
	@echo "$(YELLOW)→ Rust SDK (cargo test --features e2e)$(RESET)"
	@cd $(ROOT_DIR)/sdks/rust && EDGEQUAKE_BASE_URL="$(SDK_E2E_URL)" \
		cargo test -p edgequake-sdk --test e2e_tests --features e2e -- --nocapture
	@echo "$(YELLOW)→ Python SDK (pytest tests/test_e2e.py)$(RESET)"
	@cd $(ROOT_DIR)/sdks/python && EDGEQUAKE_E2E_URL="$(SDK_E2E_URL)" \
		python3 -m pytest tests/test_e2e.py -v
	@echo "$(YELLOW)→ TypeScript SDK (bun test tests/e2e)$(RESET)"
	@cd $(ROOT_DIR)/sdks/typescript && EDGEQUAKE_E2E_URL="$(SDK_E2E_URL)" bun test tests/e2e
	@echo "$(GREEN)✓ SDK E2E complete$(RESET)"

sdk-e2e-with-stack: stack sdk-e2e ## Start quickstart stack, then run SDK E2E (containers left running)

sdk-csharp-test-unit: ## Run C# SDK unit tests only (requires dotnet; skips live E2E tests)
	@echo "$(BLUE)C# SDK unit tests (filter out E2E trait)...$(RESET)"
	cd $(ROOT_DIR)/sdks/csharp && dotnet test --filter "E2E!=true"

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
	@open "$(BACKEND_URL)/swagger-ui" 2>/dev/null || xdg-open "$(BACKEND_URL)/swagger-ui" 2>/dev/null || echo "Open $(BACKEND_URL)/swagger-ui in your browser"

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
	@curl -s "$(BACKEND_URL)/health" | jq . 2>/dev/null || echo "  $(RED)Not running$(RESET)"
	@echo ""
	@echo "$(BOLD)Frontend:$(RESET)"
	@curl -s "$(FRONTEND_URL)" >/dev/null 2>&1 && echo "  $(GREEN)Running on $(FRONTEND_URL)$(RESET)" || echo "  $(RED)Not running$(RESET)"
	@echo ""
	@echo "$(BOLD)Database:$(RESET)"
	@if pg_isready -h localhost -p 5432 >/dev/null 2>&1; then \
		echo "  $(GREEN)Running on localhost:5432$(RESET)"; \
	else \
		echo "  $(RED)Not running$(RESET)"; \
	fi
	@echo ""
