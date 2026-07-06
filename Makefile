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


.PHONY: help install dev dev-auth dev-bg dev-auth-bg dev-memory kill-app stop clean build test lint format \
        dev-pg16 dev-pg17 dev-pg18 dev-bg-pg16 dev-bg-pg17 dev-bg-pg18 \
        backend-dev backend-db backend-memory backend-bg backend-build backend-build-online backend-sqlx-prepare backend-test backend-run \
        frontend-dev frontend-bg frontend-build frontend-test frontend-lint \
        openapi-snapshot codegen-openapi codegen-openapi-refresh codegen-openapi-live \
        db-start db-start-pg16 db-start-pg17 db-start-pg18 db-stop db-wait db-logs db-shell postgres-image-build postgres-image-build-pg17 postgres-image-build-pg18 postgres-image-build-unified check-extension-pins postgres-battle-test hnsw-dimension-battle-test spec042-battle-test-all spec044-battle-test-all dev-e2e-proof dev-e2e-proof-all docker-network-diagnose stop-docker-services \
        docker-build docker-up docker-prebuilt docker-prebuilt-down docker-prebuilt-logs docker-ps-prebuilt docker-api-only docker-down docker-logs \
        stack stack-down stack-logs stack-status stack-restart stack-pull \
        check-deps status \
        test-quality test-invariants test-timing test-count test-flaky \
        test-e2e-critical test-e2e-full test-e2e-lint test-stability-report \
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

# GNU make defaults to /bin/sh (dash on Ubuntu CI); extension-pins.sh needs bash pipefail.
SHELL := /bin/bash

# Project directories
ROOT_DIR := $(shell pwd)
BACKEND_DIR := $(ROOT_DIR)/edgequake
FRONTEND_DIR := $(ROOT_DIR)/edgequake_webui
DOCKER_DIR := $(BACKEND_DIR)/docker

# SPEC-042: PostgreSQL major profile (pg16|pg17|pg18). PG18 is recommended for new dev installs.
# Override via: make dev-pg17 | EQ_POSTGRES_PROFILE=pg16 make dev | .env EQ_POSTGRES_PROFILE=pg17
EQ_POSTGRES_PROFILE ?= pg18
export EQ_POSTGRES_PROFILE
PG_PROFILES := pg16 pg17 pg18

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

# P-G13: dev stability defaults — prevent OOM during heavy PDF ingestion.
# Override from shell or .env when you need higher throughput.
WORKER_THREADS ?= 4
MAX_TASKS_PER_TENANT ?= 2
EDGEQUAKE_PDF_CONCURRENCY ?= 2
EDGEQUAKE_PDF_VISION_JOBS ?= 2
export WORKER_THREADS MAX_TASKS_PER_TENANT EDGEQUAKE_PDF_CONCURRENCY EDGEQUAKE_PDF_VISION_JOBS

# Shared exports appended to /tmp/edgequake-start.sh by backend-bg.
define BACKEND_STABILITY_EXPORTS
printf '%s\n' "export WORKER_THREADS=\"$(WORKER_THREADS)\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export MAX_TASKS_PER_TENANT=\"$(MAX_TASKS_PER_TENANT)\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_PDF_CONCURRENCY=\"$(EDGEQUAKE_PDF_CONCURRENCY)\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_PDF_VISION_JOBS=\"$(EDGEQUAKE_PDF_VISION_JOBS)\"" >> /tmp/edgequake-start.sh;
endef
DEV_AUTH_ENABLED ?= false
DEV_DISABLE_DEMO_LOGIN ?= false
# SPEC-027 AC-4: frictionless local dev when DEV_AUTH_ENABLED=false (auth secure by default otherwise).
DEV_EDGEQUAKE_DEV_MODE := $(if $(filter false,$(DEV_AUTH_ENABLED)),true,false)

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
	@echo "  $(GREEN)make dev$(RESET)          Start full development stack (PostgreSQL PG18 — default)"
	@echo "  $(GREEN)make dev-pg16$(RESET)     Start dev stack with PostgreSQL 16 (legacy)"
	@echo "  $(GREEN)make dev-pg17$(RESET)     Start dev stack with PostgreSQL 17"
	@echo "  $(GREEN)make dev-pg18$(RESET)     Start dev stack with PostgreSQL 18 (same as make dev)"
	@echo "  $(GREEN)make dev-auth$(RESET)     Start full development stack with authentication enabled"
	@echo "  $(GREEN)make dev-bg$(RESET)       Start full stack in BACKGROUND without authentication"
	@echo "  $(GREEN)make dev-bg-pg16$(RESET)  Background dev with PostgreSQL 16"
	@echo "  $(GREEN)make dev-bg-pg17$(RESET)  Background dev with PostgreSQL 17"
	@echo "  $(GREEN)make dev-bg-pg18$(RESET)  Background dev with PostgreSQL 18"
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
	@echo "  $(GREEN)make codegen-openapi-refresh$(RESET) Refresh OpenAPI snapshot + TypeScript types (offline)"
	@echo "  $(GREEN)make codegen-openapi-live$(RESET) Fetch live OpenAPI from backend + regenerate types"
	@echo ""
	@echo "$(BOLD)$(BLUE)🗄️  Database (SPEC-042 triple-track)$(RESET)"
	@echo "  $(GREEN)make db-start$(RESET)       Start PostgreSQL (profile: $(EQ_POSTGRES_PROFILE))"
	@echo "  $(GREEN)make db-start-pg16$(RESET)  Start PostgreSQL 16 only"
	@echo "  $(GREEN)make db-start-pg17$(RESET)  Start PostgreSQL 17 only"
	@echo "  $(GREEN)make db-start-pg18$(RESET)  Start PostgreSQL 18 only"
	@echo "  $(GREEN)EQ_POSTGRES_PROFILE=pg17 make dev$(RESET)  Alternative profile override"
	@echo "  $(GREEN)make db-stop$(RESET)        Stop PostgreSQL container"
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
	@echo "  $(GREEN)make test-e2e-lint$(RESET)      Validate chromium gate for flake anti-patterns"
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

dev: kill-app check-deps check-ports ## Start full development stack without authentication
	@echo ""
	@echo "$(BOLD)$(BLUE)🚀 Starting EdgeQuake Development Stack$(RESET)"
	@echo "$(YELLOW)→ Previous app processes killed; starting fresh$(RESET)"
	@# OODA-09: Dynamically select provider based on OPENAI_API_KEY
	@if [ -n "$(OPENAI_API_KEY)" ]; then \
		echo "$(BOLD)$(YELLOW)📝 Using OpenAI provider (OPENAI_API_KEY detected)$(RESET)"; \
	else \
		echo "$(BOLD)$(YELLOW)📝 Using Ollama as default LLM provider$(RESET)"; \
	fi
	@echo ""
	@echo "$(YELLOW)→ Ensuring PostgreSQL availability (profile: $(EQ_POSTGRES_PROFILE))...$(RESET)"
	@$(MAKE) db-start --no-print-directory
	@echo ""
	@echo "  $(BLUE)PostgreSQL$(RESET): $(EQ_POSTGRES_PROFILE) (see extension-pins.sh)"
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
	$(LOAD_EFF_DB_URL); \
	echo "$(YELLOW)→ Starting backend (DATABASE_URL port: $$(printf '%s' $$_EFF_DB_URL | sed -E 's|.*:([0-9]+)/.*|\1|'))...$(RESET)"; \
	if [ -n "$(OPENAI_API_KEY)" ]; then \
		(cd $(BACKEND_DIR) && \
			PORT="$(BACKEND_PORT)" \
			DATABASE_URL="$$_EFF_DB_URL" \
			OPENAI_API_KEY="$(OPENAI_API_KEY)" \
			EDGEQUAKE_DEV_MODE="$(DEV_EDGEQUAKE_DEV_MODE)" \
			EDGEQUAKE_DEV_MODE="$(DEV_EDGEQUAKE_DEV_MODE)" \
		EDGEQUAKE_AUTH_ENABLED="$(DEV_AUTH_ENABLED)" \
			AUTH_ENABLED="$(DEV_AUTH_ENABLED)" \
			cargo run 2>&1 | sed 's/^/[backend] /') & \
		BACKEND_PID=$$!; \
	else \
		(cd $(BACKEND_DIR) && \
			PORT="$(BACKEND_PORT)" \
			DATABASE_URL="$$_EFF_DB_URL" \
			EDGEQUAKE_DEV_MODE="$(DEV_EDGEQUAKE_DEV_MODE)" \
			EDGEQUAKE_DEV_MODE="$(DEV_EDGEQUAKE_DEV_MODE)" \
		EDGEQUAKE_AUTH_ENABLED="$(DEV_AUTH_ENABLED)" \
			AUTH_ENABLED="$(DEV_AUTH_ENABLED)" \
			OLLAMA_HOST="http://localhost:11434" \
			OLLAMA_MODEL="gemma4:latest" \
			OLLAMA_EMBEDDING_MODEL="embeddinggemma:latest" \
			cargo run 2>&1 | sed 's/^/[backend] /') & \
		BACKEND_PID=$$!; \
	fi; \
	echo "$(YELLOW)→ Starting frontend on port $(FRONTEND_PORT)...$(RESET)"; \
	(sleep 2 && cd $(FRONTEND_DIR) && PORT="$(FRONTEND_PORT)" EDGEQUAKE_API_URL="$(BACKEND_URL)" NEXT_PUBLIC_API_URL="$(BACKEND_URL)" NEXT_PUBLIC_AUTH_ENABLED="$(DEV_AUTH_ENABLED)" NEXT_PUBLIC_DISABLE_DEMO_LOGIN="$(DEV_DISABLE_DEMO_LOGIN)" sh -c '(pnpm run dev 2>/dev/null || bun run dev)' 2>&1 | sed 's/^/[frontend] /') & \
	FRONTEND_PID=$$!; \
	echo "$(GREEN)✓ Startup in progress$(RESET)"; \
	echo "$(YELLOW)Press Ctrl+C to stop only this session's app processes$(RESET)"; \
	wait

# SPEC-042: PostgreSQL major profile dev shortcuts (SSOT: extension-pins.sh)
define PG_DEV_RULE
dev-$(1): ## Start dev stack with PostgreSQL $(subst pg,,$(1)) (SPEC-042)
	@$(MAKE) dev EQ_POSTGRES_PROFILE=$(1) --no-print-directory
endef

define PG_DEV_BG_RULE
dev-bg-$(1): ## Start dev stack in background with PostgreSQL $(subst pg,,$(1))
	@$(MAKE) dev-bg EQ_POSTGRES_PROFILE=$(1) --no-print-directory
endef

define PG_DB_START_RULE
db-start-$(1): ## Start PostgreSQL $(subst pg,,$(1)) container only
	@$(MAKE) db-start EQ_POSTGRES_PROFILE=$(1) --no-print-directory
endef

$(foreach p,$(PG_PROFILES),$(eval $(call PG_DEV_RULE,$(p))))
$(foreach p,$(PG_PROFILES),$(eval $(call PG_DEV_BG_RULE,$(p))))
$(foreach p,$(PG_PROFILES),$(eval $(call PG_DB_START_RULE,$(p))))

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
	@echo "$(YELLOW)→ Ensuring PostgreSQL availability (profile: $(EQ_POSTGRES_PROFILE))...$(RESET)"
	@$(MAKE) db-wait --no-print-directory
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

kill-app: ## Kill backend and frontend processes (leaves PostgreSQL running)
	@echo "$(YELLOW)→ Killing existing backend processes...$(RESET)"
	@-if [ -f /tmp/edgequake-backend.pid ]; then kill -9 $$(cat /tmp/edgequake-backend.pid) 2>/dev/null || true; rm -f /tmp/edgequake-backend.pid; fi
	@-pkill -9 -f "target/debug/edgequake" 2>/dev/null || true
	@-pkill -9 -f "target/release/edgequake" 2>/dev/null || true
	@-BPID=$$(lsof -nP -iTCP:$(BACKEND_PORT) -sTCP:LISTEN -t 2>/dev/null | head -1); \
	[ -n "$$BPID" ] && kill -9 "$$BPID" 2>/dev/null || true
	@echo "$(YELLOW)→ Killing existing frontend processes...$(RESET)"
	@-if [ -f /tmp/edgequake-frontend.pid ]; then kill -9 $$(cat /tmp/edgequake-frontend.pid) 2>/dev/null || true; rm -f /tmp/edgequake-frontend.pid; fi
	@-pkill -f "node.*edgequake_webui" 2>/dev/null || true
	@-FPID=$$(lsof -nP -iTCP:$(FRONTEND_PORT) -sTCP:LISTEN -t 2>/dev/null | head -1); \
	[ -n "$$FPID" ] && kill -9 "$$FPID" 2>/dev/null || true
	@echo "$(GREEN)✓ App processes cleared (PostgreSQL left running)$(RESET)"

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

# DRY: Single shell snippet to read the effective DATABASE_URL resolved by db-start.
#
# WHY: db-start detects when another PostgreSQL instance occupies the default port
# (e.g. infrastructure-postgres, k8s) and starts edgequake-postgres on a free port
# instead, writing the corrected URL to /tmp/edgequake-db-url.
# pg_isready alone cannot catch this case — it only checks TCP socket liveness,
# not credentials.  All backend-launching recipes MUST read from this file so they
# pass the correct port to the backend binary.
#
# Usage in any recipe:
#   @$(LOAD_EFF_DB_URL); \
#     DATABASE_URL="$$_EFF_DB_URL" cargo run ...
LOAD_EFF_DB_URL = _EFF_DB_URL=$$(cat /tmp/edgequake-db-url 2>/dev/null); [ -z "$$_EFF_DB_URL" ] && _EFF_DB_URL="$(DATABASE_URL)"

# SPEC-040 v0.4.1: pdfium is now EMBEDDED in the edgequake-pdf2md 0.4.1 binary
# via pdfium-auto at compile time. No external libpdfium.dylib, no env vars needed.

backend-dev: db-wait ## Run backend in development mode with PostgreSQL (uses .env configuration)
	@echo "$(BLUE)Starting backend with PostgreSQL storage...$(RESET)"
	@if [ -n "$(EDGEQUAKE_DEFAULT_LLM_PROVIDER)" ]; then \
		echo "$(GREEN)✓ LLM Provider: $(EDGEQUAKE_DEFAULT_LLM_PROVIDER) ($(EDGEQUAKE_DEFAULT_LLM_MODEL))$(RESET)"; \
	fi
	@$(LOAD_EFF_DB_URL); \
	cd $(BACKEND_DIR) && \
		PORT="$(BACKEND_PORT)" \
		DATABASE_URL="$$_EFF_DB_URL" \
		OPENAI_API_KEY="$(OPENAI_API_KEY)" \
		EDGEQUAKE_DEV_MODE="$(DEV_EDGEQUAKE_DEV_MODE)" \
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
	@$(LOAD_EFF_DB_URL); \
	cd $(BACKEND_DIR) && \
		PORT="$(BACKEND_PORT)" \
		DATABASE_URL="$$_EFF_DB_URL" \
		OPENAI_API_KEY="$(OPENAI_API_KEY)" \
		EDGEQUAKE_DEV_MODE="$(DEV_EDGEQUAKE_DEV_MODE)" \
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
		_llm_code=$$(curl -s -o /dev/null -w '%{http_code}' "$(BACKEND_URL)/api/v1/settings/llm-defaults" 2>/dev/null || echo 000); \
		if [ "$$_llm_code" = "200" ] || [ "$$_llm_code" = "401" ]; then \
			echo "$(GREEN)✓ Backend already healthy on port $(BACKEND_PORT)$(RESET)"; \
			exit 0; \
		fi; \
		echo "$(YELLOW)⚠ Backend on port $(BACKEND_PORT) is stale (llm-defaults HTTP $$_llm_code) — restarting...$(RESET)"; \
		if [ -f /tmp/edgequake-backend.pid ]; then kill -9 $$(cat /tmp/edgequake-backend.pid) 2>/dev/null || true; fi; \
		pkill -9 -f "target/debug/edgequake" 2>/dev/null || true; \
		pkill -9 -f "target/release/edgequake" 2>/dev/null || true; \
		rm -f /tmp/edgequake-backend.pid; \
		sleep 1; \
	fi
	@echo "$(BLUE)Starting backend in background...$(RESET)"
	@# Read the effective DATABASE_URL resolved by db-start (may differ in port
	@# when another PostgreSQL occupies the default 5432).
	@$(LOAD_EFF_DB_URL); \
	_BIN="$(BACKEND_DIR)/target/debug/edgequake"; \
	if [ -x "$$_BIN" ]; then _RUN="exec $$_BIN"; else _RUN="cd $(BACKEND_DIR) && exec cargo run"; fi; \
	if [ -n "$$MISTRAL_API_KEY" ] || [ -n "$(MISTRAL_API_KEY)" ]; then \
		_MISTRAL_KEY="$${MISTRAL_API_KEY:-$(MISTRAL_API_KEY)}"; \
		echo "$(YELLOW)→ MISTRAL_API_KEY detected - using Mistral as default provider$(RESET)"; \
		printf '%s\n' "#!/bin/bash" > /tmp/edgequake-start.sh; \
		printf '%s\n' "export PORT=\"$(BACKEND_PORT)\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export DATABASE_URL=\"$$_EFF_DB_URL\"" >> /tmp/edgequake-start.sh; \
		$(BACKEND_STABILITY_EXPORTS) \
		printf '%s\n' "export MISTRAL_API_KEY=\"$$_MISTRAL_KEY\"" >> /tmp/edgequake-start.sh; \
		[ -n "$(OPENAI_API_KEY)" ] && printf '%s\n' "export OPENAI_API_KEY=\"$(OPENAI_API_KEY)\"" >> /tmp/edgequake-start.sh; \
		[ -n "$$ANTHROPIC_API_KEY" ] && printf '%s\n' "export ANTHROPIC_API_KEY=\"$$ANTHROPIC_API_KEY\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export EDGEQUAKE_DEV_MODE=\"$(DEV_EDGEQUAKE_DEV_MODE)\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export EDGEQUAKE_AUTH_ENABLED=\"$(DEV_AUTH_ENABLED)\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export AUTH_ENABLED=\"$(DEV_AUTH_ENABLED)\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export EDGEQUAKE_LLM_PROVIDER=\"mistral\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export EDGEQUAKE_EMBEDDING_PROVIDER=\"mistral\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export MISTRAL_EMBEDDING_MODEL=\"mistral-embed\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export EDGEQUAKE_VISION_PROVIDER=\"mistral\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export EDGEQUAKE_VISION_MODEL=\"pixtral-large-latest\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export EDGEQUAKE_EMBEDDING_BATCH_SIZE=\"16\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export EDGEQUAKE_ALLOWED_PROVIDERS=\"*\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "$$_RUN" >> /tmp/edgequake-start.sh; \
		chmod +x /tmp/edgequake-start.sh; \
		/bin/bash -lc 'nohup /tmp/edgequake-start.sh > /tmp/edgequake-backend.log 2>&1 < /dev/null & backend_pid=$$!; disown "$$backend_pid"; printf "%s\n" "$$backend_pid" > /tmp/edgequake-backend.pid'; \
	elif [ -n "$(OPENAI_API_KEY)" ]; then \
		echo "$(YELLOW)→ OPENAI_API_KEY detected - using OpenAI as default provider$(RESET)"; \
		printf '%s\n' "#!/bin/bash" > /tmp/edgequake-start.sh; \
		printf '%s\n' "export PORT=\"$(BACKEND_PORT)\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export DATABASE_URL=\"$$_EFF_DB_URL\"" >> /tmp/edgequake-start.sh; \
		$(BACKEND_STABILITY_EXPORTS) \
		printf '%s\n' "export OPENAI_API_KEY=\"$(OPENAI_API_KEY)\"" >> /tmp/edgequake-start.sh; \
		[ -n "$$MISTRAL_API_KEY" ] && printf '%s\n' "export MISTRAL_API_KEY=\"$$MISTRAL_API_KEY\"" >> /tmp/edgequake-start.sh; \
		[ -n "$$ANTHROPIC_API_KEY" ] && printf '%s\n' "export ANTHROPIC_API_KEY=\"$$ANTHROPIC_API_KEY\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export EDGEQUAKE_DEV_MODE=\"$(DEV_EDGEQUAKE_DEV_MODE)\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export EDGEQUAKE_AUTH_ENABLED=\"$(DEV_AUTH_ENABLED)\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export AUTH_ENABLED=\"$(DEV_AUTH_ENABLED)\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export EDGEQUAKE_LLM_PROVIDER=\"openai\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export EDGEQUAKE_ALLOWED_PROVIDERS=\"*\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "$$_RUN" >> /tmp/edgequake-start.sh; \
		chmod +x /tmp/edgequake-start.sh; \
		/bin/bash -lc 'nohup /tmp/edgequake-start.sh > /tmp/edgequake-backend.log 2>&1 < /dev/null & backend_pid=$$!; disown "$$backend_pid"; printf "%s\n" "$$backend_pid" > /tmp/edgequake-backend.pid'; \
	else \
		echo "$(YELLOW)→ No API key detected, using Ollama provider$(RESET)"; \
		printf '%s\n' "#!/bin/bash" > /tmp/edgequake-start.sh; \
		printf '%s\n' "export PORT=\"$(BACKEND_PORT)\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export DATABASE_URL=\"$$_EFF_DB_URL\"" >> /tmp/edgequake-start.sh; \
		$(BACKEND_STABILITY_EXPORTS) \
		printf '%s\n' "export EDGEQUAKE_DEV_MODE=\"$(DEV_EDGEQUAKE_DEV_MODE)\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export EDGEQUAKE_AUTH_ENABLED=\"$(DEV_AUTH_ENABLED)\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export AUTH_ENABLED=\"$(DEV_AUTH_ENABLED)\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export EDGEQUAKE_LLM_PROVIDER=\"ollama\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export OLLAMA_HOST=\"http://localhost:11434\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export OLLAMA_MODEL=\"gemma4:latest\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export OLLAMA_EMBEDDING_MODEL=\"embeddinggemma:latest\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export EDGEQUAKE_ALLOWED_PROVIDERS=\"*\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "$$_RUN" >> /tmp/edgequake-start.sh; \
		chmod +x /tmp/edgequake-start.sh; \
		/bin/bash -lc 'nohup /tmp/edgequake-start.sh > /tmp/edgequake-backend.log 2>&1 < /dev/null & backend_pid=$$!; disown "$$backend_pid"; printf "%s\n" "$$backend_pid" > /tmp/edgequake-backend.pid'; \
	fi
	@echo "$(GREEN)✓ Backend starting in background. Log: /tmp/edgequake-backend.log$(RESET)"

backend-restart: ## Stop and restart background backend (picks up newly built binary)
	@echo "$(YELLOW)Restarting backend on port $(BACKEND_PORT)...$(RESET)"
	@-if [ -f /tmp/edgequake-backend.pid ]; then kill -9 $$(cat /tmp/edgequake-backend.pid) 2>/dev/null || true; fi
	@-pkill -9 -f "target/debug/edgequake" 2>/dev/null || true
	@-pkill -9 -f "target/release/edgequake" 2>/dev/null || true
	@-rm -f /tmp/edgequake-backend.pid /tmp/edgequake-start.sh
	@sleep 1
	@$(MAKE) backend-bg --no-print-directory BACKEND_PORT=$(BACKEND_PORT) DEV_AUTH_ENABLED="$(DEV_AUTH_ENABLED)"

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
	@cd $(FRONTEND_DIR) && PORT="$(FRONTEND_PORT)" EDGEQUAKE_API_URL="$(BACKEND_URL)" NEXT_PUBLIC_API_URL="$(BACKEND_URL)" NEXT_PUBLIC_AUTH_ENABLED="$(DEV_AUTH_ENABLED)" NEXT_PUBLIC_DISABLE_DEMO_LOGIN="$(DEV_DISABLE_DEMO_LOGIN)" sh -c '(pnpm run dev 2>/dev/null || bun run dev)'

frontend-bg: ## Start frontend development server in background
	@if curl -fsS "$(FRONTEND_URL)" 2>/dev/null | grep -qi 'EdgeQuake'; then \
		echo "$(GREEN)✓ Frontend already reachable on port $(FRONTEND_PORT)$(RESET)"; \
		exit 0; \
	fi
	@echo "$(BLUE)Starting frontend in background...$(RESET)"
	@printf '%s\n' "#!/bin/bash" > /tmp/edgequake-frontend-start.sh
	@printf '%s\n' "cd $(FRONTEND_DIR)" >> /tmp/edgequake-frontend-start.sh
	@printf '%s\n' "export PORT=\"$(FRONTEND_PORT)\"" >> /tmp/edgequake-frontend-start.sh
	@printf '%s\n' "export EDGEQUAKE_API_URL=\"$(BACKEND_URL)\"" >> /tmp/edgequake-frontend-start.sh
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
# OpenAPI / TypeScript codegen (SPEC-027 OAS-009)
# ============================================================================

openapi-snapshot: ## Regenerate committed OpenAPI snapshot from ApiDoc (offline, no backend)
	@echo "$(BLUE)Refreshing OpenAPI snapshot from edgequake-api ApiDoc...$(RESET)"
	@cd $(BACKEND_DIR) && cargo test -p edgequake-api spec027_write_openapi_snapshot \
		--test spec027_api_contract -- --ignored --nocapture
	@echo "$(GREEN)✓ Snapshot: $(FRONTEND_DIR)/openapi/openapi.snapshot.json$(RESET)"

codegen-openapi: ## Generate TypeScript types from committed OpenAPI snapshot (offline)
	@echo "$(BLUE)Generating TypeScript types from OpenAPI snapshot...$(RESET)"
	@cd $(FRONTEND_DIR) && ./scripts/codegen-openapi.sh --offline
	@echo "$(GREEN)✓ Types: $(FRONTEND_DIR)/openapi/schema.d.ts$(RESET)"

codegen-openapi-refresh: openapi-snapshot codegen-openapi ## Refresh snapshot + regenerate schema.d.ts (offline)
	@echo "$(GREEN)✓ OpenAPI codegen refresh complete$(RESET)"

codegen-openapi-live: ## Fetch live OpenAPI from running backend + regenerate schema.d.ts
	@echo "$(BLUE)Fetching OpenAPI from $(BACKEND_URL)/api-docs/openapi.json ...$(RESET)"
	@curl -fsS "$(BACKEND_URL)/health" >/dev/null 2>&1 || { \
		echo "$(RED)❌ Backend not reachable at $(BACKEND_URL). Run make backend-bg or make dev-bg first.$(RESET)"; \
		exit 1; \
	}
	@cd $(FRONTEND_DIR) && OPENAPI_URL="$(BACKEND_URL)/api-docs/openapi.json" ./scripts/codegen-openapi.sh
	@echo "$(GREEN)✓ Live OpenAPI snapshot + schema.d.ts updated$(RESET)"

# ============================================================================
# Database
# ============================================================================

db-wait: db-start ## Wait for database to be ready with credential verification (used by other targets)
	@echo "$(YELLOW)Waiting for database to be ready...$(RESET)"
	@# WHY auth probe (not just pg_isready):
	@# db-start may have started edgequake-postgres on a non-default port to avoid
	@# a port conflict.  We read the effective URL it wrote, parse credentials, and
	@# verify that a psql connection actually succeeds — not just that a socket is
	@# open.  This catches the "wrong PostgreSQL instance" failure mode early.
	@$(LOAD_EFF_DB_URL); \
	_DBWAIT_HOST=$$(printf '%s' "$$_EFF_DB_URL" | sed -E 's|^[^:]+://[^@]+@([^:/]+).*|\1|'); \
	_DBWAIT_PORT=$$(printf '%s' "$$_EFF_DB_URL" | sed -E 's|^[^:]+://[^@]+@[^:]+:([0-9]+)/.*|\1|'); \
	_DBWAIT_PORT=$${_DBWAIT_PORT:-5432}; \
	_DBWAIT_USER=$$(printf '%s' "$$_EFF_DB_URL" | sed -E 's|^[^:]+://([^:]+):.*|\1|'); \
	_DBWAIT_PASS=$$(printf '%s' "$$_EFF_DB_URL" | sed -E 's|^[^:]+://[^:]+:([^@]+)@.*|\1|'); \
	_DBWAIT_NAME=$$(printf '%s' "$$_EFF_DB_URL" | sed -E 's|^[^:]+://[^/]+/([^?]*).*|\1|'); \
	for i in 1 2 3 4 5 6 7 8 9 10; do \
		if pg_isready -h "$$_DBWAIT_HOST" -p "$$_DBWAIT_PORT" >/dev/null 2>&1 && \
		   PGPASSWORD="$$_DBWAIT_PASS" psql -h "$$_DBWAIT_HOST" -p "$$_DBWAIT_PORT" \
		       -U "$$_DBWAIT_USER" -d "$$_DBWAIT_NAME" -c '\q' >/dev/null 2>&1; then \
			echo "$(GREEN)✓ Database is ready (auth verified on $$_DBWAIT_HOST:$$_DBWAIT_PORT)$(RESET)"; \
			exit 0; \
		fi; \
		sleep 2; \
	done; \
	echo "$(RED)✗ Database failed to start or authentication failed on $$_DBWAIT_HOST:$$_DBWAIT_PORT$(RESET)"; \
	echo "$(YELLOW)  Tip: run 'make db-start' manually to see detailed diagnostics$(RESET)"; \
	exit 1

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
	@# WHY: All pg_isready probes are paired with a credential auth probe.
	@# pg_isready only checks if *any* PostgreSQL is listening on a port.
	@# When other services (infrastructure-postgres, k8s, etc.) occupy port 5432,
	@# the socket check passes but authentication fails — crashing the backend with
	@# "password authentication failed for user 'edgequake'".
	@#
	@# Fix strategy:
	@#   1. Parse credentials from DATABASE_URL.
	@#   2. After pg_isready succeeds, run a psql auth probe.
	@#   3. If auth fails → port conflict → auto-detect a free port (5433…5449).
	@#   4. Start edgequake-postgres on that free port via POSTGRES_PORT env var.
	@#   5. Write the effective DATABASE_URL (with correct port) to
	@#      /tmp/edgequake-db-url for consumption by make dev / make dev-bg / etc.
	@LOCAL_DB_PATTERN='@(localhost|127\.0\.0\.1)(:|/)|://(localhost|127\.0\.0\.1)(:|/)'; \
	if ! printf '%s' "$(DATABASE_URL)" | grep -Eiq "$$LOCAL_DB_PATTERN"; then \
		echo "$(GREEN)✓ Using external PostgreSQL from DATABASE_URL; skipping Docker startup$(RESET)"; \
		printf '%s' "$(DATABASE_URL)" > /tmp/edgequake-db-url; \
		exit 0; \
	fi; \
	_DB_USER=$$(printf '%s' "$(DATABASE_URL)" | sed -E 's|^[^:]+://([^:]+):.*|\1|'); \
	_DB_PASS=$$(printf '%s' "$(DATABASE_URL)" | sed -E 's|^[^:]+://[^:]+:([^@]+)@.*|\1|'); \
	_DB_HOST=$$(printf '%s' "$(DATABASE_URL)" | sed -E 's|^[^:]+://[^@]+@([^:/]+).*|\1|'); \
	_DB_PORT=$$(printf '%s' "$(DATABASE_URL)" | sed -E 's|^[^:]+://[^@]+@[^:]+:([0-9]+)/.*|\1|'); \
	_DB_PORT=$${_DB_PORT:-5432}; \
	_DB_NAME=$$(printf '%s' "$(DATABASE_URL)" | sed -E 's|^[^:]+://[^/]+/([^?]*).*|\1|'); \
	if pg_isready -h "$$_DB_HOST" -p "$$_DB_PORT" >/dev/null 2>&1; then \
		if PGPASSWORD="$$_DB_PASS" psql -h "$$_DB_HOST" -p "$$_DB_PORT" -U "$$_DB_USER" -d "$$_DB_NAME" -c '\q' >/dev/null 2>&1; then \
			. $(DOCKER_DIR)/extension-pins.sh; \
			_PROFILE_OK=1; \
			if command -v docker >/dev/null 2>&1 && docker ps --format '{{.Names}}' 2>/dev/null | grep -qx 'edgequake-postgres'; then \
				_RUN_MAJOR=$$(docker exec edgequake-postgres psql -U edgequake -d edgequake -tAc "SELECT (current_setting('server_version_num')::int / 10000)" 2>/dev/null | tr -d '[:space:]' || true); \
				if [ -n "$$_RUN_MAJOR" ] && [ "$$_RUN_MAJOR" != "$$EQ_POSTGRES_MAJOR" ]; then \
					echo "$(YELLOW)→ Port $$_DB_PORT has PG$$_RUN_MAJOR but $$EQ_POSTGRES_PROFILE (PG$$EQ_POSTGRES_MAJOR) was requested; recreating...$(RESET)"; \
					docker rm -f edgequake-postgres >/dev/null 2>&1 || true; \
					_PROFILE_OK=0; \
				fi; \
			fi; \
			if [ "$$_PROFILE_OK" = "1" ]; then \
				echo "$(GREEN)✓ PostgreSQL already reachable on port $$_DB_PORT (credentials verified)$(RESET)"; \
				printf '%s' "$(DATABASE_URL)" > /tmp/edgequake-db-url; \
				printf '%s' "$$EQ_POSTGRES_PROFILE" > /tmp/edgequake-postgres-profile; \
				exit 0; \
			fi; \
		else \
			echo "$(YELLOW)⚠  Port $$_DB_PORT is occupied by a PostgreSQL instance that does not accept our credentials.$(RESET)"; \
			echo "$(YELLOW)   Root cause: another service (infrastructure-postgres, k8s, etc.) is using that port.$(RESET)"; \
			if command -v docker >/dev/null 2>&1 && docker ps --format '{{.Names}}' 2>/dev/null | grep -qx 'edgequake-postgres'; then \
				. $(DOCKER_DIR)/extension-pins.sh; \
				_RUN_MAJOR=$$(docker exec edgequake-postgres psql -U edgequake -d edgequake -tAc "SELECT (current_setting('server_version_num')::int / 10000)" 2>/dev/null | tr -d '[:space:]' || true); \
				if [ -n "$$_RUN_MAJOR" ] && [ "$$_RUN_MAJOR" != "$$EQ_POSTGRES_MAJOR" ]; then \
					echo "$(YELLOW)→ edgequake-postgres is PG$$_RUN_MAJOR but $$EQ_POSTGRES_PROFILE (PG$$EQ_POSTGRES_MAJOR) was requested; recreating...$(RESET)"; \
					docker rm -f edgequake-postgres >/dev/null 2>&1 || true; \
				else \
					_EQ_PORT=$$(docker port edgequake-postgres 5432/tcp 2>/dev/null | sed -E 's/.*:([0-9]+).*/\1/' | head -1); \
					if [ -n "$$_EQ_PORT" ] && PGPASSWORD="$$_DB_PASS" psql -h localhost -p "$$_EQ_PORT" -U "$$_DB_USER" -d "$$_DB_NAME" -c '\q' >/dev/null 2>&1; then \
						echo "$(GREEN)✓ Reusing edgequake-postgres (PG$$_RUN_MAJOR) on port $$_EQ_PORT (credentials verified)$(RESET)"; \
						_EFF_URL=$$(printf '%s' "$(DATABASE_URL)" | sed -E "s|(@[^:]+):[0-9]+/|\1:$$_EQ_PORT/|"); \
						printf '%s' "$$_EFF_URL" > /tmp/edgequake-db-url; \
						printf '%s' "$$EQ_POSTGRES_PROFILE" > /tmp/edgequake-postgres-profile; \
						exit 0; \
					fi; \
				fi; \
			fi; \
			echo "$(YELLOW)   Auto-detecting a free port for edgequake-postgres...$(RESET)"; \
			_FREE_PORT=""; \
			for _TRY in 5433 5434 5435 5436 5437 5438 5439 5440 5441 5442 5443 5444 5445 5446 5447 5448 5449; do \
				if ! lsof -iTCP:"$$_TRY" -sTCP:LISTEN >/dev/null 2>&1; then \
					if ! PGPASSWORD="$$_DB_PASS" psql -h localhost -p "$$_TRY" -U "$$_DB_USER" -d "$$_DB_NAME" -c '\q' >/dev/null 2>&1; then \
						_FREE_PORT="$$_TRY"; \
						break; \
					fi; \
				fi; \
			done; \
			if [ -z "$$_FREE_PORT" ]; then \
				echo "$(RED)✗ No free PostgreSQL port found in range 5433-5449$(RESET)"; \
				exit 1; \
			fi; \
			echo "$(YELLOW)→ Will start edgequake-postgres on port $$_FREE_PORT instead$(RESET)"; \
			_DB_PORT="$$_FREE_PORT"; \
			if command -v docker >/dev/null 2>&1 && docker ps -a --format '{{.Names}}' 2>/dev/null | grep -qx 'edgequake-postgres'; then \
				echo "$(YELLOW)→ Removing stale edgequake-postgres container to rebind on port $$_FREE_PORT...$(RESET)"; \
				docker rm -f edgequake-postgres >/dev/null 2>&1 || true; \
			fi; \
		fi; \
	fi; \
	if command -v docker >/dev/null 2>&1 && docker ps -a --format '{{.Names}}' 2>/dev/null | grep -qx 'edgequake-postgres'; then \
		. $(DOCKER_DIR)/extension-pins.sh; \
		_RUN_MAJOR=$$(docker exec edgequake-postgres psql -U edgequake -d edgequake -tAc "SELECT (current_setting('server_version_num')::int / 10000)" 2>/dev/null | tr -d '[:space:]' || true); \
		if [ -n "$$_RUN_MAJOR" ] && [ "$$_RUN_MAJOR" != "$$EQ_POSTGRES_MAJOR" ]; then \
			echo "$(YELLOW)→ PostgreSQL profile mismatch (running PG$$_RUN_MAJOR, requested $$EQ_POSTGRES_PROFILE); recreating container...$(RESET)"; \
			docker rm -f edgequake-postgres >/dev/null 2>&1 || true; \
		fi; \
	fi; \
	if command -v docker >/dev/null 2>&1 && docker ps -a --format '{{.Names}} {{.Status}}' 2>/dev/null | grep -E 'edgequake-postgres.*(Restarting|Exited)'; then \
		if docker logs edgequake-postgres 2>&1 | grep -q 'Counter to that, there appears to be PostgreSQL data'; then \
			echo "$(YELLOW)→ edgequake-postgres crash-loop: PG18+ volume layout mismatch; removing container$(RESET)"; \
			docker rm -f edgequake-postgres >/dev/null 2>&1 || true; \
		fi; \
	fi; \
	if command -v docker >/dev/null 2>&1 && docker ps --format '{{.Names}}' 2>/dev/null | grep -qx 'edgequake-postgres'; then \
		. $(DOCKER_DIR)/extension-pins.sh; \
		_PG_EXT="/usr/share/postgresql/$$EQ_POSTGRES_MAJOR/extension"; \
		for i in 1 2 3 4 5; do \
			if pg_isready -h localhost -p "$$_DB_PORT" >/dev/null 2>&1; then \
				_PV_SHIP=$$(docker exec edgequake-postgres sed -n "s/default_version = '\([^']*\)'.*/\1/p" "$$_PG_EXT/vector.control" 2>/dev/null || true); \
				case "$$_PV_SHIP" in \
					0.8.*|0.9.*|[1-9]*) \
						echo "$(GREEN)✓ Existing edgequake-postgres container is already running and reachable$(RESET)"; \
						_EFF_URL=$$(printf '%s' "$(DATABASE_URL)" | sed -E "s|(@[^:]+):[0-9]+/|\1:$$_DB_PORT/|"); \
						printf '%s' "$$_EFF_URL" > /tmp/edgequake-db-url; \
						printf '%s' "$$EQ_POSTGRES_PROFILE" > /tmp/edgequake-postgres-profile; \
						exit 0 ;; \
					*) \
						echo "$(YELLOW)→ edgequake-postgres ships pgvector $$_PV_SHIP (< 0.8); rebuilding container...$(RESET)"; \
						docker rm -f edgequake-postgres >/dev/null 2>&1 || true; \
						break ;; \
				esac; \
			fi; \
			sleep 2; \
		done; \
		echo "$(YELLOW)→ Existing edgequake-postgres container is running but not reachable on localhost:$$_DB_PORT; recreating it$(RESET)"; \
		docker rm -f edgequake-postgres >/dev/null 2>&1 || true; \
	fi; \
	if command -v docker >/dev/null 2>&1 && docker ps -a --format '{{.Names}}' 2>/dev/null | grep -qx 'edgequake-postgres'; then \
		echo "$(YELLOW)→ Starting existing edgequake-postgres container...$(RESET)"; \
		docker start edgequake-postgres >/dev/null 2>&1 || true; \
		. $(DOCKER_DIR)/extension-pins.sh; \
		_PG_EXT="/usr/share/postgresql/$$EQ_POSTGRES_MAJOR/extension"; \
		for i in 1 2 3 4 5; do \
			if pg_isready -h localhost -p "$$_DB_PORT" >/dev/null 2>&1; then \
				_PV_SHIP=$$(docker exec edgequake-postgres sed -n "s/default_version = '\([^']*\)'.*/\1/p" "$$_PG_EXT/vector.control" 2>/dev/null || true); \
				case "$$_PV_SHIP" in \
					0.8.*|0.9.*|[1-9]*) \
						echo "$(GREEN)✓ Existing edgequake-postgres container is ready$(RESET)"; \
						_EFF_URL=$$(printf '%s' "$(DATABASE_URL)" | sed -E "s|(@[^:]+):[0-9]+/|\1:$$_DB_PORT/|"); \
						printf '%s' "$$_EFF_URL" > /tmp/edgequake-db-url; \
						printf '%s' "$$EQ_POSTGRES_PROFILE" > /tmp/edgequake-postgres-profile; \
						exit 0 ;; \
					*) \
						echo "$(YELLOW)→ edgequake-postgres ships pgvector $$_PV_SHIP (< 0.8); rebuilding container...$(RESET)"; \
						docker rm -f edgequake-postgres >/dev/null 2>&1 || true; \
						break ;; \
				esac; \
			fi; \
			sleep 2; \
		done; \
		echo "$(YELLOW)→ Existing edgequake-postgres container is not reachable on localhost:$$_DB_PORT; recreating it$(RESET)"; \
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
	. $(DOCKER_DIR)/extension-pins.sh; \
	if [ "$$EQ_POSTGRES_MAJOR" -ge 18 ] 2>/dev/null; then \
		_PG_DATA_DIR=/var/lib/postgresql; \
		_PG_VOL_NAME=postgres-data-pg18; \
	else \
		_PG_DATA_DIR=/var/lib/postgresql/data; \
		_PG_VOL_NAME=postgres-data-pg$$EQ_POSTGRES_MAJOR; \
	fi; \
	echo "$(BLUE)→ PostgreSQL profile: $$EQ_POSTGRES_PROFILE (PG$$EQ_POSTGRES_MAJOR, $$(basename $$EQ_POSTGRES_DOCKERFILE))$(RESET)"; \
	_start_postgres() { \
		cd $(DOCKER_DIR) && EQ_POSTGRES_PROFILE="$$EQ_POSTGRES_PROFILE" EQ_POSTGRES_DOCKERFILE="$$EQ_POSTGRES_DOCKERFILE" \
			POSTGRES_PORT="$$_DB_PORT" \
			POSTGRES_VOLUME_NAME="$$_PG_VOL_NAME" \
			POSTGRES_DATA_DIR="$$_PG_DATA_DIR" \
			docker compose up -d --build postgres >"$$TMP_LOG" 2>&1; \
	}; \
	if _start_postgres; then \
		cat "$$TMP_LOG"; \
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
	echo "$(GREEN)✓ PostgreSQL container started on port $$_DB_PORT$(RESET)"; \
	echo "$(YELLOW)Waiting for database to be ready...$(RESET)"; \
	_wait_db() { \
		for i in $$(seq 1 30); do \
			if pg_isready -h localhost -p "$$_DB_PORT" >/dev/null 2>&1 && \
			   PGPASSWORD="$$_DB_PASS" psql -h localhost -p "$$_DB_PORT" -U "$$_DB_USER" -d "$$_DB_NAME" -c '\q' >/dev/null 2>&1; then \
				return 0; \
			fi; \
			echo "Waiting..."; sleep 2; \
		done; \
		return 1; \
	}; \
	if ! _wait_db; then \
		if docker logs edgequake-postgres 2>&1 | grep -q 'Counter to that, there appears to be PostgreSQL data'; then \
			echo "$(YELLOW)→ PG$$EQ_POSTGRES_MAJOR volume layout mismatch (upgrade from older image); recreating volume $$_PG_VOL_NAME...$(RESET)"; \
			docker rm -f edgequake-postgres >/dev/null 2>&1 || true; \
			docker volume rm "edgequake-dev_$$_PG_VOL_NAME" "docker_$$_PG_VOL_NAME" "edgequake-dev_postgres-data" "docker_postgres-data" 2>/dev/null || true; \
			if _start_postgres; then cat "$$TMP_LOG"; else cat "$$TMP_LOG"; rm -f "$$TMP_LOG"; exit 1; fi; \
			if ! _wait_db; then \
				echo "$(RED)✗ Database failed to start after volume reset$(RESET)"; \
				docker logs edgequake-postgres 2>&1 | tail -30; \
				rm -f "$$TMP_LOG"; exit 1; \
			fi; \
		else \
			echo "$(RED)✗ Database failed to start$(RESET)"; \
			docker logs edgequake-postgres 2>&1 | tail -30; \
			rm -f "$$TMP_LOG"; exit 1; \
		fi; \
	fi; \
	rm -f "$$TMP_LOG"; \
	echo "$(GREEN)✓ Database is ready$(RESET)"; \
	. $(DOCKER_DIR)/extension-pins.sh; \
	_PG_EXT="/usr/share/postgresql/$$EQ_POSTGRES_MAJOR/extension"; \
	_PV_DB=$$(docker exec edgequake-postgres psql -U edgequake -d edgequake -tAc "SELECT extversion FROM pg_extension WHERE extname = 'vector'" 2>/dev/null | tr -d '[:space:]' || true); \
	_PV_SHIP=$$(docker exec edgequake-postgres sed -n "s/default_version = '\([^']*\)'.*/\1/p" "$$_PG_EXT/vector.control" 2>/dev/null | tr -d '[:space:]' || true); \
	if [ -n "$$_PV_DB" ] && [ -n "$$_PV_SHIP" ] && [ "$$_PV_DB" != "$$_PV_SHIP" ]; then \
		echo "$(YELLOW)→ Upgrading pgvector catalog $$_PV_DB → $$_PV_SHIP (ALTER EXTENSION vector UPDATE)...$(RESET)"; \
		docker exec edgequake-postgres psql -U edgequake -d edgequake -c "ALTER EXTENSION vector UPDATE;" >/dev/null 2>&1 || \
			echo "$(YELLOW)  pgvector catalog upgrade deferred to backend migration 042$(RESET)"; \
	fi; \
	_AGE_DB=$$(docker exec edgequake-postgres psql -U edgequake -d edgequake -tAc "SELECT extversion FROM pg_extension WHERE extname = 'age'" 2>/dev/null | tr -d '[:space:]' || true); \
	_AGE_SHIP=$$(docker exec edgequake-postgres sed -n "s/default_version = '\([^']*\)'.*/\1/p" "$$_PG_EXT/age.control" 2>/dev/null | tr -d '[:space:]' || true); \
	if [ -n "$$_AGE_DB" ] && [ -n "$$_AGE_SHIP" ] && [ "$$_AGE_DB" != "$$_AGE_SHIP" ]; then \
		echo "$(YELLOW)→ Upgrading Apache AGE catalog $$_AGE_DB → $$_AGE_SHIP (ALTER EXTENSION age UPDATE)...$(RESET)"; \
		docker exec edgequake-postgres psql -U edgequake -d edgequake -c "ALTER EXTENSION age UPDATE;" >/dev/null 2>&1 || \
			echo "$(YELLOW)  AGE catalog upgrade deferred to backend migration 043$(RESET)"; \
	fi; \
	_EFF_URL=$$(printf '%s' "$(DATABASE_URL)" | sed -E "s|(@[^:]+):[0-9]+/|\1:$$_DB_PORT/|"); \
	printf '%s' "$$_EFF_URL" > /tmp/edgequake-db-url; \
	printf '%s' "$$EQ_POSTGRES_PROFILE" > /tmp/edgequake-postgres-profile; \
	echo "$(GREEN)✓ Effective DATABASE_URL written to /tmp/edgequake-db-url (profile: $$EQ_POSTGRES_PROFILE)$(RESET)"

postgres-image-build: ## Build and verify edgequake-postgres Docker image (pgvector 0.8.3 + AGE 1.6.0, PG16)
	@echo "$(BLUE)Building edgequake-postgres image (PG16)...$(RESET)"
	@cd $(DOCKER_DIR) && docker build -f Dockerfile.postgres -t edgequake-postgres:pg16 .
	@chmod +x $(DOCKER_DIR)/verify-postgres-extensions.sh
	@EQ_POSTGRES_PROFILE=pg16 bash $(DOCKER_DIR)/verify-postgres-extensions.sh edgequake-postgres:pg16
	@echo "$(GREEN)✓ edgequake-postgres:pg16 ready$(RESET)"

postgres-image-build-pg17: ## Build and verify edgequake-postgres PG17 image (pgvector 0.8.3 + AGE 1.7.0)
	@echo "$(BLUE)Building edgequake-postgres image (PG17 / SPEC-042-C)...$(RESET)"
	@cd $(DOCKER_DIR) && docker build -f Dockerfile.postgres.pg17 -t edgequake-postgres:pg17 .
	@chmod +x $(DOCKER_DIR)/verify-postgres-extensions.sh
	@EQ_POSTGRES_PROFILE=pg17 bash $(DOCKER_DIR)/verify-postgres-extensions.sh edgequake-postgres:pg17
	@echo "$(GREEN)✓ edgequake-postgres:pg17 ready$(RESET)"

postgres-image-build-pg18: ## Build and verify edgequake-postgres PG18 image (pgvector 0.8.3 + AGE 1.7.0) — default dev profile
	@echo "$(BLUE)Building edgequake-postgres image (PG18 / SPEC-042-B)...$(RESET)"
	@cd $(DOCKER_DIR) && docker build -f Dockerfile.postgres.pg18 -t edgequake-postgres:pg18 -t edgequake-postgres:local .
	@chmod +x $(DOCKER_DIR)/verify-postgres-extensions.sh
	@EQ_POSTGRES_PROFILE=pg18 bash $(DOCKER_DIR)/verify-postgres-extensions.sh edgequake-postgres:local
	@echo "$(GREEN)✓ edgequake-postgres:local (PG18) ready$(RESET)"

postgres-image-build-unified: ## Build any PG profile via unified Dockerfile (DRY — SPEC-042)
	@_p="$${EQ_POSTGRES_PROFILE:-pg18}"; \
	source $(DOCKER_DIR)/extension-pins.sh; \
	echo "$(BLUE)Building edgequake-postgres ($$_p) via unified Dockerfile...$(RESET)"; \
	cd $(DOCKER_DIR) && docker build \
		--build-arg PG_MAJOR="$$EQ_POSTGRES_MAJOR" \
		--build-arg PGVECTOR_VERSION="$$EQ_PGVECTOR_VERSION" \
		--build-arg AGE_GIT_REF="$$EQ_AGE_GIT_REF" \
		--build-arg AGE_EXPECTED_VERSION="$$EQ_AGE_MIN" \
		-f Dockerfile.postgres.unified \
		-t "edgequake-postgres:$$EQ_POSTGRES_GHCR_SUFFIX" .; \
	chmod +x $(DOCKER_DIR)/verify-postgres-extensions.sh; \
	EQ_POSTGRES_PROFILE="$$_p" bash $(DOCKER_DIR)/verify-postgres-extensions.sh "edgequake-postgres:$$EQ_POSTGRES_GHCR_SUFFIX"; \
	echo "$(GREEN)✓ edgequake-postgres:$$EQ_POSTGRES_GHCR_SUFFIX ready (unified)$(RESET)"

check-extension-pins: ## Verify Dockerfile pins match extension-pins.sh SSOT (SPEC-042 DRY gate)
	@bash scripts/check_extension_pins.sh all

postgres-battle-test: ## Run SPEC-042 version feature battle test (all PG tiers)
	@chmod +x specs/042-update-age-pgvector/e2e/run_version_feature_battle_test.sh
	@./specs/042-update-age-pgvector/e2e/run_version_feature_battle_test.sh all

hnsw-dimension-battle-test: ## Run SPEC-042 #275 HNSW dimension guard battle test (all PG tiers)
	@chmod +x specs/042-update-age-pgvector/e2e/run_hnsw_dimension_battle_test.sh
	@./specs/042-update-age-pgvector/e2e/run_hnsw_dimension_battle_test.sh all

spec042-battle-test-all: ## Run full SPEC-042 battle suite (pins + version + Phase E + #275)
	@chmod +x specs/042-update-age-pgvector/e2e/run_all_battle_tests.sh
	@./specs/042-update-age-pgvector/e2e/run_all_battle_tests.sh

spec044-battle-test-all: ## SPEC-044 triple-track Cypher bind battle test (pg16 + pg17 + pg18)
	@chmod +x specs/044-upgrate-issue-study/e2e/run_triple_track_cypher_proof.sh
	@./specs/044-upgrate-issue-study/e2e/run_triple_track_cypher_proof.sh all

phase-e-battle-test: ## Run SPEC-042-E Phase E acceptance probes (pg17 + pg18)
	@chmod +x specs/042-update-age-pgvector/e2e/run_phase_e_battle_test.sh
	@./specs/042-update-age-pgvector/e2e/run_phase_e_battle_test.sh all

dev-e2e-proof: ## SPEC-042 dev E2E proof + screenshots (requires running stack; uses active PG profile)
	@chmod +x specs/042-update-age-pgvector/e2e/run_dev_e2e_proof.sh
	@./specs/042-update-age-pgvector/e2e/run_dev_e2e_proof.sh

dev-e2e-proof-all: ## SPEC-042 dev E2E proof on pg16 + pg17 + pg18 (switch DB per profile)
	@chmod +x specs/042-update-age-pgvector/e2e/run_dev_e2e_proof_all_profiles.sh
	@SKIP_IMAGE_BUILD=1 ./specs/042-update-age-pgvector/e2e/run_dev_e2e_proof_all_profiles.sh

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

test-spec021: ## SPEC-021 ingest resilience + P-G2 persister contracts (Rust + TS unit)
	@echo "$(BLUE)Running SPEC-021 ingest resilience contracts...$(RESET)"
	@cd edgequake && cargo test -p edgequake-api --test e2e_spec021_ingest_resilience -- --nocapture
	@cd edgequake && cargo test -p edgequake-api --test e2e_spec021_ingestion_persister -- --nocapture
	@cd edgequake && cargo test -p edgequake-api --test e2e_spec021_query_modes_http -- --nocapture
	@cd edgequake && cargo test -p edgequake-pipeline --test contract_ingestion_persistence -- --nocapture
	@cd edgequake && cargo test -p edgequake-query --test contract_query_modes -- --nocapture
	@cd edgequake && cargo test -p edgequake-query --test contract_query_result_cache -- --nocapture
	@cd edgequake && cargo test -p edgequake-pipeline --test contract_merger_graph_batch -- --nocapture
	@cd edgequake && cargo test -p edgequake-api --test e2e_spec021_query_cache_invalidation -- --nocapture
	@cd edgequake && cargo test -p edgequake-api --test e2e_spec021_worker_cache_invalidation -- --nocapture
	@cd edgequake && cargo test -p edgequake-api --test spec021_test_provider_override_contract -- --nocapture
	@cd edgequake && cargo test -p edgequake-api --test spec021_processor_cache_invalidator_contract -- --nocapture
	@cd edgequake && cargo test -p edgequake-core --test spec021_orchestrator_cache_invalidation -- --nocapture
	@cd edgequake && cargo test -p edgequake-api --lib ingest_admission -- --nocapture
	@cd edgequake && cargo test -p edgequake-api --lib pdf_admission_registry -- --nocapture
	@cd edgequake && cargo test -p edgequake-api --lib health_probes -- --nocapture
	@cd $(FRONTEND_DIR) && bun test src/lib/api/__tests__/backend-readiness.test.ts
	@echo "$(GREEN)✓ SPEC-021 contract tests passed$(RESET)"

test-flaky: ## Run flaky test detection (3 iterations)
	@echo "$(BLUE)Running flaky test detection...$(RESET)"
	@./scripts/detect_flaky_tests.sh 3 all

test-e2e-critical: ## Run E2E critical path tests
	@echo "$(BLUE)Running E2E critical path tests...$(RESET)"
	@cd $(FRONTEND_DIR) && PLAYWRIGHT_BASE_URL=http://localhost:3000 \
		pnpm exec playwright test ooda-228-critical-path.spec.ts --reporter=line

test-e2e-lint: ## Fail if chromium-gate e2e specs contain flake anti-patterns
	@python3 $(FRONTEND_DIR)/scripts/validate-e2e-flake.py

test-e2e-ui: test-e2e-lint ## UI-only chromium gate (no backend; skips integration specs)
	@echo "$(BLUE)Running UI-only E2E chromium gate (PLAYWRIGHT_SKIP_STACK_CHECK=1)$(RESET)"
	@FPID=$$(lsof -nP -iTCP:3001 -sTCP:LISTEN -t 2>/dev/null | head -1); \
	if [ -n "$$FPID" ] && ! curl -fsS --max-time 3 http://127.0.0.1:3001 2>/dev/null | grep -qi EdgeQuake; then \
		echo "$(YELLOW)→ Killing unhealthy frontend listener on port 3001$(RESET)"; \
		kill "$$FPID" 2>/dev/null || true; \
		sleep 1; \
	fi
	@cd $(FRONTEND_DIR) && PLAYWRIGHT_SKIP_STACK_CHECK=1 \
		pnpm exec playwright test --project=chromium --reporter=line

test-e2e-full: dev-bg test-e2e-lint ## Run full E2E suite (requires make dev-bg stack)
	@echo "$(BLUE)Running full E2E suite → frontend $(FRONTEND_URL) backend $(BACKEND_URL)$(RESET)"
	@curl -sf "$(BACKEND_URL)/health" >/dev/null || { \
		echo "$(RED)✗ EdgeQuake backend not healthy at $(BACKEND_URL)$(RESET)"; exit 1; \
	}
	@cd $(FRONTEND_DIR) && EQ_BACKEND_URL="$(BACKEND_URL)" E2E_BACKEND_URL="$(BACKEND_URL)" \
		SPEC013_BACKEND_URL="$(BACKEND_URL)" E2E_LIVE_STACK=1 PLAYWRIGHT_BASE_URL="$(FRONTEND_URL)" \
		pnpm exec playwright test --project=chromium --reporter=line

spec043-e2e: dev-bg ## SPEC-043 model picker + attribution E2E with screenshots
	@echo "$(BLUE)SPEC-043 E2E → frontend $(FRONTEND_URL) backend $(BACKEND_URL)$(RESET)"
	@for i in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15; do \
		curl -sf "$(BACKEND_URL)/health" >/dev/null && curl -sf "$(FRONTEND_URL)/" >/dev/null && break; \
		sleep 2; \
	done
	@curl -sf "$(BACKEND_URL)/health" >/dev/null || { \
		echo "$(RED)✗ EdgeQuake backend not healthy at $(BACKEND_URL)$(RESET)"; exit 1; \
	}
	@curl -sf "$(FRONTEND_URL)/" >/dev/null || { \
		echo "$(RED)✗ Frontend not reachable at $(FRONTEND_URL)$(RESET)"; exit 1; \
	}
	@cd $(FRONTEND_DIR) && EQ_BACKEND_URL="$(BACKEND_URL)" E2E_BACKEND_URL="$(BACKEND_URL)" \
		E2E_LIVE_STACK=1 PLAYWRIGHT_SKIP_STACK_CHECK=1 PLAYWRIGHT_BASE_URL="$(FRONTEND_URL)" \
		pnpm exec playwright test e2e/spec043-llm-model-picker.spec.ts --project=chromium --reporter=line

# ============================================================================
# SPEC-013 — GitHub issues #216–#233 (May 2026)
# ============================================================================
SPEC013_BACKEND_PORT ?= 8081
SPEC013_BACKEND_URL ?= http://localhost:$(SPEC013_BACKEND_PORT)

spec013-e2e-rust: db-wait ## In-process API tests for SPEC-013 fixes (PostgreSQL, mock LLM)
	@echo "$(BLUE)SPEC-013 Rust E2E (PostgreSQL in-process)...$(RESET)"
	@_DB=$$(cat /tmp/edgequake-db-url 2>/dev/null); \
	[ -n "$$_DB" ] || _DB="$(DATABASE_URL)"; \
	[ -n "$$_DB" ] || { echo "$(RED)✗ DATABASE_URL required (make db-wait)$(RESET)"; exit 1; }; \
	cd $(BACKEND_DIR) && DATABASE_URL="$$_DB" cargo test -p edgequake-api --features postgres \
		--test e2e_spec013_github_issues -- --nocapture
	@cd $(BACKEND_DIR) && cargo test -p edgequake-pipeline --lib entity_type -- --nocapture

spec013-mistral-backend-bg: db-wait ## Backend on :8081 with Mistral (avoids Docker :8080)
	@if [ -z "$(MISTRAL_API_KEY)" ] && [ -z "$$MISTRAL_API_KEY" ]; then \
		echo "$(RED)✗ MISTRAL_API_KEY required for spec013-mistral-backend-bg$(RESET)"; exit 1; \
	fi
	@$(MAKE) backend-bg BACKEND_PORT=$(SPEC013_BACKEND_PORT) DEV_AUTH_ENABLED=false --no-print-directory

spec013-e2e-playwright-intensive: ## Playwright intensive SPEC-013 suite (Mistral stack)
	@echo "$(BLUE)SPEC-013 Playwright intensive → backend $(SPEC013_BACKEND_URL)$(RESET)"
	@curl -sf "$(SPEC013_BACKEND_URL)/health" >/dev/null 2>&1 || { \
		echo "$(RED)✗ Backend not healthy at $(SPEC013_BACKEND_URL)$(RESET)"; \
		echo "  Run: $(GREEN)make spec013-mistral-backend-bg$(RESET) and $(GREEN)make frontend-bg$(RESET)"; \
		exit 1; \
	}
	@curl -sf "$(SPEC013_BACKEND_URL)/health" | python3 -c 'import json,sys; d=json.load(sys.stdin); p=d.get("llm_provider_name") or d.get("providers",{}).get("llm",{}).get("name"); sys.exit(0 if p=="mistral" else 1)' || { \
		echo "$(RED)✗ Backend is not using live Mistral (llm_provider_name != mistral)$(RESET)"; \
		echo "  Current health: $$(curl -sf "$(SPEC013_BACKEND_URL)/health" 2>/dev/null || echo unavailable)"; \
		exit 1; \
	}
	@cd $(FRONTEND_DIR) && SPEC013_BACKEND_URL="$(SPEC013_BACKEND_URL)" \
		E2E_BACKEND_URL="$(SPEC013_BACKEND_URL)" \
		PLAYWRIGHT_BASE_URL=http://localhost:$(FRONTEND_PORT) \
		pnpm exec playwright test -c playwright.spec013.config.ts --reporter=line

test-e2e-mistral-live: ## Run chromium e2e against live Mistral backend (requires MISTRAL_API_KEY)
	@if [ -z "$(MISTRAL_API_KEY)" ] && [ -z "$$MISTRAL_API_KEY" ]; then \
		echo "$(RED)✗ MISTRAL_API_KEY required for test-e2e-mistral-live$(RESET)"; \
		exit 1; \
	fi
	@BPID=$$(lsof -nP -iTCP:$(BACKEND_PORT) -sTCP:LISTEN -t 2>/dev/null | head -1); \
	if [ -n "$$BPID" ]; then \
		echo "$(YELLOW)→ Restarting backend on port $(BACKEND_PORT) for deterministic auth/provider config$(RESET)"; \
		kill "$$BPID" 2>/dev/null || true; \
		sleep 1; \
	fi
	@FPID=$$(lsof -nP -iTCP:$(FRONTEND_PORT) -sTCP:LISTEN -t 2>/dev/null | head -1); \
	if [ -n "$$FPID" ]; then \
		echo "$(YELLOW)→ Freeing frontend port $(FRONTEND_PORT) for Playwright-managed webServer$(RESET)"; \
		kill "$$FPID" 2>/dev/null || true; \
		sleep 1; \
	fi
	@$(MAKE) backend-bg DEV_AUTH_ENABLED=false WORKER_THREADS=1 MAX_TASKS_PER_TENANT=1 --no-print-directory
	@for i in $$(seq 1 30); do \
		if curl -sf "$(BACKEND_URL)/health" >/dev/null 2>&1; then break; fi; \
		sleep 2; \
	done; \
	curl -sf "$(BACKEND_URL)/health" >/dev/null || { \
		echo "$(RED)✗ Backend not healthy at $(BACKEND_URL)$(RESET)"; \
		echo "  Last backend logs:"; tail -20 /tmp/edgequake-backend.log 2>/dev/null || true; \
		exit 1; \
	}
	@curl -sf "$(BACKEND_URL)/health" | python3 -c 'import json,sys; d=json.load(sys.stdin); p=d.get("llm_provider_name") or d.get("providers",{}).get("llm",{}).get("name"); sys.exit(0 if p=="mistral" else 1)' || { \
		echo "$(RED)✗ Backend is not running live Mistral$(RESET)"; \
		echo "  Current health: $$(curl -sf "$(BACKEND_URL)/health" 2>/dev/null || echo unavailable)"; \
		exit 1; \
	}
	@echo "$(GREEN)✓ Live Mistral backend verified at $(BACKEND_URL)$(RESET)"
	@cd $(FRONTEND_DIR) && EQ_BACKEND_URL="$(BACKEND_URL)" E2E_BACKEND_URL="$(BACKEND_URL)" \
		SPEC013_BACKEND_URL="$(BACKEND_URL)" E2E_LIVE_STACK=1 NEXT_PUBLIC_API_URL="$(BACKEND_URL)" \
		EDGEQUAKE_API_URL="$(BACKEND_URL)" NEXT_PUBLIC_AUTH_ENABLED=false \
		NEXT_PUBLIC_DISABLE_DEMO_LOGIN=false PLAYWRIGHT_SKIP_STACK_CHECK=1 \
		pnpm exec playwright test --project=chromium --project=load --reporter=line

spec013-e2e-mistral-live: db-wait ## Live Mistral document ingest (MISTRAL_API_KEY + PostgreSQL)
	@echo "$(BLUE)SPEC-013 live Mistral ingest test (PostgreSQL)...$(RESET)"
	@_DB=$$(cat /tmp/edgequake-db-url 2>/dev/null); \
	[ -n "$$_DB" ] || _DB="$(DATABASE_URL)"; \
	[ -n "$$_DB" ] || { echo "$(RED)✗ DATABASE_URL required$(RESET)"; exit 1; }; \
	cd $(BACKEND_DIR) && DATABASE_URL="$$_DB" cargo test -p edgequake-api --features postgres \
		--test e2e_spec013_mistral_live -- --ignored --nocapture

spec013-e2e-mistral: spec013-e2e-rust ## Rust + Playwright + Mistral workspace/live (start spec013-mistral-backend-bg first)
	@if [ -n "$(MISTRAL_API_KEY)" ] || [ -n "$$MISTRAL_API_KEY" ]; then \
		$(MAKE) spec013-e2e-mistral-rust-live --no-print-directory; \
	else \
		echo "$(YELLOW)→ Skipping Mistral live Rust tests (MISTRAL_API_KEY not set)$(RESET)"; \
	fi
	@$(MAKE) spec013-e2e-playwright-intensive --no-print-directory
	@echo "$(GREEN)✓ SPEC-013 intensive E2E complete$(RESET)"

spec013-e2e-mistral-rust-live: db-wait ## Mistral workspace + ingest tests (PostgreSQL, requires MISTRAL_API_KEY)
	@_DB=$$(cat /tmp/edgequake-db-url 2>/dev/null); \
	[ -n "$$_DB" ] || _DB="$(DATABASE_URL)"; \
	[ -n "$$_DB" ] || { echo "$(RED)✗ DATABASE_URL required$(RESET)"; exit 1; }; \
	cd $(BACKEND_DIR) && DATABASE_URL="$$_DB" cargo test -p edgequake-api --features postgres \
		--test e2e_spec013_mistral_live -- --nocapture; \
	cd $(BACKEND_DIR) && DATABASE_URL="$$_DB" cargo test -p edgequake-api --features postgres \
		--test e2e_spec013_mistral_live -- --ignored --nocapture

SPEC013_CARGO_TEST_ARGS ?= --test-threads=1

spec013-proof-preflight: db-wait ## Fail fast if SPEC-013 proof prerequisites are missing or unsafe
	@_DB=$$(cat /tmp/edgequake-db-url 2>/dev/null); \
	[ -n "$$_DB" ] || _DB="$(DATABASE_URL)"; \
	[ -n "$$_DB" ] || { echo "$(RED)✗ DATABASE_URL required$(RESET)"; exit 1; }; \
	[ -n "$(MISTRAL_API_KEY)" ] || [ -n "$$MISTRAL_API_KEY" ] || { \
		echo "$(RED)✗ MISTRAL_API_KEY required$(RESET)"; exit 1; \
	}; \
	if curl -sf "$(BACKEND_URL)/health" >/dev/null 2>&1 && [ "$(SPEC013_INCLUDE_LIVE_API_TESTS)" != "1" ]; then \
		echo "$(RED)✗ Dev backend is up at $(BACKEND_URL) — stop it before in-process spec013-proof$(RESET)"; \
		echo "  $(GREEN)make stop$(RESET) (or set SPEC013_INCLUDE_LIVE_API_TESTS=1 only with backend stopped for cargo tests)"; \
		exit 1; \
	fi; \
	echo "$(GREEN)✓ SPEC-013 preflight OK$(RESET)"

spec013-proof: spec013-proof-preflight ## Deterministic SPEC-013 proof (PostgreSQL + Mistral PDF ingest/query invariants)
	@echo "$(BOLD)$(BLUE)SPEC-013 deterministic proof$(RESET)"
	@_DB=$$(cat /tmp/edgequake-db-url 2>/dev/null); \
	[ -n "$$_DB" ] || _DB="$(DATABASE_URL)"; \
	_LIVE="$(SPEC013_LIVE_API_URL)"; \
	if [ "$(SPEC013_INCLUDE_LIVE_API_TESTS)" = "1" ] && [ -z "$$_LIVE" ]; then \
		_LIVE="$(BACKEND_URL)"; \
	fi; \
	if [ -n "$$_LIVE" ]; then \
		echo "$(YELLOW)→ Live API tests enabled ($$_LIVE) — stop dev backend to avoid duplicate workers on DATABASE_URL$(RESET)"; \
	else \
		echo "$(YELLOW)→ In-process only (no SPEC013_LIVE_API_URL — avoids DB worker contention)$(RESET)"; \
	fi; \
	cd $(BACKEND_DIR) && DATABASE_URL="$$_DB" SPEC013_LIVE_API_URL="$$_LIVE" cargo test -p edgequake-api --features postgres \
		--test e2e_spec013_github_issues -- $(SPEC013_CARGO_TEST_ARGS) --nocapture; \
	cd $(BACKEND_DIR) && DATABASE_URL="$$_DB" EDGEQUAKE_REQUIRE_MISTRAL_TESTS=1 cargo test -p edgequake-api --features postgres \
		--test e2e_spec013_mistral_pdf_query -- $(SPEC013_CARGO_TEST_ARGS) --nocapture; \
	cd $(BACKEND_DIR) && DATABASE_URL="$$_DB" EDGEQUAKE_REQUIRE_POSTGRES_TESTS=1 cargo test -p edgequake-storage --features postgres \
		--test postgres_workspace_vector_stats -- $(SPEC013_CARGO_TEST_ARGS) --nocapture
	@echo "$(GREEN)✓ SPEC-013 proof passed$(RESET)"

SPEC013_PROOF_REPEAT ?= 5
spec013-proof-repeat: db-wait ## Run spec013-proof N times to detect flakiness (N=SPEC013_PROOF_REPEAT)
	@echo "$(BOLD)$(BLUE)SPEC-013 proof repeat ($(SPEC013_PROOF_REPEAT)x)$(RESET)"
	@i=1; \
	while [ $$i -le $(SPEC013_PROOF_REPEAT) ]; do \
		echo "$(YELLOW)→ Iteration $$i/$(SPEC013_PROOF_REPEAT)$(RESET)"; \
		$(MAKE) spec013-proof --no-print-directory || exit 1; \
		i=$$((i+1)); \
	done
	@echo "$(GREEN)✓ SPEC-013 proof repeat complete$(RESET)"

spec013-proof-ci: db-wait ## CI-strict proof gate (3x repeat, fails on missing Mistral env)
	@SPEC013_INGEST_SLO_SECS=$${SPEC013_INGEST_SLO_SECS:-900}; \
	SPEC013_QUERY_SLO_SECS=$${SPEC013_QUERY_SLO_SECS:-120}; \
	export SPEC013_INGEST_SLO_SECS SPEC013_QUERY_SLO_SECS; \
	echo "$(YELLOW)SLO gates: ingest=$$SPEC013_INGEST_SLO_SECS s query=$$SPEC013_QUERY_SLO_SECS s$(RESET)"; \
	$(MAKE) spec013-proof-repeat SPEC013_PROOF_REPEAT=3 --no-print-directory

SPEC013_BACKEND_URL ?= $(BACKEND_URL)
SPEC013_FRONTEND_URL ?= http://localhost:$(FRONTEND_PORT)

# Resolve backend URL after backend-bg (PORT in start script may differ from make-time BACKEND_PORT).
define spec013_effective_backend_url
$(shell if [ -f /tmp/edgequake-start.sh ]; then \
	_P=$$(grep '^export PORT=' /tmp/edgequake-start.sh 2>/dev/null | sed -E 's/^export PORT="?([^"]+)"?/\1/'); \
	[ -n "$$_P" ] && echo "http://localhost:$$_P" || echo "$(SPEC013_BACKEND_URL)"; \
else echo "$(SPEC013_BACKEND_URL)"; fi)
endef

spec013-proof-ui: ## Playwright SPEC-013 UI proof (#216–#233); requires backend + frontend up
	@echo "$(BOLD)$(BLUE)SPEC-013 UI proof (Playwright)$(RESET)"
	@$(MAKE) spec013-wait-stack --no-print-directory
	@_BE="$(call spec013_effective_backend_url)"; \
	echo "$(YELLOW)→ Backend: $$_BE$(RESET)"
	@curl -sfI "$(SPEC013_FRONTEND_URL)" >/dev/null || { \
		echo "$(RED)✗ Frontend not reachable at $(SPEC013_FRONTEND_URL)$(RESET)"; exit 1; \
	}; \
	if ! curl -sf "$(SPEC013_FRONTEND_URL)" | grep -qi edgequake; then \
		echo "$(RED)✗ Port $(SPEC013_FRONTEND_URL) is not EdgeQuake WebUI (wrong app?)$(RESET)"; \
		echo "  Hint: $(GREEN)make dev-bg$(RESET) or set FRONTEND_PORT to the EdgeQuake port (often 3001)"; exit 1; \
	fi
	@_BE="$(call spec013_effective_backend_url)"; \
	cd $(FRONTEND_DIR) && E2E_BACKEND_URL="$$_BE" \
		SPEC013_BACKEND_URL="$$_BE" \
		PLAYWRIGHT_BASE_URL="$(SPEC013_FRONTEND_URL)" \
		pnpm exec playwright test --config playwright.spec013-ui.config.ts
	@echo "$(GREEN)✓ SPEC-013 UI proof passed$(RESET)"

spec013-proof-preflight-pr: db-wait ## PR gate preflight (PostgreSQL only; no Mistral key)
	@_DB=$$(cat /tmp/edgequake-db-url 2>/dev/null); \
	[ -n "$$_DB" ] || _DB="$(DATABASE_URL)"; \
	[ -n "$$_DB" ] || { echo "$(RED)✗ DATABASE_URL required$(RESET)"; exit 1; }; \
	if curl -sf "$(BACKEND_URL)/health" >/dev/null 2>&1 && [ "$(SPEC013_INCLUDE_LIVE_API_TESTS)" != "1" ]; then \
		echo "$(RED)✗ Dev backend is up at $(BACKEND_URL) — stop it before in-process spec013-proof-pr$(RESET)"; \
		echo "  $(GREEN)make stop$(RESET)"; exit 1; \
	fi; \
	echo "$(GREEN)✓ SPEC-013 PR preflight OK$(RESET)"

spec013-proof-pr: spec013-proof-preflight-pr ## Fast PR gate: mock API + vector stats (no Mistral, no live API)
	@echo "$(BOLD)$(BLUE)SPEC-013 PR proof (mock + storage)$(RESET)"
	@_DB=$$(cat /tmp/edgequake-db-url 2>/dev/null); \
	[ -n "$$_DB" ] || _DB="$(DATABASE_URL)"; \
	cd $(BACKEND_DIR) && DATABASE_URL="$$_DB" cargo test -p edgequake-api --features postgres \
		--test e2e_spec013_github_issues -- $(SPEC013_CARGO_TEST_ARGS) --nocapture; \
	cd $(BACKEND_DIR) && DATABASE_URL="$$_DB" EDGEQUAKE_REQUIRE_POSTGRES_TESTS=1 cargo test -p edgequake-storage --features postgres \
		--test postgres_workspace_vector_stats -- $(SPEC013_CARGO_TEST_ARGS) --nocapture
	@echo "$(GREEN)✓ SPEC-013 PR proof passed$(RESET)"

spec013-wait-stack: ## Wait until backend + EdgeQuake frontend are healthy (SPEC013_*_URL)
	@_BE="$(call spec013_effective_backend_url)"; \
	echo "$(YELLOW)→ Waiting for stack at $$_BE / $(SPEC013_FRONTEND_URL)$(RESET)"; \
	ok=0; \
	for i in $$(seq 1 60); do \
		if curl -sf "$$_BE/health" >/dev/null 2>&1 \
			&& curl -sfI "$(SPEC013_FRONTEND_URL)" >/dev/null 2>&1 \
			&& curl -sf "$(SPEC013_FRONTEND_URL)" 2>/dev/null | grep -qi edgequake; then \
			ok=1; break; \
		fi; \
		sleep 2; \
	done; \
	[ "$$ok" = "1" ] || { \
		echo "$(RED)✗ Stack not ready after 90s$(RESET)"; \
		echo "  Backend log: /tmp/edgequake-backend.log"; \
		echo "  Frontend log: /tmp/edgequake-frontend.log"; exit 1; \
	}; \
	echo "$(GREEN)✓ Stack ready$(RESET)"

spec013-proof-full: ## Stop dev stack → Rust proof → start stack → Playwright (#216–#233)
	@echo "$(BOLD)$(BLUE)SPEC-013 full proof (Rust + UI)$(RESET)"
	@$(MAKE) stop --no-print-directory 2>/dev/null || true
	@$(MAKE) spec013-proof --no-print-directory
	@$(MAKE) backend-bg frontend-bg --no-print-directory
	@$(MAKE) spec013-wait-stack --no-print-directory
	@$(MAKE) spec013-proof-ui --no-print-directory
	@echo "$(GREEN)✓ SPEC-013 full proof passed$(RESET)"

spec013-entity-type-audit: ## Audit graph entity types vs workspace allow-list (needs TENANT_ID + WORKSPACE_ID)
	@[ -n "$(TENANT_ID)" ] && [ -n "$(WORKSPACE_ID)" ] || { \
		echo "$(RED)✗ TENANT_ID and WORKSPACE_ID required$(RESET)"; \
		echo "  Example: make spec013-entity-type-audit TENANT_ID=... WORKSPACE_ID=..."; exit 1; \
	}
	@python3 $(ROOT_DIR)/scripts/spec013_entity_type_audit.py \
		--api "$(SPEC013_BACKEND_URL)" \
		--tenant-id "$(TENANT_ID)" \
		--workspace-id "$(WORKSPACE_ID)"

spec013-entity-type-audit-all: ## Audit all tenants/workspaces (API must be up; optional JSON_OUT=path)
	@_BE="$(call spec013_effective_backend_url)"; \
	curl -sf "$$_BE/health" >/dev/null || { \
		echo "$(RED)✗ Backend not healthy at $$_BE$(RESET)"; exit 1; \
	}; \
	python3 $(ROOT_DIR)/scripts/spec013_entity_type_audit.py \
		--api "$$_BE" --scan-all \
		$(if $(JSON_OUT),--json-out $(JSON_OUT),)

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
	@cd $(DOCKER_DIR) && docker compose -f docker-compose.test.yml up -d --build postgres-test
	@echo "$(YELLOW)Waiting for databases to be ready...$(RESET)"
	@for i in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 20 25 30; do \
		(docker exec edgequake-postgres-test pg_isready -U edgequake_test -d edgequake_test 2>/dev/null) && break || sleep 2; \
	done
	@echo "$(YELLOW)Verifying pgvector + AGE extensions...$(RESET)"
	@docker exec edgequake-postgres-test psql -U edgequake_test -d edgequake_test -c "CREATE EXTENSION IF NOT EXISTS vector; CREATE EXTENSION IF NOT EXISTS age;" >/dev/null 2>&1 \
		|| (echo "$(RED)✗ Failed to enable vector/age extensions$(RESET)" && exit 1)
	@echo "$(GREEN)✓ PostgreSQL test containers ready (pgvector + AGE)$(RESET)"

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

.PHONY: spec020-qc-proof observability-proof observability-jaeger resource-proof resource-proof-postgres release-gates

resource-proof: ## Run SPEC-006 resource safety proof suite (mock; no Postgres required)
	@chmod +x specifications/006-ensure-perf/e2e/run_resource_proof.sh scripts/spec006_no_get_all_api.sh scripts/spec006_budget_catalog_sync.sh scripts/spec006_source_ids_migration.sh scripts/spec006_no_unguarded_community_api.sh scripts/spec006_no_adhoc_resource_budget.sh scripts/spec006_apply_migration_038.sh edgequake/scripts/migrations/apply_038.sh
	@DATABASE_URL= POSTGRES_PASSWORD= ./specifications/006-ensure-perf/e2e/run_resource_proof.sh

resource-proof-postgres: test-postgres-start ## SPEC-006 battle test with live Postgres (migration bootstrap e2e)
	@echo "$(BLUE)Running SPEC-006 Postgres battle tests...$(RESET)"
	@cd $(BACKEND_DIR) && \
		POSTGRES_HOST=localhost \
		POSTGRES_PORT=5433 \
		POSTGRES_DB=edgequake_test \
		POSTGRES_USER=edgequake_test \
		POSTGRES_PASSWORD=test_password_123 \
		DATABASE_URL="postgresql://edgequake_test:test_password_123@localhost:5433/edgequake_test" \
		cargo test -p edgequake-api --test migration_bootstrap_proof --test migration_readiness_proof --features postgres --quiet
	@POSTGRES_HOST=localhost POSTGRES_PORT=5433 POSTGRES_DB=edgequake_test POSTGRES_USER=edgequake_test POSTGRES_PASSWORD=test_password_123 \
		DATABASE_URL="postgresql://edgequake_test:test_password_123@localhost:5433/edgequake_test" \
		./specifications/006-ensure-perf/e2e/run_resource_proof.sh
	@echo "$(GREEN)✓ SPEC-006 resource-proof-postgres complete$(RESET)"

spec020-qc-proof: ## SPEC-020 full Playwright quality-control E2E (screenshots + proof)
	@chmod +x specs/020-e2e-quality-control/e2e/run_quality_control_proof.sh
	@./specs/020-e2e-quality-control/e2e/run_quality_control_proof.sh

spec020-qc-proof-strict: ## SPEC-020 prod gate (migration-038 + /ready strict)
	@chmod +x specs/020-e2e-quality-control/e2e/run_quality_control_proof.sh
	@SPEC020_STRICT_MIGRATION=1 ./specs/020-e2e-quality-control/e2e/run_quality_control_proof.sh

spec020-qc-proof-full: ## SPEC-020 full prod gate (strict migration + require Ollama)
	@chmod +x specs/020-e2e-quality-control/e2e/run_quality_control_proof.sh
	@SPEC020_STRICT_MIGRATION=1 SPEC020_REQUIRE_OLLAMA=1 ./specs/020-e2e-quality-control/e2e/run_quality_control_proof.sh

spec020-qc-proof-auth: ## SPEC-020 auth-enabled login proof (DEV_AUTH_ENABLED=true)
	@chmod +x specs/020-e2e-quality-control/e2e/run_quality_control_proof.sh
	@SPEC020_AUTH_PROOF=1 ./specs/020-e2e-quality-control/e2e/run_quality_control_proof.sh

release-gates: ## Pre-release gate: fmt, per-crate clippy, tests, SPEC-006 + SPEC-018 proofs
	@chmod +x scripts/release_gates.sh
	@./scripts/release_gates.sh

observability-proof: ## Run SPEC-018 observability proof suite (Rust + WebUI)
	@./specs/018-observability/e2e/run_observability_proof.sh

observability-jaeger: ## Docker stack with Jaeger OTLP + JSON logs (SPEC-018)
	@cd $(DOCKER_DIR) && docker compose -f docker-compose.yml -f docker-compose.observability.yml --profile observability up --build

.PHONY: spec028-mcp-test mcp-registry-validate mcp-registry-publish

spec028-mcp-test: ## Run SPEC-028 MCP E2E + registry contract tests
	@cd edgequake && cargo test -p edgequake-api --features postgres \
		--test spec028_mcp_e2e --test spec028_mcp_transport --test spec028_mcp_oauth_e2e \
		--test spec028_mcp_registry --test spec028_api_contract

mcp-registry-validate: spec028-mcp-test ## Validate MCP Registry server.json SSOT (code is law)
	@command -v mcp-publisher >/dev/null || { \
		curl -fsSL "https://github.com/modelcontextprotocol/registry/releases/latest/download/mcp-publisher_$$(uname -s | tr '[:upper:]' '[:lower:]')_$$(uname -m | sed 's/x86_64/amd64/;s/aarch64/arm64/').tar.gz" \
			| tar xz mcp-publisher; \
		mv mcp-publisher /tmp/mcp-publisher; \
	}; \
	cd specs/028-edgequake-query-service/mcp && /tmp/mcp-publisher validate || mcp-publisher validate

mcp-registry-publish: mcp-registry-validate ## Publish EdgeQuake to official MCP Registry (requires: mcp-publisher login github)
	@command -v mcp-publisher >/dev/null || { echo "$(RED)✗ Install mcp-publisher — see specs/028-edgequake-query-service/mcp/007-sota-implementation-roadmap.md$(RESET)"; exit 1; }
	@cd specs/028-edgequake-query-service/mcp && mcp-publisher publish

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
	@_STATUS_DB_URL=$$(cat /tmp/edgequake-db-url 2>/dev/null); \
	_STATUS_DB_PORT=$$(printf '%s' "$$_STATUS_DB_URL" | sed -E 's|^[^:]+://[^@]+@[^:]+:([0-9]+)/.*|\1|'); \
	_STATUS_DB_PORT=$${_STATUS_DB_PORT:-5432}; \
	_STATUS_DB_PASS=$$(printf '%s' "$$_STATUS_DB_URL" | sed -E 's|^[^:]+://[^:]+:([^@]+)@.*|\1|'); \
	_STATUS_DB_USER=$$(printf '%s' "$$_STATUS_DB_URL" | sed -E 's|^[^:]+://([^:]+):.*|\1|'); \
	_STATUS_DB_NAME=$$(printf '%s' "$$_STATUS_DB_URL" | sed -E 's|^[^:]+://[^/]+/([^?]*).*|\1|'); \
	if pg_isready -h localhost -p "$$_STATUS_DB_PORT" >/dev/null 2>&1 && \
	   PGPASSWORD="$$_STATUS_DB_PASS" psql -h localhost -p "$$_STATUS_DB_PORT" -U "$$_STATUS_DB_USER" -d "$$_STATUS_DB_NAME" -c '\q' >/dev/null 2>&1; then \
		_STATUS_PG_MAJOR=$$(PGPASSWORD="$$_STATUS_DB_PASS" psql -h localhost -p "$$_STATUS_DB_PORT" -U "$$_STATUS_DB_USER" -d "$$_STATUS_DB_NAME" -tAc "SELECT (current_setting('server_version_num')::int / 10000)" 2>/dev/null | tr -d '[:space:]' || true); \
		_STATUS_PROFILE=$$(cat /tmp/edgequake-postgres-profile 2>/dev/null || echo "$(EQ_POSTGRES_PROFILE)"); \
		echo "  $(GREEN)Running on localhost:$$_STATUS_DB_PORT — PostgreSQL $$_STATUS_PG_MAJOR ($$_STATUS_PROFILE)$(RESET)"; \
	elif pg_isready -h localhost -p 5432 >/dev/null 2>&1; then \
		echo "  $(YELLOW)Port 5432 reachable but not edgequake credentials — check /tmp/edgequake-db-url$(RESET)"; \
	else \
		echo "  $(RED)Not running$(RESET)"; \
	fi
	@echo ""
