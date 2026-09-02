.PHONY: sdk-rust-build sdk-rust-publish sdk-rust-version
.PHONY: sdk-python-build sdk-python-publish sdk-python-version
.PHONY: sdk-typescript-build sdk-typescript-publish sdk-typescript-version
.PHONY: sdk-java-build sdk-java-publish sdk-java-version
.PHONY: sdk-kotlin-build sdk-kotlin-publish sdk-kotlin-version

# Portable in-place sed (GNU vs BSD/macOS). Temp-file rewrite avoids `sed -i ''`.
define SED_INPLACE
tmp=$$(mktemp "$${TMPDIR:-/tmp}/edgequake-sed.XXXXXX"); \
sed -E $(1) $(2) > "$$tmp" && mv "$$tmp" $(2)
endef

sdk-rust-version: ## Update the version of the Rust SDK (sdks/rust). Usage: make sdk-rust-version VERSION=0.2.0
	@if [ -z "$(VERSION)" ]; then \
		echo "Usage: make sdk-rust-version VERSION=<new_version>"; \
		exit 1; \
	fi
	@$(call SED_INPLACE,'s/^version = ".*"/version = "$(VERSION)"/',sdks/rust/Cargo.toml)
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
	@$(call SED_INPLACE,'s/^version = ".*"/version = "$(VERSION)"/',sdks/python/pyproject.toml)
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
	@$(call SED_INPLACE,'s/"version": ".*"/"version": "$(VERSION)"/',sdks/typescript/package.json)
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
	@$(call SED_INPLACE,'s/<version>.*<\/version>/<version>$(VERSION)<\/version>/',sdks/java/pom.xml)
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
	@$(call SED_INPLACE,'s/<version>.*<\/version>/<version>$(VERSION)<\/version>/',sdks/kotlin/pom.xml)
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


.PHONY: help install dev dev-auth dev-bg dev-auth-bg dev-langfuse dev-bg-langfuse dev-memory kill-app stop clean build test lint format sync-dev-ports \
        ops17-smoke spec046-acc data-access-perf-matrix data-access-perf-matrix-release data-access-perf-matrix-prod data-access-perf-capacity-ladder ann-scale-battle ceiling-proof recall-pareto dedicated-midscale diskann-battle diskann-recall-pareto diskann-rescore-smoke filtered-recall-gate precision-layers-gate binary-quantize-bakeoff filtered-diskann-labels-bakeoff midscale-quantize-labels tiny-slice-exact-gate serving-view-check push-scale-ladder wave2-greenfield-env product-limits-check compare-eq-perf \
        postgres-image-build-pg18-vectorscale \
        dev-pg16 dev-pg17 dev-pg18 dev-bg-pg16 dev-bg-pg17 dev-bg-pg18 \
        backend-dev backend-db backend-memory backend-bg backend-build backend-build-online backend-sqlx-prepare backend-test backend-run \
        frontend-dev frontend-bg frontend-build frontend-test frontend-lint \
        openapi-snapshot codegen-openapi codegen-openapi-refresh codegen-openapi-live \
        codegen-vision-prompts \
        db-start postgres-start db-start-pg16 db-start-pg17 db-start-pg18 db-stop db-wait db-logs db-shell postgres-image-build postgres-image-build-pg17 postgres-image-build-pg18 postgres-image-build-pg18-vectorscale postgres-image-build-unified check-extension-pins postgres-battle-test hnsw-dimension-battle-test spec042-battle-test-all spec044-battle-test-all dev-e2e-proof dev-e2e-proof-all docker-network-diagnose stop-docker-services \
        docker-build docker-up docker-prebuilt docker-prebuilt-down docker-prebuilt-logs docker-ps-prebuilt docker-api-only docker-down docker-logs \
        langfuse-up langfuse-down langfuse-logs langfuse-status langfuse-smoke langfuse-reset spec124-langfuse-e2e \
        langfuse-3.1-up langfuse-3.1-down langfuse-3.1-reset spec124-langfuse-3.1-e2e \
        langfuse-3.22-up langfuse-3.22-down langfuse-3.22-reset spec124-langfuse-3.22-e2e \
        langfuse-3.225-up langfuse-3.225-down langfuse-3.225-reset spec124-langfuse-3.225-e2e \
        spec124-langfuse-cloud-e2e spec124-langfuse-matrix \
        langfuse-sync-prices \
        k8s-prereqs k8s-kind-up k8s-kind-down k8s-install k8s-uninstall k8s-status spec138-kubernetes-proof spec138-helm-template \
        stack stack-down stack-logs stack-status stack-restart stack-pull \
        spec091-upgrade-soak spec091-gates spec103-llm-cache-proof \
        spec109-reasoning-effort-proof \
        spec110-migrate-118-proof spec137-migrate-025-026-proof \
        spec139-migrate-engine-proof \
        spec93-migration-assessment spec93-migration-assessment-pg16 \
        spec93-migration-assessment-pg17 spec93-migration-assessment-pg18 \
        check-deps status \
        test-quality test-invariants test-timing test-count test-flaky \
	test-e2e-critical test-e2e-full test-e2e-lint test-stability-report \
        measure-bulk-ingest \
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

# Colors for terminal output (printf so ESC bytes work with bash echo)
BLUE := $(shell printf '\033[34m')
GREEN := $(shell printf '\033[32m')
YELLOW := $(shell printf '\033[33m')
RED := $(shell printf '\033[31m')
BOLD := $(shell printf '\033[1m')
RESET := $(shell printf '\033[0m')

# GNU make defaults to /bin/sh (dash on Ubuntu CI); extension-pins.sh needs bash pipefail.
SHELL := /bin/bash

# Project directories
ROOT_DIR := $(shell pwd)
BACKEND_DIR := $(ROOT_DIR)/edgequake
FRONTEND_DIR := $(ROOT_DIR)/edgequake_webui
DOCKER_DIR := $(BACKEND_DIR)/docker
LANGFUSE_COMPOSE := $(DOCKER_DIR)/docker-compose.langfuse.yml
LANGFUSE_COMPOSE_PROJECT := edgequake-langfuse
LANGFUSE_311_COMPOSE := $(DOCKER_DIR)/docker-compose.langfuse-3.1.yml
LANGFUSE_311_COMPOSE_PROJECT := edgequake-langfuse-3-1
LANGFUSE_311_PORT ?= 3320
LANGFUSE_311_UI_URL ?= http://localhost:$(LANGFUSE_311_PORT)
LANGFUSE_311_PK ?= pk-lf-edgequake-311
LANGFUSE_311_SK ?= sk-lf-edgequake-311-dev
LANGFUSE_311_PROJECT_ID ?= edgequake-local-311
LANGFUSE_322_COMPOSE := $(DOCKER_DIR)/docker-compose.langfuse-3.22.yml
LANGFUSE_322_COMPOSE_PROJECT := edgequake-langfuse-3-22
LANGFUSE_322_PORT ?= 3330
LANGFUSE_322_UI_URL ?= http://localhost:$(LANGFUSE_322_PORT)
LANGFUSE_322_PK ?= pk-lf-edgequake-322
LANGFUSE_322_SK ?= sk-lf-edgequake-322-dev
LANGFUSE_322_PROJECT_ID ?= edgequake-local-322
LANGFUSE_3225_COMPOSE := $(DOCKER_DIR)/docker-compose.langfuse-3.225.yml
LANGFUSE_3225_COMPOSE_PROJECT := edgequake-langfuse-3-225
LANGFUSE_3225_PORT ?= 3340
LANGFUSE_3225_UI_URL ?= http://localhost:$(LANGFUSE_3225_PORT)
LANGFUSE_3225_PK ?= pk-lf-edgequake-3225
LANGFUSE_3225_SK ?= sk-lf-edgequake-3225-dev
LANGFUSE_3225_PROJECT_ID ?= edgequake-local-3225
# 3100 is gps-mcp on this machine; 3310 is the EdgeQuake Langfuse UI default.
LANGFUSE_PORT ?= 3310
LANGFUSE_UI_URL := http://localhost:$(LANGFUSE_PORT)
LANGFUSE_LOCAL_PK := pk-lf-edgequake-local
LANGFUSE_LOCAL_SK := sk-lf-edgequake-local-dev
LANGFUSE_LOCAL_PROJECT_ID := edgequake-local
# 1 = start isolated Langfuse v4 and force Compose init keys into the backend
# (overrides .env Cloud/placeholder keys). Used by make dev-langfuse / spec124-langfuse-e2e.
WITH_LANGFUSE ?=

# SPEC-042: PostgreSQL major profile (pg16|pg17|pg18). PG18 is recommended for new dev installs.
# Override via: make dev-pg17 | EQ_POSTGRES_PROFILE=pg16 make dev | .env EQ_POSTGRES_PROFILE=pg17
EQ_POSTGRES_PROFILE ?= pg18
export EQ_POSTGRES_PROFILE
PG_PROFILES := pg16 pg17 pg18

# Local development ports (SSOT: scripts/sync_dev_ports.py + .edgequake-dev-ports.env).
# WHY: 8080/3000 collide with gps-backend, Docker quickstart, and other local stacks.
# EdgeQuake scans upward from 8090/3010 only when those are taken.
DEFAULT_BACKEND_PORT ?= 8090
DEFAULT_FRONTEND_PORT ?= 3010
PORT_SCAN_WINDOW ?= 20
DEV_PORTS_ENV := $(ROOT_DIR)/.edgequake-dev-ports.env

# User secrets / LLM keys — loaded first.
-include $(ROOT_DIR)/.env

# Generated ports override .env PORT/BACKEND_PORT (written by `make sync-dev-ports`).
-include $(DEV_PORTS_ENV)

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
COMPOSE_PROJECT_NAME ?= edgequake-dev
export COMPOSE_PROJECT_NAME

export

# Environment variables (can be overridden from shell)
OPENAI_API_KEY ?= $(shell echo $$OPENAI_API_KEY)
# SPEC-124 Langfuse: do NOT use `?= $(shell echo $$VAR)` here.
# When unset, that defines an empty Make var; bare `export` above then exports
# LANGFUSE_*='' into every recipe, and `LANGFUSE_FOO="$(LANGFUSE_FOO)"` on
# cargo lines clears a real shell export. Rely on Make's automatic env import,
# `-include .env`, and APPLY_LANGFUSE_ENV (sources .env, never forces empty).

# ── Ingest throughput profile (SPEC-047 / P-G13) ─────────────────────────────
# Detected workstation class: 16 logical CPUs · ~128 GiB RAM.
# Hierarchy (outer → inner):
#   WORKER_THREADS ⊃ MAX_TASKS_PER_TENANT ⊃ PDF_VISION_JOBS ⊃ PDF_CONCURRENCY
#     ⊃ MM_IMAGE_CONCURRENCY ⊃ MAX_CONCURRENT_EXTRACTIONS
# Peak vision in-flight ≈ PDF_VISION_JOBS × PDF_CONCURRENCY (cloud: 4×4=16).
# Dial down if provider 429s or RSS climbs; set MEM_LIMIT so budget code is aware.
# Low-RAM override example: WORKER_THREADS=4 MAX_TASKS_PER_TENANT=2 \
#   EDGEQUAKE_PDF_VISION_JOBS=1 EDGEQUAKE_PDF_CONCURRENCY=1
# Local (Ollama) concurrency defaults are applied AFTER provider detection below.
# Escape hatch: EDGEQUAKE_ALLOW_LOCAL_HIGH_CONCURRENCY=1 \
#   EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS=8 make dev
# WORKER_THREADS / MAX_TASKS_PER_TENANT / EMBED_MAX_ASYNC / MERGE_MAX_ASYNC
# are set in the provider-aware block below (local vs cloud).
EDGEQUAKE_MEM_LIMIT ?= 48g
# SPEC-034: native SQL AGE upserts (~69× faster than Cypher MERGE).
EDGEQUAKE_NATIVE_GRAPH_WRITES ?= 1
# Mix intent arm gate (default on = production). Bench001 Acc fairness uses false (LR-like 3 arms).
# Product Smart = LightRAG mix: always local∥global∥naive (065). Acc pins false too.
EDGEQUAKE_MIX_ARM_GATE ?= false
# SPEC-086 E2-occ = product best Mix profile (Acc law + LightRAG identity).
# Acc still pins these explicitly; product/dev backends inherit the same defaults.
EDGEQUAKE_MIX_FUSION ?= round_robin
EDGEQUAKE_HYBRID_FUSION ?= round_robin
EDGEQUAKE_GRAPH_WALK ?= bfs
EDGEQUAKE_ENTITY_RANK ?= retrieval
EDGEQUAKE_KG_CHUNK_OCCURRENCE_SORT ?= 1
EDGEQUAKE_KG_CHUNK_PICK_LR_BUDGET ?= 1
EDGEQUAKE_L2_BM25_UNION ?= 1
EDGEQUAKE_L2_BM25_MODE ?= fact_replace
EDGEQUAKE_L2_BM25_MIX_TOP_K ?= 30
# SPEC-047: skip Louvain tax in local/bench; fail-open MM for throughput.
EDGEQUAKE_COMMUNITY_GLOBAL ?= false
EDGEQUAKE_MULTIMODAL_FAIL_MODE ?= degraded
# Local never-stuck Pass B: classify-only, figure cap, wall budget (cloud ignores defaults).
EDGEQUAKE_MM_LOCAL_CLASSIFY_ONLY ?= 1
EDGEQUAKE_MM_MAX_FIGURES ?= 12
EDGEQUAKE_MM_PASS_B_TIMEOUT_SECS ?= 600
EDGEQUAKE_MM_PASS_B_PAGE_TIMEOUT_SECS ?= 90
# SPEC-026/047: inline chart/figure VLM analyze (Pass B). Opt out with false.
VLM_PROCESS_ENABLE ?= true
# SPEC-047 P5: HNSW upsert progress cadence (default 1000); chunk_only skips KG extract.
EDGEQUAKE_VECTOR_UPSERT_CHUNK ?= 1000
EDGEQUAKE_INGEST_PROFILE ?= full
# SPEC-047 P7a: LightRAG FORCE_LLM_SUMMARY_ON_MERGE — join <N fragments with <SEP>, else LLM.
EDGEQUAKE_FORCE_LLM_SUMMARY_ON_MERGE ?= 8
# SPEC-047 P7d: LightRAG SOURCE_IDS KEEP — skip description updates when saturated (default 200).
EDGEQUAKE_MAX_SOURCE_IDS_PER_ENTITY ?= 200
EDGEQUAKE_MAX_SOURCE_IDS_PER_RELATION ?= 200
EDGEQUAKE_SOURCE_IDS_LIMIT_METHOD ?= KEEP
# SPEC-047 P7f: native/Cypher graph upsert chunk size (rows per UNNEST/UNWIND statement).
EDGEQUAKE_GRAPH_UPSERT_CHUNK ?= 500

# SPEC-091 W1/Wave D: chunk text is relational-authoritative. The KV store was
# dropped by migration 125 and the backfill verified zero mismatches, so the
# spine (public.chunks) is the SSOT. `dual`/`kv` are rollback-only settings for
# deployments that have NOT run the drop (post-drop they hit 42P01).
EDGEQUAKE_CHUNK_TEXT_AUTHORITY ?= relational
EDGEQUAKE_MIGRATION_MODE ?= automatic
EDGEQUAKE_SERVING_FENCE ?= on
# SPEC-091 W2: dedup hash family reads the typed ingestion_dedup table in dev
# (migration 117 backfills legacy KV rows at boot before serving).
EDGEQUAKE_KV_FAMILY_DOC_HASH ?= relational
EDGEQUAKE_KV_FAMILY_WSDOC ?= relational
EDGEQUAKE_KV_FAMILY_CHECKPOINT ?= relational
EDGEQUAKE_KV_FAMILY_ARTIFACT ?= relational
EDGEQUAKE_KV_FAMILY_INJECTION ?= relational
EDGEQUAKE_KV_FAMILY_METADATA ?= relational

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

# Provider-aware ingest concurrency (must follow DEFAULT_LLM_PROVIDER resolution).
# Ollama ~1 parallel sequence — cloud-scale fan-out (32) causes connection storms.
# Local profile also clamps workers / embed / merge (parity with extract=2).
ifeq ($(EDGEQUAKE_DEFAULT_LLM_PROVIDER),$(filter $(EDGEQUAKE_DEFAULT_LLM_PROVIDER),ollama lmstudio lm-studio lm_studio))
  WORKER_THREADS ?= 2
  MAX_TASKS_PER_TENANT ?= 1
  EDGEQUAKE_PDF_CONCURRENCY ?= 1
  EDGEQUAKE_PDF_VISION_JOBS ?= 1
  EDGEQUAKE_MM_IMAGE_CONCURRENCY ?= 1
  # Serial local extract: Ollama `-np 1` + gate budget 1 (reliability plan).
  EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS ?= 1
  EDGEQUAKE_EMBED_MAX_ASYNC ?= 1
  EDGEQUAKE_MERGE_MAX_ASYNC ?= 1
  EDGEQUAKE_LOCAL_MAX_INFLIGHT ?= 1
  EDGEQUAKE_PROVIDER_BUDGET ?= 1
  EDGEQUAKE_EXTRACT_REASONING_EFFORT ?= none
  OLLAMA_CONTEXT_LENGTH ?= 8192
  # Leave headroom for interactive HTTP reads under gemma4 ingest.
  DATABASE_POOL_SIZE ?= 16
else
  WORKER_THREADS ?= 16
  MAX_TASKS_PER_TENANT ?= 12
  EDGEQUAKE_PDF_CONCURRENCY ?= 4
  EDGEQUAKE_PDF_VISION_JOBS ?= 4
  EDGEQUAKE_MM_IMAGE_CONCURRENCY ?= 8
  EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS ?= 32
  EDGEQUAKE_EMBED_MAX_ASYNC ?= 8
  EDGEQUAKE_MERGE_MAX_ASYNC ?= 8
  EDGEQUAKE_LOCAL_MAX_INFLIGHT ?= 0
  EDGEQUAKE_PROVIDER_BUDGET ?= 0
  DATABASE_POOL_SIZE ?= 32
endif

# Extract think-off (`none` → Ollama `think:false`) is local-only.
# Cloud OpenAI/OpenRouter treat `none` as disable-reasoning; many endpoints 400
# ("Reasoning is mandatory for this endpoint and cannot be disabled").
# Do not pin EDGEQUAKE_EXTRACT_REASONING_EFFORT=none for openai / openrouter —
# extract uses the registry's lowest *enabled* effort (minimal/low).
OLLAMA_CONTEXT_LENGTH ?= 8192
EDGEQUAKE_PROVIDER_BUDGET ?= 1

export WORKER_THREADS MAX_TASKS_PER_TENANT \
	EDGEQUAKE_PDF_CONCURRENCY EDGEQUAKE_PDF_VISION_JOBS \
	EDGEQUAKE_MM_IMAGE_CONCURRENCY EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS \
	EDGEQUAKE_MEM_LIMIT EDGEQUAKE_NATIVE_GRAPH_WRITES \
	EDGEQUAKE_COMMUNITY_GLOBAL EDGEQUAKE_MULTIMODAL_FAIL_MODE VLM_PROCESS_ENABLE \
	EDGEQUAKE_MM_LOCAL_CLASSIFY_ONLY EDGEQUAKE_MM_MAX_FIGURES \
	EDGEQUAKE_MM_PASS_B_TIMEOUT_SECS EDGEQUAKE_MM_PASS_B_PAGE_TIMEOUT_SECS \
	EDGEQUAKE_VECTOR_UPSERT_CHUNK EDGEQUAKE_INGEST_PROFILE \
	EDGEQUAKE_EMBED_MAX_ASYNC EDGEQUAKE_FORCE_LLM_SUMMARY_ON_MERGE \
	EDGEQUAKE_MERGE_MAX_ASYNC EDGEQUAKE_MAX_SOURCE_IDS_PER_ENTITY \
	EDGEQUAKE_MAX_SOURCE_IDS_PER_RELATION EDGEQUAKE_SOURCE_IDS_LIMIT_METHOD \
	EDGEQUAKE_GRAPH_UPSERT_CHUNK EDGEQUAKE_LOCAL_MAX_INFLIGHT \
	EDGEQUAKE_PROVIDER_BUDGET EDGEQUAKE_EXTRACT_REASONING_EFFORT \
	OLLAMA_CONTEXT_LENGTH DATABASE_POOL_SIZE

# SPEC-124: load repo `.env` into the current recipe shell, then apply Make/CLI
# overrides ONLY when the shell var is still empty (do not clobber bash-sourced
# values). GNU Make `-include .env` keeps surrounding quotes in $(VAR); bash
# `. .env` strips them — overwriting after source caused silent Langfuse 401.
# Always strip one matching quote pair after apply so process env stays clean.
define APPLY_LANGFUSE_ENV
set -a; [ -f "$(ROOT_DIR)/.env" ] && . "$(ROOT_DIR)/.env"; set +a; \
[ -z "$$LANGFUSE_PUBLIC_KEY" ] && [ -n "$(LANGFUSE_PUBLIC_KEY)" ] && export LANGFUSE_PUBLIC_KEY="$(LANGFUSE_PUBLIC_KEY)"; \
[ -z "$$LANGFUSE_SECRET_KEY" ] && [ -n "$(LANGFUSE_SECRET_KEY)" ] && export LANGFUSE_SECRET_KEY="$(LANGFUSE_SECRET_KEY)"; \
[ -z "$$LANGFUSE_BASE_URL" ] && [ -n "$(LANGFUSE_BASE_URL)" ] && export LANGFUSE_BASE_URL="$(LANGFUSE_BASE_URL)"; \
[ -z "$$LANGFUSE_HOST" ] && [ -n "$(LANGFUSE_HOST)" ] && export LANGFUSE_HOST="$(LANGFUSE_HOST)"; \
[ -z "$$LANGFUSE_PROJECT_ID" ] && [ -n "$(LANGFUSE_PROJECT_ID)" ] && export LANGFUSE_PROJECT_ID="$(LANGFUSE_PROJECT_ID)"; \
[ -z "$$EDGEQUAKE_LANGFUSE_ENABLED" ] && [ -n "$(EDGEQUAKE_LANGFUSE_ENABLED)" ] && export EDGEQUAKE_LANGFUSE_ENABLED="$(EDGEQUAKE_LANGFUSE_ENABLED)"; \
_eq_unquote_env() { \
	_v=$$1; \
	case "$$_v" in \
		\"*\") _v=$${_v#\"}; _v=$${_v%\"} ;; \
		\'*\') _v=$${_v#\'}; _v=$${_v%\'} ;; \
	esac; \
	printf '%s' "$$_v"; \
}; \
[ -n "$$LANGFUSE_PUBLIC_KEY" ] && export LANGFUSE_PUBLIC_KEY="$$(_eq_unquote_env "$$LANGFUSE_PUBLIC_KEY")"; \
[ -n "$$LANGFUSE_SECRET_KEY" ] && export LANGFUSE_SECRET_KEY="$$(_eq_unquote_env "$$LANGFUSE_SECRET_KEY")"; \
[ -n "$$LANGFUSE_BASE_URL" ] && export LANGFUSE_BASE_URL="$$(_eq_unquote_env "$$LANGFUSE_BASE_URL")"; \
[ -n "$$LANGFUSE_HOST" ] && export LANGFUSE_HOST="$$(_eq_unquote_env "$$LANGFUSE_HOST")"; \
[ -n "$$LANGFUSE_PROJECT_ID" ] && export LANGFUSE_PROJECT_ID="$$(_eq_unquote_env "$$LANGFUSE_PROJECT_ID")"; \
[ -n "$$EDGEQUAKE_LANGFUSE_ENABLED" ] && export EDGEQUAKE_LANGFUSE_ENABLED="$$(_eq_unquote_env "$$EDGEQUAKE_LANGFUSE_ENABLED")"; \
if [ -n "$$LANGFUSE_PUBLIC_KEY" ] && [ -n "$$LANGFUSE_SECRET_KEY" ]; then \
	echo "$(YELLOW)→ LANGFUSE_* keys detected — Langfuse OTLP export enabled (SPEC-124)$(RESET)"; \
else \
	echo "$(YELLOW)→ LANGFUSE_* not set — add to $(ROOT_DIR)/.env or export in this shell, then restart$(RESET)"; \
fi
endef

# SPEC-124: force Compose headless init keys (wins over .env Cloud/placeholder).
define APPLY_LANGFUSE_LOCAL_ENV
export LANGFUSE_PUBLIC_KEY="$(LANGFUSE_LOCAL_PK)"; \
export LANGFUSE_SECRET_KEY="$(LANGFUSE_LOCAL_SK)"; \
export LANGFUSE_BASE_URL="$(LANGFUSE_UI_URL)"; \
export LANGFUSE_HOST="$(LANGFUSE_UI_URL)"; \
export LANGFUSE_PROJECT_ID="$(LANGFUSE_LOCAL_PROJECT_ID)"; \
export EDGEQUAKE_LANGFUSE_ENABLED=1; \
echo "$(YELLOW)→ WITH_LANGFUSE=1 — forcing local Langfuse keys ($(LANGFUSE_UI_URL))$(RESET)"
endef

define APPLY_LANGFUSE_ENV_EFFECTIVE
$(APPLY_LANGFUSE_ENV); \
if [ "$(WITH_LANGFUSE)" = "1" ]; then \
	$(APPLY_LANGFUSE_LOCAL_ENV); \
fi
endef

# Exit 0 when GET /api/v1/settings/langfuse is export_active against local UI.
define LANGFUSE_LOCAL_BACKEND_WIRED
curl -sf "$(BACKEND_URL)/api/v1/settings/langfuse" | python3 -c 'import json,sys; b=json.load(sys.stdin); ui=(b.get("ui_url") or b.get("base_url") or "").rstrip("/"); want=sys.argv[1].rstrip("/"); sys.exit(0 if b.get("export_active") and ui==want else 1)' "$(LANGFUSE_UI_URL)"
endef

# Shared exports appended to /tmp/edgequake-start.sh by backend-bg.
# SPEC-047: also pin VLM + chart modality so bench restarts do not silently drop MV-32.
define BACKEND_STABILITY_EXPORTS
printf '%s\n' "export WORKER_THREADS=\"$(WORKER_THREADS)\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export MAX_TASKS_PER_TENANT=\"$(MAX_TASKS_PER_TENANT)\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_PDF_CONCURRENCY=\"$(EDGEQUAKE_PDF_CONCURRENCY)\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_PDF_VISION_JOBS=\"$(EDGEQUAKE_PDF_VISION_JOBS)\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_MM_IMAGE_CONCURRENCY=\"$(EDGEQUAKE_MM_IMAGE_CONCURRENCY)\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS=\"$(EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS)\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_PROVIDER_BUDGET=\"$${EDGEQUAKE_PROVIDER_BUDGET:-$(EDGEQUAKE_PROVIDER_BUDGET)}\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_EXTRACT_REASONING_EFFORT=\"$${EDGEQUAKE_EXTRACT_REASONING_EFFORT:-$(EDGEQUAKE_EXTRACT_REASONING_EFFORT)}\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export OLLAMA_CONTEXT_LENGTH=\"$${OLLAMA_CONTEXT_LENGTH:-$(OLLAMA_CONTEXT_LENGTH)}\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_MEM_LIMIT=\"$(EDGEQUAKE_MEM_LIMIT)\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_NATIVE_GRAPH_WRITES=\"$(EDGEQUAKE_NATIVE_GRAPH_WRITES)\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_COMMUNITY_GLOBAL=\"$(EDGEQUAKE_COMMUNITY_GLOBAL)\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_MULTIMODAL_FAIL_MODE=\"$(EDGEQUAKE_MULTIMODAL_FAIL_MODE)\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_MM_LOCAL_CLASSIFY_ONLY=\"$(EDGEQUAKE_MM_LOCAL_CLASSIFY_ONLY)\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_MM_MAX_FIGURES=\"$(EDGEQUAKE_MM_MAX_FIGURES)\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_MM_PASS_B_TIMEOUT_SECS=\"$(EDGEQUAKE_MM_PASS_B_TIMEOUT_SECS)\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_MM_PASS_B_PAGE_TIMEOUT_SECS=\"$(EDGEQUAKE_MM_PASS_B_PAGE_TIMEOUT_SECS)\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_VECTOR_UPSERT_CHUNK=\"$(EDGEQUAKE_VECTOR_UPSERT_CHUNK)\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_INGEST_PROFILE=\"$(EDGEQUAKE_INGEST_PROFILE)\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_EMBED_MAX_ASYNC=\"$(EDGEQUAKE_EMBED_MAX_ASYNC)\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_FORCE_LLM_SUMMARY_ON_MERGE=\"$(EDGEQUAKE_FORCE_LLM_SUMMARY_ON_MERGE)\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_MERGE_MAX_ASYNC=\"$(EDGEQUAKE_MERGE_MAX_ASYNC)\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_MAX_SOURCE_IDS_PER_ENTITY=\"$(EDGEQUAKE_MAX_SOURCE_IDS_PER_ENTITY)\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_MAX_SOURCE_IDS_PER_RELATION=\"$(EDGEQUAKE_MAX_SOURCE_IDS_PER_RELATION)\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_SOURCE_IDS_LIMIT_METHOD=\"$(EDGEQUAKE_SOURCE_IDS_LIMIT_METHOD)\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_GRAPH_UPSERT_CHUNK=\"$(EDGEQUAKE_GRAPH_UPSERT_CHUNK)\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_CHUNK_TEXT_AUTHORITY=\"$(EDGEQUAKE_CHUNK_TEXT_AUTHORITY)\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_MIGRATION_MODE=\"$(EDGEQUAKE_MIGRATION_MODE)\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_SERVING_FENCE=\"$(EDGEQUAKE_SERVING_FENCE)\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_KV_FAMILY_DOC_HASH=\"$(EDGEQUAKE_KV_FAMILY_DOC_HASH)\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_KV_FAMILY_WSDOC=\"$(EDGEQUAKE_KV_FAMILY_WSDOC)\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_KV_FAMILY_CHECKPOINT=\"$(EDGEQUAKE_KV_FAMILY_CHECKPOINT)\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_KV_FAMILY_ARTIFACT=\"$(EDGEQUAKE_KV_FAMILY_ARTIFACT)\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_KV_FAMILY_INJECTION=\"$(EDGEQUAKE_KV_FAMILY_INJECTION)\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_KV_FAMILY_METADATA=\"$(EDGEQUAKE_KV_FAMILY_METADATA)\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_LOCAL_MAX_INFLIGHT=\"$(EDGEQUAKE_LOCAL_MAX_INFLIGHT)\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_MIX_ARM_GATE=\"$(EDGEQUAKE_MIX_ARM_GATE)\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_MIX_FUSION=\"$(EDGEQUAKE_MIX_FUSION)\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_HYBRID_FUSION=\"$(EDGEQUAKE_HYBRID_FUSION)\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_GRAPH_WALK=\"$(EDGEQUAKE_GRAPH_WALK)\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_ENTITY_RANK=\"$(EDGEQUAKE_ENTITY_RANK)\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_KG_CHUNK_OCCURRENCE_SORT=\"$(EDGEQUAKE_KG_CHUNK_OCCURRENCE_SORT)\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_KG_CHUNK_PICK_LR_BUDGET=\"$(EDGEQUAKE_KG_CHUNK_PICK_LR_BUDGET)\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_L2_BM25_UNION=\"$(EDGEQUAKE_L2_BM25_UNION)\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_L2_BM25_MODE=\"$(EDGEQUAKE_L2_BM25_MODE)\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_L2_BM25_MIX_TOP_K=\"$(EDGEQUAKE_L2_BM25_MIX_TOP_K)\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_RELATED_CHUNK_NUMBER=\"$${EDGEQUAKE_RELATED_CHUNK_NUMBER:-5}\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_ORPHAN_RETRACT_ON_RECOVER=\"$${EDGEQUAKE_ORPHAN_RETRACT_ON_RECOVER:-0}\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_ADAPTIVE_CHUNKING=\"$${EDGEQUAKE_ADAPTIVE_CHUNKING:-1}\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_CHUNK_SIZE=\"$${EDGEQUAKE_CHUNK_SIZE:-1200}\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_CHUNK_OVERLAP=\"$${EDGEQUAKE_CHUNK_OVERLAP:-100}\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export DATABASE_POOL_SIZE=\"$(DATABASE_POOL_SIZE)\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export VLM_PROCESS_ENABLE=\"$(VLM_PROCESS_ENABLE)\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_CHART_MODALITY_FILTER=\"true\"" >> /tmp/edgequake-start.sh; \
printf '%s\n' "export EDGEQUAKE_MM_ANALYSIS_CACHE=\"true\"" >> /tmp/edgequake-start.sh; \
[ -n "$${LANGFUSE_PUBLIC_KEY}" ] && printf '%s\n' "export LANGFUSE_PUBLIC_KEY=\"$${LANGFUSE_PUBLIC_KEY}\"" >> /tmp/edgequake-start.sh; \
[ -n "$${LANGFUSE_SECRET_KEY}" ] && printf '%s\n' "export LANGFUSE_SECRET_KEY=\"$${LANGFUSE_SECRET_KEY}\"" >> /tmp/edgequake-start.sh; \
[ -n "$${LANGFUSE_BASE_URL}" ] && printf '%s\n' "export LANGFUSE_BASE_URL=\"$${LANGFUSE_BASE_URL}\"" >> /tmp/edgequake-start.sh; \
[ -n "$${LANGFUSE_HOST}" ] && printf '%s\n' "export LANGFUSE_HOST=\"$${LANGFUSE_HOST}\"" >> /tmp/edgequake-start.sh; \
[ -n "$${LANGFUSE_PROJECT_ID}" ] && printf '%s\n' "export LANGFUSE_PROJECT_ID=\"$${LANGFUSE_PROJECT_ID}\"" >> /tmp/edgequake-start.sh; \
[ -n "$${EDGEQUAKE_LANGFUSE_ENABLED}" ] && printf '%s\n' "export EDGEQUAKE_LANGFUSE_ENABLED=\"$${EDGEQUAKE_LANGFUSE_ENABLED}\"" >> /tmp/edgequake-start.sh; \
if [ "$${WAVE2_GREENFIELD:-$(WAVE2_GREENFIELD)}" = "1" ]; then \
  printf '%s\n' "export EDGEQUAKE_VECTOR_STORAGE=halfvec" >> /tmp/edgequake-start.sh; \
  printf '%s\n' "export EDGEQUAKE_HNSW_PARTIAL_BY_WORKSPACE=1" >> /tmp/edgequake-start.sh; \
  printf '%s\n' "export EDGEQUAKE_HNSW_EF_SEARCH=240" >> /tmp/edgequake-start.sh; \
  echo "$(YELLOW)→ WAVE2_GREENFIELD=1 — halfvec + partial HNSW + ef_search=240 (SPEC-071 turnkey)$(RESET)"; \
fi;
endef

# SPEC-071: opt-in Wave-2 greenfield (empty = off; set WAVE2_GREENFIELD=1 for make dev / backend-bg)
WAVE2_GREENFIELD ?=

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
	@echo "  $(GREEN)make dev-langfuse$(RESET) Full stack + local Langfuse v4 (UI :3310; injects init keys)"
	@echo "  $(GREEN)make dev-pg16$(RESET)     Start dev stack with PostgreSQL 16 (legacy)"
	@echo "  $(GREEN)make dev-pg17$(RESET)     Start dev stack with PostgreSQL 17"
	@echo "  $(GREEN)make dev-pg18$(RESET)     Start dev stack with PostgreSQL 18 (same as make dev)"
	@echo "  $(GREEN)make dev-auth$(RESET)     Start full development stack with authentication enabled"
	@echo "  $(GREEN)make dev-bg$(RESET)       Start full stack in BACKGROUND without authentication"
	@echo "  $(GREEN)make dev-bg-langfuse$(RESET) Background stack + local Langfuse v4 (UI :3310)"
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
	@echo "  $(GREEN)make wave2-greenfield-env$(RESET)  Print Wave-2 100k turnkey exports (claim gates ≠ day-2 sizing; docs/product-limits.md)"
	@echo "  $(GREEN)WAVE2_GREENFIELD=1 make backend-bg$(RESET)  Opt-in Wave-2 recipe for greenfield installs"
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
	@echo "  $(GREEN)make langfuse-up$(RESET)             Start local Langfuse v4 (UI :3310, optional)"
	@echo "  $(GREEN)make langfuse-down$(RESET)           Stop local Langfuse (keeps volumes)"
	@echo "  $(GREEN)make langfuse-smoke$(RESET)          Health + GET /api/public/projects"
	@echo "  $(GREEN)make spec124-langfuse-e2e$(RESET)    One-command live Settings + sessions vs local Langfuse (starts stack; needs Ollama or OPENAI_API_KEY)"
	@echo "  $(GREEN)make langfuse-3.1-up$(RESET)         Start Langfuse 3.1.1 (UI :3320, ingestion-fallback E2E)"
	@echo "  $(GREEN)make spec124-langfuse-3.1-e2e$(RESET) Unfakable ingestion-fallback E2E vs Langfuse 3.1.1"
	@echo "  $(GREEN)make langfuse-3.1-reset$(RESET)      Wipe Langfuse 3.1.1 volumes (CONFIRM=yes)"
	@echo "  $(GREEN)make langfuse-3.22-up$(RESET)        Start Langfuse 3.22.0 (UI :3330, first OTLP)"
	@echo "  $(GREEN)make spec124-langfuse-3.22-e2e$(RESET) Unfakable OTLP route+probe vs Langfuse 3.22.0"
	@echo "  $(GREEN)make spec124-langfuse-3.225-e2e$(RESET) Unfakable OTLP persist E2E vs Langfuse 3.225.5"
	@echo "  $(GREEN)make spec124-langfuse-cloud-e2e$(RESET) Unfakable OTLP persist E2E vs current Langfuse Cloud"
	@echo "  $(GREEN)make spec124-langfuse-matrix$(RESET)  3.1.1 + 3.22.0 + 3.225.5 + Cloud (needs Cloud keys)"
	@echo "  $(GREEN)make langfuse-sync-prices$(RESET)    Push models.toml pricing into Langfuse"
	@echo ""
	@echo "$(BOLD)$(BLUE)☸ Kubernetes (SPEC-138)$(RESET)"
	@echo "  $(GREEN)make k8s-prereqs$(RESET)             cert-manager + ClickHouse operator + nginx ingress"
	@echo "  $(GREEN)make k8s-kind-up$(RESET)             Create kind cluster (requires: brew install kind)"
	@echo "  $(GREEN)make k8s-install$(RESET)             Install Langfuse + EdgeQuake Helm stack"
	@echo "  $(GREEN)make spec138-helm-template$(RESET)   Render charts (no cluster)"
	@echo "  $(GREEN)make spec138-kubernetes-proof$(RESET) Full kind E2E — OTLP traces to Langfuse (~16GB RAM)"
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

check-ports: sync-dev-ports ## Validate configured ports without killing unrelated processes
	@echo "$(BLUE)Checking selected ports from $(DEV_PORTS_ENV)...$(RESET)"
	@set -a && . $(DEV_PORTS_ENV) && set +a; \
	if [ "$$BACKEND_PORT" != "$(DEFAULT_BACKEND_PORT)" ]; then \
		echo "$(YELLOW)→ Preferred backend port $(DEFAULT_BACKEND_PORT) is busy; using $$BACKEND_PORT to avoid interference$(RESET)"; \
	fi; \
	if [ "$$FRONTEND_PORT" != "$(DEFAULT_FRONTEND_PORT)" ]; then \
		echo "$(YELLOW)→ Preferred frontend port $(DEFAULT_FRONTEND_PORT) is busy; using $$FRONTEND_PORT instead$(RESET)"; \
		echo "$(YELLOW)  Open $$FRONTEND_URL in your browser for this session$(RESET)"; \
	fi; \
	for port in $$BACKEND_PORT $$FRONTEND_PORT; do \
		PID=$$(lsof -nP -iTCP:$$port -sTCP:LISTEN -t 2>/dev/null | head -n 1 || true); \
		if [ -z "$$PID" ]; then \
			continue; \
		fi; \
		CMD=$$(ps -p "$$PID" -o command= 2>/dev/null || true); \
		if [ "$$port" = "$$BACKEND_PORT" ] && python3 -c "import sys; sys.path.insert(0,'$(ROOT_DIR)/scripts'); from select_edgequake_port import is_edgequake; raise SystemExit(0 if is_edgequake('backend', int(sys.argv[1])) else 1)" "$$port" 2>/dev/null; then \
			echo "$(YELLOW)→ Port $$BACKEND_PORT is already serving EdgeQuake; reusing it$(RESET)"; \
			continue; \
		fi; \
		if [ "$$port" = "$$FRONTEND_PORT" ] && curl -fsS "$$FRONTEND_URL" 2>/dev/null | grep -qi 'EdgeQuake'; then \
			echo "$(YELLOW)→ Port $$FRONTEND_PORT is already serving the EdgeQuake UI; reusing it$(RESET)"; \
			continue; \
		fi; \
		echo "$(RED)✗ Selected port $$port is already bound by another application$(RESET)"; \
		echo "  PID: $$PID"; \
		echo "  CMD: $$CMD"; \
		echo "  Hint: run $(GREEN)make sync-dev-ports$(RESET) or set BACKEND_PORT/FRONTEND_PORT explicitly."; \
		exit 1; \
	done
	@echo "$(GREEN)✓ Port check complete$(RESET)"

sync-dev-ports: ## Regenerate collision-safe dev ports (.edgequake-dev-ports.env)
	@python3 $(ROOT_DIR)/scripts/sync_dev_ports.py $(DEFAULT_BACKEND_PORT) $(DEFAULT_FRONTEND_PORT) $(PORT_SCAN_WINDOW) >/dev/null
	@echo "$(GREEN)✓ Dev ports:$(RESET) backend $$(grep '^BACKEND_PORT=' $(DEV_PORTS_ENV) | cut -d= -f2) · frontend $$(grep '^FRONTEND_PORT=' $(DEV_PORTS_ENV) | cut -d= -f2)"
	@echo "  UI: $$(grep '^FRONTEND_URL=' $(DEV_PORTS_ENV) | cut -d= -f2) · API: $$(grep '^BACKEND_URL=' $(DEV_PORTS_ENV) | cut -d= -f2)"

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
	@if [ "$(WITH_LANGFUSE)" = "1" ]; then \
		echo "$(YELLOW)→ Starting local Langfuse v4 (WITH_LANGFUSE=1)...$(RESET)"; \
		$(MAKE) langfuse-up --no-print-directory; \
	fi
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
	@if [ "$(WITH_LANGFUSE)" = "1" ]; then \
		echo "  $(BLUE)Langfuse$(RESET): $(LANGFUSE_UI_URL)  (login: dev@example.com / edgequake-local-dev)"; \
	fi
	@echo ""
	@DEV_LOCK="/tmp/edgequake-make-dev.lock"; \
	if ! mkdir "$$DEV_LOCK" 2>/dev/null; then \
		OTHER=$$(cat "$$DEV_LOCK/pid" 2>/dev/null || true); \
		if [ -n "$$OTHER" ] && kill -0 "$$OTHER" 2>/dev/null; then \
			echo "$(RED)✗ Another make dev is already running (pid $$OTHER).$(RESET)"; \
			echo "  Stop it with Ctrl+C in that terminal, or run $(GREEN)make kill-app$(RESET) then retry."; \
			exit 1; \
		fi; \
		rm -rf "$$DEV_LOCK"; \
		mkdir "$$DEV_LOCK" || exit 1; \
	fi; \
	echo "$$$$" > "$$DEV_LOCK/pid"; \
	trap 'echo ""; echo "$(YELLOW)Stopping only the processes started by this make dev session...$(RESET)"; [ -n "$$BACKEND_PID" ] && kill "$$BACKEND_PID" 2>/dev/null || true; [ -n "$$FRONTEND_PID" ] && kill "$$FRONTEND_PID" 2>/dev/null || true; rm -rf /tmp/edgequake-make-dev.lock; echo "$(GREEN)✓ App processes stopped. PostgreSQL is left running for faster restarts.$(RESET)"; exit 0' INT; \
	set -a && . $(DEV_PORTS_ENV) && set +a; \
	BACKEND_PID=""; \
	FRONTEND_PID=""; \
	$(LOAD_EFF_DB_URL); \
	$(APPLY_LANGFUSE_ENV_EFFECTIVE); \
	$(VISIBLE_MIGRATE_STEP); \
	for BPID in $$(lsof -nP -iTCP:$$BACKEND_PORT -sTCP:LISTEN -t 2>/dev/null || true); do \
		echo "$(YELLOW)→ Freeing port $$BACKEND_PORT (PID $$BPID) before backend start$(RESET)"; \
		kill -9 "$$BPID" 2>/dev/null || true; \
	done; \
	sleep 0.3; \
	echo "$(YELLOW)→ Starting backend on port $$BACKEND_PORT (DATABASE_URL port: $$(printf '%s' $$_EFF_DB_URL | sed -E 's|.*:([0-9]+)/.*|\1|'))...$(RESET)"; \
	$(APPLY_LANGFUSE_ENV_EFFECTIVE); \
	if [ -n "$(OPENAI_API_KEY)" ]; then \
		(cd $(BACKEND_DIR) && \
			PORT="$$BACKEND_PORT" \
			DATABASE_URL="$$_EFF_DB_URL" \
			OPENAI_API_KEY="$(OPENAI_API_KEY)" \
			EDGEQUAKE_DEV_MODE="$(DEV_EDGEQUAKE_DEV_MODE)" \
		EDGEQUAKE_AUTH_ENABLED="$(DEV_AUTH_ENABLED)" \
		AUTH_ENABLED="$(DEV_AUTH_ENABLED)" \
		EDGEQUAKE_NATIVE_GRAPH_WRITES="$(EDGEQUAKE_NATIVE_GRAPH_WRITES)" \
		VLM_PROCESS_ENABLE="$(VLM_PROCESS_ENABLE)" \
		EDGEQUAKE_MULTIMODAL_FAIL_MODE="$(EDGEQUAKE_MULTIMODAL_FAIL_MODE)" \
		EDGEQUAKE_MM_LOCAL_CLASSIFY_ONLY="$(EDGEQUAKE_MM_LOCAL_CLASSIFY_ONLY)" \
		EDGEQUAKE_MM_MAX_FIGURES="$(EDGEQUAKE_MM_MAX_FIGURES)" \
		EDGEQUAKE_MM_PASS_B_TIMEOUT_SECS="$(EDGEQUAKE_MM_PASS_B_TIMEOUT_SECS)" \
		EDGEQUAKE_MM_PASS_B_PAGE_TIMEOUT_SECS="$(EDGEQUAKE_MM_PASS_B_PAGE_TIMEOUT_SECS)" \
		EDGEQUAKE_MM_IMAGE_CONCURRENCY="$(EDGEQUAKE_MM_IMAGE_CONCURRENCY)" \
		cargo run 2>&1 | sed 's/^/[backend] /') & \
	BACKEND_PID=$$!; \
	else \
		(cd $(BACKEND_DIR) && \
			PORT="$$BACKEND_PORT" \
			DATABASE_URL="$$_EFF_DB_URL" \
			EDGEQUAKE_DEV_MODE="$(DEV_EDGEQUAKE_DEV_MODE)" \
		EDGEQUAKE_AUTH_ENABLED="$(DEV_AUTH_ENABLED)" \
		AUTH_ENABLED="$(DEV_AUTH_ENABLED)" \
		EDGEQUAKE_NATIVE_GRAPH_WRITES="$(EDGEQUAKE_NATIVE_GRAPH_WRITES)" \
		VLM_PROCESS_ENABLE="$(VLM_PROCESS_ENABLE)" \
		EDGEQUAKE_MULTIMODAL_FAIL_MODE="$(EDGEQUAKE_MULTIMODAL_FAIL_MODE)" \
		EDGEQUAKE_MM_LOCAL_CLASSIFY_ONLY="$(EDGEQUAKE_MM_LOCAL_CLASSIFY_ONLY)" \
		EDGEQUAKE_MM_MAX_FIGURES="$(EDGEQUAKE_MM_MAX_FIGURES)" \
		EDGEQUAKE_MM_PASS_B_TIMEOUT_SECS="$(EDGEQUAKE_MM_PASS_B_TIMEOUT_SECS)" \
		EDGEQUAKE_MM_PASS_B_PAGE_TIMEOUT_SECS="$(EDGEQUAKE_MM_PASS_B_PAGE_TIMEOUT_SECS)" \
		EDGEQUAKE_MM_IMAGE_CONCURRENCY="$(EDGEQUAKE_MM_IMAGE_CONCURRENCY)" \
		OLLAMA_HOST="http://localhost:11434" \
			OLLAMA_MODEL="gemma4:latest" \
			OLLAMA_EMBEDDING_MODEL="embeddinggemma:latest" \
			OLLAMA_CONTEXT_LENGTH="$${OLLAMA_CONTEXT_LENGTH:-8192}" \
			cargo run 2>&1 | sed 's/^/[backend] /') & \
		BACKEND_PID=$$!; \
	fi; \
	echo "$(YELLOW)→ Starting frontend on port $$FRONTEND_PORT...$(RESET)"; \
	(bash $(FRONTEND_DIR)/scripts/ensure-dev-cache.sh && sleep 2 && cd $(FRONTEND_DIR) && PORT="$$FRONTEND_PORT" EDGEQUAKE_API_URL="$$EDGEQUAKE_API_URL" NEXT_PUBLIC_API_URL="$$NEXT_PUBLIC_API_URL" NEXT_PUBLIC_AUTH_ENABLED="$(DEV_AUTH_ENABLED)" NEXT_PUBLIC_DISABLE_DEMO_LOGIN="$(DEV_DISABLE_DEMO_LOGIN)" sh -c '(pnpm run dev 2>/dev/null || bun run dev)' 2>&1 | sed 's/^/[frontend] /') & \
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

dev-langfuse: ## Start full development stack with local Langfuse v4 (UI :3310)
	@$(MAKE) dev --no-print-directory WITH_LANGFUSE=1

dev-bg-langfuse: ## Start background stack with local Langfuse v4 (UI :3310)
	@$(MAKE) dev-bg --no-print-directory WITH_LANGFUSE=1

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
	@if [ "$(WITH_LANGFUSE)" = "1" ]; then \
		echo "$(YELLOW)→ Starting local Langfuse v4 (WITH_LANGFUSE=1)...$(RESET)"; \
		$(MAKE) langfuse-up --no-print-directory; \
	fi
	@echo ""
	@if curl -fsS "$(BACKEND_URL)/health" >/dev/null 2>&1 && [ "$(WITH_LANGFUSE)" != "1" ]; then \
		echo "$(GREEN)✓ Backend already healthy on port $(BACKEND_PORT)$(RESET)"; \
	else \
		echo "$(YELLOW)→ Starting backend in background...$(RESET)"; \
		$(MAKE) backend-bg --no-print-directory DEV_AUTH_ENABLED="$(DEV_AUTH_ENABLED)" WITH_LANGFUSE="$(WITH_LANGFUSE)"; \
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
		echo "  $(BLUE)LLM Provider$(RESET): mistral (mistral-large-latest)"; \
		echo "  $(BLUE)Embedding$(RESET): mistral (mistral-embed, 1024d)"; \
		echo "  $(BLUE)Vision$(RESET): mistral (mistral-large-latest)"; \
	else \
		echo "  $(BLUE)LLM Provider$(RESET): ollama (gemma4:latest)"; \
		echo "  $(BLUE)Embedding$(RESET): ollama (embeddinggemma:latest, 768d)"; \
	fi
	@if [ "$(WITH_LANGFUSE)" = "1" ]; then \
		echo "  $(BLUE)Langfuse$(RESET): $(LANGFUSE_UI_URL)  (login: dev@example.com / edgequake-local-dev)"; \
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
	@-pkill -9 -f "cargo run --bin edgequake" 2>/dev/null || true
	@-set -a && [ -f "$(DEV_PORTS_ENV)" ] && . "$(DEV_PORTS_ENV)" && set +a; \
	for port in "$${BACKEND_PORT:-$(BACKEND_PORT)}" "$(DEFAULT_BACKEND_PORT)" "$(BACKEND_PORT)"; do \
		[ -z "$$port" ] && continue; \
		for BPID in $$(lsof -nP -iTCP:$$port -sTCP:LISTEN -t 2>/dev/null || true); do \
			kill -9 "$$BPID" 2>/dev/null || true; \
		done; \
	done
	@echo "$(YELLOW)→ Killing existing frontend processes...$(RESET)"
	@-if [ -f /tmp/edgequake-frontend.pid ]; then kill -9 $$(cat /tmp/edgequake-frontend.pid) 2>/dev/null || true; rm -f /tmp/edgequake-frontend.pid; fi
	@-pkill -f "node.*edgequake_webui" 2>/dev/null || true
	@-set -a && [ -f "$(DEV_PORTS_ENV)" ] && . "$(DEV_PORTS_ENV)" && set +a; \
	for port in "$${FRONTEND_PORT:-$(FRONTEND_PORT)}" "$(DEFAULT_FRONTEND_PORT)" "$(FRONTEND_PORT)"; do \
		[ -z "$$port" ] && continue; \
		for FPID in $$(lsof -nP -iTCP:$$port -sTCP:LISTEN -t 2>/dev/null || true); do \
			kill -9 "$$FPID" 2>/dev/null || true; \
		done; \
	done
	@rm -rf /tmp/edgequake-make-dev.lock
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

# LAW-MIG / SPEC-111: versions with known broken→fixed checksum repair modules.
# Twin of `KNOWN_CHECKSUM_REPAIR_VERSIONS` in migration_bootstrap/checksum_repair.rs.
# Scoped allowlist so local migrate works even when DEV_AUTH_ENABLED=true (DEV_MODE=false).
KNOWN_CHECKSUM_REPAIR_VERSIONS := 71,78,118,121,125,131

# SPEC-091 Doc 17 (LD-15): explicit, visible schema apply before any server
# start. The server binary never auto-migrates — boot refuses (exit 78) when
# expandable schema is behind. Irreversible drops (125/126/131) stay human-gated:
# `edgequake migrate` applies expandables first, then soft-exits with WARN when
# only drops remain (so make_dev can start). Confirm with --confirm-drop when
# drop-readiness is GREEN. A hard failure here still aborts before the server.
#
# LAW-MIG-3: pass scoped checksum-repair allowlist (+ DEV_MODE for local friction).
# Production images leave both unset and fail loud (X-02).
VISIBLE_MIGRATE_STEP = \
	echo "$(YELLOW)→ edgequake migrate — applying database schema (explicit step, SPEC-091 LD-15)$(RESET)"; \
	( cd $(BACKEND_DIR) && \
		DATABASE_URL="$$_EFF_DB_URL" \
		EDGEQUAKE_DEV_MODE="$(DEV_EDGEQUAKE_DEV_MODE)" \
		EDGEQUAKE_ALLOW_CHECKSUM_REPAIR="$(KNOWN_CHECKSUM_REPAIR_VERSIONS)" \
		cargo run -- migrate ) || { \
		echo "$(RED)✗ edgequake migrate failed — server not started.$(RESET)"; \
		echo "  Preview impact first: (cd $(BACKEND_DIR) && DATABASE_URL=\"$$_EFF_DB_URL\" cargo run -- migrate dry-run)"; \
		echo "  If checksum drift: EDGEQUAKE_ALLOW_CHECKSUM_REPAIR=$(KNOWN_CHECKSUM_REPAIR_VERSIONS) cargo run -- migrate"; \
		echo "  Spec: specs/111-issues/10-migration-immutability.md"; \
		echo "  If only an irreversible drop remains, soft-exit is expected — check WARN above."; \
		echo "  When fleet/KV drop-readiness is GREEN: cargo run -- migrate --confirm-drop"; \
		exit 1; \
	}

# SPEC-040 v0.4.1: pdfium is now EMBEDDED in the edgequake-pdf2md 0.4.1 binary
# via pdfium-auto at compile time. No external libpdfium.dylib, no env vars needed.

backend-dev: db-wait ## Run backend in development mode with PostgreSQL (uses .env configuration)
	@echo "$(BLUE)Starting backend with PostgreSQL storage...$(RESET)"
	@if [ -n "$(EDGEQUAKE_DEFAULT_LLM_PROVIDER)" ]; then \
		echo "$(GREEN)✓ LLM Provider: $(EDGEQUAKE_DEFAULT_LLM_PROVIDER) ($(EDGEQUAKE_DEFAULT_LLM_MODEL))$(RESET)"; \
	fi
	@	$(LOAD_EFF_DB_URL); \
	$(VISIBLE_MIGRATE_STEP); \
	$(APPLY_LANGFUSE_ENV_EFFECTIVE); \
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
		VLM_PROCESS_ENABLE="$(VLM_PROCESS_ENABLE)" \
		OLLAMA_HOST="http://localhost:11434" \
		OLLAMA_MODEL="gemma4:latest" \
		OLLAMA_EMBEDDING_MODEL="embeddinggemma:latest" \
		OLLAMA_CONTEXT_LENGTH="$${OLLAMA_CONTEXT_LENGTH:-8192}" \
		cargo run

backend-db: db-wait ## Run backend with PostgreSQL storage (uses .env configuration)
	@echo "$(BLUE)Starting backend with PostgreSQL storage (explicit)...$(RESET)"
	@if [ -n "$(EDGEQUAKE_DEFAULT_LLM_PROVIDER)" ]; then \
		echo "$(GREEN)✓ LLM Provider: $(EDGEQUAKE_DEFAULT_LLM_PROVIDER) ($(EDGEQUAKE_DEFAULT_LLM_MODEL))$(RESET)"; \
	fi
	@	$(LOAD_EFF_DB_URL); \
	$(VISIBLE_MIGRATE_STEP); \
	$(APPLY_LANGFUSE_ENV_EFFECTIVE); \
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
		VLM_PROCESS_ENABLE="$(VLM_PROCESS_ENABLE)" \
		OLLAMA_HOST="http://localhost:11434" \
		OLLAMA_MODEL="gemma4:latest" \
		OLLAMA_EMBEDDING_MODEL="embeddinggemma:latest" \
		OLLAMA_CONTEXT_LENGTH="$${OLLAMA_CONTEXT_LENGTH:-8192}" \
		cargo run

# OODA-03: In-memory storage has been REMOVED for production consistency.
# This target now fails with guidance to use PostgreSQL instead.
backend-memory: ## DEPRECATED - In-memory storage removed, use backend-dev with PostgreSQL
	@echo "$(RED)╔══════════════════════════════════════════════════════════════════╗$(RESET)"
	@echo "$(RED)║    ERROR: In-memory storage has been REMOVED                     ║$(RESET)"
	@echo "$(RED)║                                                                  ║$(RESET)"
	@echo "$(RED)║  The mission directive requires PostgreSQL for all operations.   ║$(RESET)"
	@echo "$(RED)║  Please use one of these alternatives:                           ║$(RESET)"
	@echo "$(RED)║                                                                  ║$(RESET)"
	@echo "$(RED)║    make dev          # Full stack with PostgreSQL                ║$(RESET)"
	@echo "$(RED)║    make backend-dev  # Backend only with PostgreSQL              ║$(RESET)"
	@echo "$(RED)║                                                                  ║$(RESET)"
	@echo "$(RED)╚══════════════════════════════════════════════════════════════════╝$(RESET)"
	@exit 1

backend-bg: sync-dev-ports db-wait ## Run backend in background with PostgreSQL (respects MISTRAL_API_KEY, OPENAI_API_KEY if set)
	@if [ "$(WITH_LANGFUSE)" = "1" ]; then \
		$(MAKE) langfuse-up --no-print-directory; \
	fi
	@if curl -fsS "$(BACKEND_URL)/health" >/dev/null 2>&1; then \
		_llm_code=$$(curl -s -o /dev/null -w '%{http_code}' "$(BACKEND_URL)/api/v1/settings/llm-defaults" 2>/dev/null || echo 000); \
		_langfuse_ok=1; \
		if [ "$(WITH_LANGFUSE)" = "1" ]; then \
			_langfuse_ok=0; \
			if $(LANGFUSE_LOCAL_BACKEND_WIRED); then _langfuse_ok=1; fi; \
		fi; \
		if [ "$$_llm_code" = "200" ] || [ "$$_llm_code" = "401" ]; then \
			if [ "$$_langfuse_ok" = "1" ]; then \
				echo "$(GREEN)✓ Backend already healthy on port $(BACKEND_PORT)$(RESET)"; \
				exit 0; \
			fi; \
			echo "$(YELLOW)⚠ Backend healthy but not wired to local Langfuse ($(LANGFUSE_UI_URL)) — restarting...$(RESET)"; \
		else \
			echo "$(YELLOW)⚠ Backend on port $(BACKEND_PORT) is stale (llm-defaults HTTP $$_llm_code) — restarting...$(RESET)"; \
		fi; \
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
	$(APPLY_LANGFUSE_ENV_EFFECTIVE); \
	$(VISIBLE_MIGRATE_STEP); \
	set -a && [ -f "$(DEV_PORTS_ENV)" ] && . "$(DEV_PORTS_ENV)" && set +a; \
	for BPID in $$(lsof -nP -iTCP:$${BACKEND_PORT:-$(BACKEND_PORT)} -sTCP:LISTEN -t 2>/dev/null || true); do \
		echo "$(YELLOW)→ Freeing port $${BACKEND_PORT:-$(BACKEND_PORT)} (PID $$BPID) before backend-bg start$(RESET)"; \
		kill -9 "$$BPID" 2>/dev/null || true; \
	done; \
	sleep 0.3; \
	_BIN="$(BACKEND_DIR)/target/debug/edgequake"; \
	if [ -x "$$_BIN" ]; then _RUN="exec $$_BIN"; else _RUN="cd $(BACKEND_DIR) && exec cargo run"; fi; \
	if [ -n "$$MISTRAL_API_KEY" ] || [ -n "$(MISTRAL_API_KEY)" ]; then \
		_MISTRAL_KEY="$${MISTRAL_API_KEY:-$(MISTRAL_API_KEY)}"; \
		echo "$(YELLOW)→ MISTRAL_API_KEY detected - using Mistral as default provider$(RESET)"; \
		printf '%s\n' "#!/bin/bash" > /tmp/edgequake-start.sh; \
		printf '%s\n' "set -a && . \"$(DEV_PORTS_ENV)\" && set +a" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export PORT=\"$${BACKEND_PORT:-8090}\"" >> /tmp/edgequake-start.sh; \
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
		printf '%s\n' "export EDGEQUAKE_VISION_MODEL=\"$${EDGEQUAKE_VISION_MODEL:-mistral-small-latest}\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export EDGEQUAKE_LLM_MODEL=\"$${EDGEQUAKE_LLM_MODEL:-mistral-small-latest}\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export MISTRAL_MODEL=\"$${MISTRAL_MODEL:-$${EDGEQUAKE_LLM_MODEL:-mistral-small-latest}}\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export EDGEQUAKE_EMBEDDING_BATCH_SIZE=\"16\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export EDGEQUAKE_ALLOWED_PROVIDERS=\"*\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "$$_RUN" >> /tmp/edgequake-start.sh; \
		chmod +x /tmp/edgequake-start.sh; \
		/bin/bash -lc 'nohup /tmp/edgequake-start.sh > /tmp/edgequake-backend.log 2>&1 < /dev/null & backend_pid=$$!; disown "$$backend_pid"; printf "%s\n" "$$backend_pid" > /tmp/edgequake-backend.pid'; \
	elif [ -n "$(OPENAI_API_KEY)" ]; then \
		echo "$(YELLOW)→ OPENAI_API_KEY detected - using OpenAI as default provider$(RESET)"; \
		printf '%s\n' "#!/bin/bash" > /tmp/edgequake-start.sh; \
		printf '%s\n' "set -a && . \"$(DEV_PORTS_ENV)\" && set +a" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export PORT=\"$${BACKEND_PORT:-8090}\"" >> /tmp/edgequake-start.sh; \
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
		printf '%s\n' "set -a && . \"$(DEV_PORTS_ENV)\" && set +a" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export PORT=\"$${BACKEND_PORT:-8090}\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export DATABASE_URL=\"$$_EFF_DB_URL\"" >> /tmp/edgequake-start.sh; \
		$(BACKEND_STABILITY_EXPORTS) \
		printf '%s\n' "export EDGEQUAKE_DEV_MODE=\"$(DEV_EDGEQUAKE_DEV_MODE)\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export EDGEQUAKE_AUTH_ENABLED=\"$(DEV_AUTH_ENABLED)\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export AUTH_ENABLED=\"$(DEV_AUTH_ENABLED)\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export EDGEQUAKE_LLM_PROVIDER=\"ollama\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export OLLAMA_HOST=\"http://localhost:11434\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export OLLAMA_MODEL=\"gemma4:latest\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export OLLAMA_EMBEDDING_MODEL=\"embeddinggemma:latest\"" >> /tmp/edgequake-start.sh; \
		printf '%s\n' "export OLLAMA_CONTEXT_LENGTH=\"$${OLLAMA_CONTEXT_LENGTH:-8192}\"" >> /tmp/edgequake-start.sh; \
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
	@bash $(FRONTEND_DIR)/scripts/ensure-dev-cache.sh
	@cd $(FRONTEND_DIR) && PORT="$(FRONTEND_PORT)" EDGEQUAKE_API_URL="$(BACKEND_URL)" NEXT_PUBLIC_API_URL="$(BACKEND_URL)" NEXT_PUBLIC_AUTH_ENABLED="$(DEV_AUTH_ENABLED)" NEXT_PUBLIC_DISABLE_DEMO_LOGIN="$(DEV_DISABLE_DEMO_LOGIN)" sh -c '(pnpm run dev 2>/dev/null || bun run dev)'

frontend-bg: sync-dev-ports ## Start frontend development server in background
	@if curl -fsS "$(FRONTEND_URL)" 2>/dev/null | grep -qi 'EdgeQuake'; then \
		echo "$(GREEN)✓ Frontend already reachable on port $(FRONTEND_PORT)$(RESET)"; \
		exit 0; \
	fi
	@echo "$(BLUE)Starting frontend in background...$(RESET)"
	@bash $(FRONTEND_DIR)/scripts/ensure-dev-cache.sh
	@printf '%s\n' "#!/bin/bash" > /tmp/edgequake-frontend-start.sh
	@printf '%s\n' "set -a && . \"$(DEV_PORTS_ENV)\" && set +a" >> /tmp/edgequake-frontend-start.sh
	@printf '%s\n' "cd $(FRONTEND_DIR)" >> /tmp/edgequake-frontend-start.sh
	@printf '%s\n' "if [ -f .env.local.ports ]; then set -a && . ./.env.local.ports && set +a; fi" >> /tmp/edgequake-frontend-start.sh
	@# Never inherit backend PORT from a shared env — Next binds to PORT.
	@printf '%s\n' "export PORT=\"$${FRONTEND_PORT:-3010}\"" >> /tmp/edgequake-frontend-start.sh
	@printf '%s\n' "export EDGEQUAKE_API_URL=\"$${EDGEQUAKE_API_URL:-http://127.0.0.1:8090}\"" >> /tmp/edgequake-frontend-start.sh
	@printf '%s\n' "export NEXT_PUBLIC_API_URL=\"$${NEXT_PUBLIC_API_URL:-$$EDGEQUAKE_API_URL}\"" >> /tmp/edgequake-frontend-start.sh
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
	@# SPEC-083 / X-32: fail closed when tests fail (no echo fallback).
	@cd $(FRONTEND_DIR) && if command -v pnpm >/dev/null 2>&1; then pnpm test; else bun test; fi

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

codegen-vision-prompts: ## SPEC-015V: regenerate FE Vision system-prompt mirror from Rust SSOT
	@echo "$(BLUE)Regenerating Vision prompt mirror from Rust SSOT...$(RESET)"
	@cd $(BACKEND_DIR) && cargo test -p edgequake-api spec015v_write_vision_prompt_codegen \
		--test spec015v_vision_prompt_codegen -- --ignored --nocapture
	@echo "$(GREEN)✓ Prompts: $(FRONTEND_DIR)/src/lib/vision/default-system-prompts.ts$(RESET)"

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


postgres-start: db-start ## Alias for db-start (AGENTS.md / wiki compatibility)

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

postgres-image-build: ## Build and verify edgequake-postgres Docker image (pgvector 0.8.5 + AGE 1.6.0, PG16)
	@echo "$(BLUE)Building edgequake-postgres image (PG16)...$(RESET)"
	@cd $(DOCKER_DIR) && docker build -f Dockerfile.postgres -t edgequake-postgres:pg16 .
	@chmod +x $(DOCKER_DIR)/verify-postgres-extensions.sh
	@EQ_POSTGRES_PROFILE=pg16 bash $(DOCKER_DIR)/verify-postgres-extensions.sh edgequake-postgres:pg16
	@echo "$(GREEN)✓ edgequake-postgres:pg16 ready$(RESET)"

postgres-image-build-pg17: ## Build and verify edgequake-postgres PG17 image (pgvector 0.8.5 + AGE 1.7.0)
	@echo "$(BLUE)Building edgequake-postgres image (PG17 / SPEC-042-C)...$(RESET)"
	@cd $(DOCKER_DIR) && docker build -f Dockerfile.postgres.pg17 -t edgequake-postgres:pg17 .
	@chmod +x $(DOCKER_DIR)/verify-postgres-extensions.sh
	@EQ_POSTGRES_PROFILE=pg17 bash $(DOCKER_DIR)/verify-postgres-extensions.sh edgequake-postgres:pg17
	@echo "$(GREEN)✓ edgequake-postgres:pg17 ready$(RESET)"

postgres-image-build-pg18: ## Build and verify edgequake-postgres PG18 image (pgvector 0.8.5 + AGE 1.8.0) — default dev profile
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

ops17-smoke: ## SPEC-046 OPS-17: pin smoke for pg16/pg17/pg18 (non-flaky; add --battle for Docker)
	@chmod +x specs/046-graphrag-study/e2e/run_ops17_perf_smoke.sh
	@./specs/046-graphrag-study/e2e/run_ops17_perf_smoke.sh

data-access-perf-matrix: ## SPEC-061/062: inviolable DataAccess p95/EXPLAIN/stress on PG16/17/18
	@chmod +x specs/061-multi-version-data-access-perf/e2e/run_data_access_perf_matrix.sh
	@./specs/061-multi-version-data-access-perf/e2e/run_data_access_perf_matrix.sh $${EQ_PERF_PROFILES:-all}

data-access-perf-matrix-release: ## SPEC-062: same matrix with cargo --release
	@chmod +x specs/061-multi-version-data-access-perf/e2e/run_data_access_perf_matrix.sh
	@EDGEQUAKE_PERF_RELEASE=1 ./specs/061-multi-version-data-access-perf/e2e/run_data_access_perf_matrix.sh $${EQ_PERF_PROFILES:-all}

data-access-perf-matrix-prod: ## SPEC-062: release + EDGEQUAKE_PERF_SCALE=prod (50k ANN/FTS, Mix 5k)
	@chmod +x specs/061-multi-version-data-access-perf/e2e/run_data_access_perf_matrix.sh
	@EDGEQUAKE_PERF_RELEASE=1 EDGEQUAKE_PERF_SCALE=prod ./specs/061-multi-version-data-access-perf/e2e/run_data_access_perf_matrix.sh $${EQ_PERF_PROFILES:-all}

data-access-perf-capacity-ladder: ## SPEC-063: L1/L2/L3 ANN soak on pg18 (EDGEQUAKE_CAPACITY_LADDER=L1|L2|L3)
	@chmod +x specs/063-architecture-capacity-assessment/e2e/run_capacity_ladder.sh
	@./specs/063-architecture-capacity-assessment/e2e/run_capacity_ladder.sh $${EQ_PERF_PROFILES:-pg18}

ann-scale-battle: ## SPEC-064: filtered ANN scale battle (halfvec / partial HNSW / GUC) @100k on pg18
	@chmod +x specs/064-filtered-ann-scale-battle/e2e/run_ann_scale_battle.sh
	@./specs/064-filtered-ann-scale-battle/e2e/run_ann_scale_battle.sh $${EQ_PERF_PROFILES:-pg18}

ceiling-proof: ## SPEC-066/067 claim gate (not day-2 sizing; see docs/product-limits.md). EQ_CEILING_STEP=L2|L3|SEEK|G1
	@chmod +x specs/066-ceiling-proof/e2e/run_ceiling_ladder.sh
	@./specs/066-ceiling-proof/e2e/run_ceiling_ladder.sh $${EQ_PERF_PROFILES:-pg18}

recall-pareto: ## SPEC-068 recall×latency claim gate (not day-2 sizing). EQ_PARETO_ROWS_LIST / EQ_PARETO_EF_LIST / EQ_PARETO_REBUILD
	@chmod +x specs/068-recall-quality-scale/e2e/run_recall_pareto.sh
	@./specs/068-recall-quality-scale/e2e/run_recall_pareto.sh $${EQ_PERF_PROFILES:-pg18}

dedicated-midscale: ## SPEC-069 dedicated WS table mid-scale claim gate (not day-2 sizing)
	@chmod +x specs/069-dedicated-midscale/e2e/run_dedicated_midscale.sh
	@./specs/069-dedicated-midscale/e2e/run_dedicated_midscale.sh $${EQ_PERF_PROFILES:-pg18}

wave2-greenfield-env: ## SPEC-071: print Wave-2 turnkey exports (claim gates ≠ day-2 sizing; see docs/product-limits.md)
	@chmod +x scripts/wave2_greenfield_env.sh scripts/wave2_warmup.sh
	@./scripts/wave2_greenfield_env.sh
	@echo "Apply: eval \"\$$(make -s wave2-greenfield-env)\"  or  WAVE2_GREENFIELD=1 make backend-bg" >&2
	@echo "Warmup: ./scripts/wave2_warmup.sh <workspace_uuid>  (or POST /api/v1/admin/ann/warmup)" >&2
	@echo "Claim gates (not day-2 sizing): make ceiling-proof · make recall-pareto · make filtered-recall-gate · make product-limits-check — docs/product-limits.md" >&2

postgres-image-build-pg18-vectorscale: ## SPEC-070: build/verify pg18 + pgvectorscale (DiskANN) opt-in image
	@echo "$(BLUE)Building edgequake-postgres:pg18-vectorscale (SPEC-070)...$(RESET)"
	@cd $(DOCKER_DIR) && docker build -f Dockerfile.postgres.pg18-vectorscale \
		-t edgequake-postgres:pg18-vectorscale .
	@chmod +x $(DOCKER_DIR)/verify-postgres-extensions.sh
	@EQ_POSTGRES_PROFILE=pg18-vectorscale bash $(DOCKER_DIR)/verify-postgres-extensions.sh edgequake-postgres:pg18-vectorscale
	@echo "$(GREEN)✓ edgequake-postgres:pg18-vectorscale ready$(RESET)"

diskann-battle: ## SPEC-070 DiskANN vs HNSW dedicated battle (claim gate; not day-2 sizing)
	@chmod +x specs/070-diskann-study/e2e/run_diskann_battle.sh
	@./specs/070-diskann-study/e2e/run_diskann_battle.sh $${EQ_PERF_PROFILES:-pg18-vectorscale}

diskann-recall-pareto: ## SPEC-072 DiskANN recall×latency Pareto @150k (claim gate; not day-2 sizing)
	@chmod +x specs/072-diskann-recall-pareto/e2e/run_diskann_recall_pareto.sh
	@./specs/072-diskann-recall-pareto/e2e/run_diskann_recall_pareto.sh $${EQ_PERF_PROFILES:-pg18-vectorscale}

diskann-rescore-smoke: ## SPEC-074 DiskANN list=400 + rescore=200 smoke (opt-in recipe; not silent default)
	@chmod +x specs/074-storage-p0-hardening/e2e/run_diskann_rescore_smoke.sh
	@./specs/074-storage-p0-hardening/e2e/run_diskann_rescore_smoke.sh $${EQ_PERF_PROFILES:-pg18-vectorscale}

filtered-recall-gate: ## SPEC-075 filtered recall@20 claim gate (Wave-2 smoke; not day-2 sizing). EQ_FILTERED_RECALL_ROWS
	@chmod +x specs/075-filtered-recall-gates/e2e/run_filtered_recall_gate.sh
	@./specs/075-filtered-recall-gates/e2e/run_filtered_recall_gate.sh $${EQ_PERF_PROFILES:-pg18}

precision-layers-gate: ## SPEC-076 A3 exact-reorder + A4 sparse RRF tip (contracts; EQ_PRECISION_SMOKE=1 for DB)
	@chmod +x specs/076-precision-reorder-rrf/e2e/run_precision_layers_gate.sh
	@./specs/076-precision-reorder-rrf/e2e/run_precision_layers_gate.sh

binary-quantize-bakeoff: ## SPEC-077 binary_quantize+rerank vs Wave-2 (study; not silent default). EQ_BQ_ROWS
	@chmod +x specs/077-binary-quantize-bakeoff/e2e/run_binary_quantize_bakeoff.sh
	@./specs/077-binary-quantize-bakeoff/e2e/run_binary_quantize_bakeoff.sh $${EQ_PERF_PROFILES:-pg18}

filtered-diskann-labels-bakeoff: ## SPEC-078 Filtered-DiskANN labels vs Wave-2 (study; not silent default). EQ_FDL_ROWS
	@chmod +x specs/078-filtered-diskann-labels/e2e/run_filtered_diskann_labels.sh
	@./specs/078-filtered-diskann-labels/e2e/run_filtered_diskann_labels.sh $${EQ_PERF_PROFILES:-pg18-vectorscale}

midscale-quantize-labels: ## SPEC-079 mid-scale B2+A6 @50k/100k (study archive; not silent default)
	@chmod +x specs/079-midscale-quantize-labels/e2e/run_midscale_quantize_labels.sh
	@./specs/079-midscale-quantize-labels/e2e/run_midscale_quantize_labels.sh

tiny-slice-exact-gate: ## SPEC-080 B3 tiny-slice exact (skip Wave-2 planner bias below EDGEQUAKE_ANN_EXACT_MAX_ROWS)
	@chmod +x specs/080-tiny-slice-exact/e2e/run_tiny_slice_exact_gate.sh
	@./specs/080-tiny-slice-exact/e2e/run_tiny_slice_exact_gate.sh

serving-view-check: ## SPEC-081 C5 serving-view dual-SSOT (migrate + contract)
	@chmod +x specs/081-serving-view-dual-ssot/e2e/run_serving_view_check.sh
	@./specs/081-serving-view-dual-ssot/e2e/run_serving_view_check.sh

push-scale-ladder: ## SPEC-082 A6@150/250 + Wave-2@150 spot + DiskANN@250 full-gate (raise floors only if green)
	@chmod +x specs/082-push-scale-floors/e2e/run_push_scale_ladder.sh
	@./specs/082-push-scale-floors/e2e/run_push_scale_ladder.sh

product-limits-check: ## SPEC-065–082 honesty gate for docs/product-limits.md vs FAQ/envelope
	@python3 scripts/product_limits_check.py

compare-eq-perf: ## SPEC-062: cross-major 2× gate on archived JSONL (or ARGS=a.jsonl b.jsonl)
	@python3 scripts/compare_eq_perf_jsonl.py --cross-major \
		specs/061-multi-version-data-access-perf/e2e/artifacts/eq-perf-pg16.jsonl \
		specs/061-multi-version-data-access-perf/e2e/artifacts/eq-perf-pg17.jsonl \
		specs/061-multi-version-data-access-perf/e2e/artifacts/eq-perf-pg18.jsonl

spec046-acc: ## SPEC-046 science ACC gate + JSON artifact (deterministic; no API key)
	@chmod +x specs/046-graphrag-study/e2e/run_spec046_acc.sh
	@./specs/046-graphrag-study/e2e/run_spec046_acc.sh

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

spec045-battle-test-all: ## SPEC-045 ingestion reliability battle test (edge cases + health SQL)
	@chmod +x specs/045-fix-ingestion-errors/e2e/run_ingestion_health_proof.sh
	@./specs/045-fix-ingestion-errors/e2e/run_ingestion_health_proof.sh

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

# ============================================================================
# SPEC-124 — optional local Langfuse v4 (isolated Compose project)
# Does not start with make dev. make stop does not tear this down.
# ============================================================================

langfuse-up: ## Start local Langfuse v4 (UI http://localhost:3310)
	@echo "$(BOLD)$(BLUE)Starting local Langfuse v4$(RESET)"
	@if [ "$(LANGFUSE_PULL)" = "1" ]; then \
		echo "$(YELLOW)→ Pulling langfuse:4 images$(RESET)"; \
		cd $(DOCKER_DIR) && LANGFUSE_PORT="$(LANGFUSE_PORT)" NEXTAUTH_URL="$(LANGFUSE_UI_URL)" \
			docker compose -f docker-compose.langfuse.yml --project-name $(LANGFUSE_COMPOSE_PROJECT) pull; \
	fi
	@cd $(DOCKER_DIR) && LANGFUSE_PORT="$(LANGFUSE_PORT)" NEXTAUTH_URL="$(LANGFUSE_UI_URL)" \
		docker compose -f docker-compose.langfuse.yml --project-name $(LANGFUSE_COMPOSE_PROJECT) up -d
	@echo "$(YELLOW)→ Waiting for Langfuse health/ready (up to 180s)...$(RESET)"
	@ready=0; \
	for i in $$(seq 1 90); do \
		if curl -sf "$(LANGFUSE_UI_URL)/api/public/health" >/dev/null 2>&1 \
			&& curl -sf "$(LANGFUSE_UI_URL)/api/public/ready" >/dev/null 2>&1; then \
			ready=1; break; \
		fi; \
		printf "."; sleep 2; \
	done; \
	echo ""; \
	if [ "$$ready" != "1" ]; then \
		echo "$(RED)✗ Langfuse did not become ready at $(LANGFUSE_UI_URL)$(RESET)"; \
		cd $(DOCKER_DIR) && docker compose -f docker-compose.langfuse.yml --project-name $(LANGFUSE_COMPOSE_PROJECT) ps; \
		exit 1; \
	fi
	@echo "$(GREEN)✓ Langfuse UI $(LANGFUSE_UI_URL)$(RESET)"
	@echo "  One-command stack: $(GREEN)make dev-langfuse$(RESET) / $(GREEN)make dev-bg-langfuse$(RESET) (injects init keys; no .env edit)"
	@echo "  Or point repo-root .env at local keys, then restart the backend:"
	@echo "    LANGFUSE_PUBLIC_KEY=$(LANGFUSE_LOCAL_PK)"
	@echo "    LANGFUSE_SECRET_KEY=$(LANGFUSE_LOCAL_SK)"
	@echo "    LANGFUSE_BASE_URL=$(LANGFUSE_UI_URL)"
	@echo "    LANGFUSE_PROJECT_ID=$(LANGFUSE_LOCAL_PROJECT_ID)"
	@echo "  Login: dev@example.com / edgequake-local-dev"

langfuse-down: ## Stop local Langfuse (volumes kept)
	@echo "$(YELLOW)Stopping local Langfuse (volumes kept)$(RESET)"
	@cd $(DOCKER_DIR) && docker compose -f docker-compose.langfuse.yml --project-name $(LANGFUSE_COMPOSE_PROJECT) down
	@echo "$(GREEN)✓ Langfuse stopped$(RESET)"

langfuse-logs: ## Tail local Langfuse logs
	@cd $(DOCKER_DIR) && docker compose -f docker-compose.langfuse.yml --project-name $(LANGFUSE_COMPOSE_PROJECT) logs -f

langfuse-status: ## Show local Langfuse container + health
	@cd $(DOCKER_DIR) && docker compose -f docker-compose.langfuse.yml --project-name $(LANGFUSE_COMPOSE_PROJECT) ps
	@if curl -sf "$(LANGFUSE_UI_URL)/api/public/health" >/dev/null 2>&1; then \
		echo "$(GREEN)✓ health $(LANGFUSE_UI_URL)/api/public/health$(RESET)"; \
	else \
		echo "$(RED)Not healthy at $(LANGFUSE_UI_URL)$(RESET)"; \
	fi

langfuse-smoke: ## Health + GET /api/public/projects (headless init keys)
	@chmod +x $(ROOT_DIR)/scripts/langfuse_local_smoke.sh
	@LANGFUSE_PORT="$(LANGFUSE_PORT)" LANGFUSE_UI_URL="$(LANGFUSE_UI_URL)" \
		LANGFUSE_LOCAL_PK="$(LANGFUSE_LOCAL_PK)" LANGFUSE_LOCAL_SK="$(LANGFUSE_LOCAL_SK)" \
		LANGFUSE_LOCAL_PROJECT_ID="$(LANGFUSE_LOCAL_PROJECT_ID)" \
		$(ROOT_DIR)/scripts/langfuse_local_smoke.sh

langfuse-reset: ## Delete local Langfuse volumes (CONFIRM=yes required)
	@if [ "$(CONFIRM)" != "yes" ]; then \
		echo "$(RED)Refusing to wipe Langfuse volumes.$(RESET)"; \
		echo "  This removes ClickHouse/Postgres/MinIO/Redis data for project $(LANGFUSE_COMPOSE_PROJECT)."; \
		echo "  Re-run: $(GREEN)make langfuse-reset CONFIRM=yes$(RESET)"; \
		exit 1; \
	fi
	@echo "$(YELLOW)→ Removing Langfuse containers and volumes$(RESET)"
	@cd $(DOCKER_DIR) && docker compose -f docker-compose.langfuse.yml --project-name $(LANGFUSE_COMPOSE_PROJECT) down -v
	@echo "$(GREEN)✓ Langfuse volumes removed$(RESET)"

langfuse-sync-prices: ## Push models.toml pricing into Langfuse (fixes $0.00 cost)
	@echo "$(YELLOW)→ Syncing EdgeQuake model prices into Langfuse...$(RESET)"
	@# LAW-124-12: EdgeQuake never emits cost attrs; Langfuse prices from its catalogue.
	@set -a; [ -f "$(ROOT_DIR)/.env" ] && . "$(ROOT_DIR)/.env"; set +a; \
	LANGFUSE_BASE_URL="$${LANGFUSE_BASE_URL:-$(LANGFUSE_UI_URL)}" \
	LANGFUSE_PUBLIC_KEY="$${LANGFUSE_PUBLIC_KEY:-$(LANGFUSE_LOCAL_PK)}" \
	LANGFUSE_SECRET_KEY="$${LANGFUSE_SECRET_KEY:-$(LANGFUSE_LOCAL_SK)}" \
	python3 $(ROOT_DIR)/scripts/langfuse_sync_model_prices.py $(if $(DRY_RUN),--dry-run) $(if $(FORCE),--force)

langfuse-3.1-up: ## Start Langfuse 3.1.1 (UI http://localhost:3320) for ingestion-fallback E2E
	@echo "$(BOLD)$(BLUE)Starting Langfuse 3.1.1$(RESET)"
	@cd $(DOCKER_DIR) && LANGFUSE_311_PORT="$(LANGFUSE_311_PORT)" NEXTAUTH_URL="$(LANGFUSE_311_UI_URL)" \
		docker compose -f docker-compose.langfuse-3.1.yml --project-name $(LANGFUSE_311_COMPOSE_PROJECT) up -d
	@echo "$(YELLOW)→ Waiting for Langfuse 3.1.1 health (up to 180s)...$(RESET)"
	@ready=0; \
	for i in $$(seq 1 90); do \
		if curl -sf "$(LANGFUSE_311_UI_URL)/api/public/health" >/dev/null 2>&1 \
			&& curl -sf "$(LANGFUSE_311_UI_URL)/api/public/ready" >/dev/null 2>&1; then \
			ready=1; break; \
		fi; \
		printf "."; sleep 2; \
	done; \
	echo ""; \
	if [ "$$ready" != "1" ]; then \
		echo "$(RED)✗ Langfuse 3.1.1 did not become ready at $(LANGFUSE_311_UI_URL)$(RESET)"; \
		cd $(DOCKER_DIR) && docker compose -f docker-compose.langfuse-3.1.yml --project-name $(LANGFUSE_311_COMPOSE_PROJECT) ps; \
		exit 1; \
	fi
	@echo "$(GREEN)✓ Langfuse 3.1.1 ready at $(LANGFUSE_311_UI_URL)$(RESET)"
	@echo "$(YELLOW)→ Restarting worker after web Prisma migrations (3.1 race)$(RESET)"
	@cd $(DOCKER_DIR) && docker compose -f docker-compose.langfuse-3.1.yml --project-name $(LANGFUSE_311_COMPOSE_PROJECT) restart langfuse-worker
	@sleep 5
	@echo "$(GREEN)✓ Langfuse 3.1.1 worker restarted$(RESET)"

langfuse-3.1-down: ## Stop Langfuse 3.1.1 stack (volumes kept)
	@cd $(DOCKER_DIR) && docker compose -f docker-compose.langfuse-3.1.yml --project-name $(LANGFUSE_311_COMPOSE_PROJECT) down

langfuse-3.1-reset: ## Delete Langfuse 3.1.1 volumes (CONFIRM=yes required; does not touch v4)
	@if [ "$(CONFIRM)" != "yes" ]; then \
		echo "$(RED)Refusing to wipe Langfuse 3.1.1 volumes.$(RESET)"; \
		echo "  This removes ClickHouse/Postgres/MinIO/Redis data for project $(LANGFUSE_311_COMPOSE_PROJECT)."; \
		echo "  Re-run: $(GREEN)make langfuse-3.1-reset CONFIRM=yes$(RESET)"; \
		exit 1; \
	fi
	@echo "$(YELLOW)→ Removing Langfuse 3.1.1 containers and volumes$(RESET)"
	@cd $(DOCKER_DIR) && docker compose -f docker-compose.langfuse-3.1.yml --project-name $(LANGFUSE_311_COMPOSE_PROJECT) down -v
	@echo "$(GREEN)✓ Langfuse 3.1.1 volumes removed$(RESET)"

spec124-langfuse-3.1-e2e: ## Unfakable ingestion-fallback E2E vs Langfuse 3.1.1 (starts 3.1.1)
	@$(MAKE) langfuse-3.1-up --no-print-directory
	@chmod +x $(ROOT_DIR)/scripts/spec124_langfuse_3_1_ingestion_e2e.sh
	@LANGFUSE_311_PORT="$(LANGFUSE_311_PORT)" LANGFUSE_311_UI_URL="$(LANGFUSE_311_UI_URL)" \
		LANGFUSE_311_PK="$(LANGFUSE_311_PK)" LANGFUSE_311_SK="$(LANGFUSE_311_SK)" \
		LANGFUSE_311_PROJECT_ID="$(LANGFUSE_311_PROJECT_ID)" \
		$(ROOT_DIR)/scripts/spec124_langfuse_3_1_ingestion_e2e.sh

langfuse-3.22-up: ## Start Langfuse 3.22.0 (UI http://localhost:3330) for OTLP E2E
	@echo "$(BOLD)$(BLUE)Starting Langfuse 3.22.0$(RESET)"
	@cd $(DOCKER_DIR) && LANGFUSE_322_PORT="$(LANGFUSE_322_PORT)" NEXTAUTH_URL="$(LANGFUSE_322_UI_URL)" \
		docker compose -f docker-compose.langfuse-3.22.yml --project-name $(LANGFUSE_322_COMPOSE_PROJECT) up -d
	@echo "$(YELLOW)→ Waiting for Langfuse 3.22.0 health (up to 180s)...$(RESET)"
	@ready=0; \
	for i in $$(seq 1 90); do \
		if curl -sf "$(LANGFUSE_322_UI_URL)/api/public/health" >/dev/null 2>&1 \
			&& curl -sf "$(LANGFUSE_322_UI_URL)/api/public/ready" >/dev/null 2>&1; then \
			ready=1; break; \
		fi; \
		printf "."; sleep 2; \
	done; \
	echo ""; \
	if [ "$$ready" != "1" ]; then \
		echo "$(RED)✗ Langfuse 3.22.0 did not become ready at $(LANGFUSE_322_UI_URL)$(RESET)"; \
		cd $(DOCKER_DIR) && docker compose -f docker-compose.langfuse-3.22.yml --project-name $(LANGFUSE_322_COMPOSE_PROJECT) ps; \
		exit 1; \
	fi
	@echo "$(GREEN)✓ Langfuse 3.22.0 ready at $(LANGFUSE_322_UI_URL)$(RESET)"
	@echo "$(YELLOW)→ Restarting worker after web Prisma migrations$(RESET)"
	@cd $(DOCKER_DIR) && docker compose -f docker-compose.langfuse-3.22.yml --project-name $(LANGFUSE_322_COMPOSE_PROJECT) restart langfuse-worker
	@sleep 5
	@echo "$(GREEN)✓ Langfuse 3.22.0 worker restarted$(RESET)"

langfuse-3.22-down: ## Stop Langfuse 3.22.0 stack (volumes kept)
	@cd $(DOCKER_DIR) && docker compose -f docker-compose.langfuse-3.22.yml --project-name $(LANGFUSE_322_COMPOSE_PROJECT) down

langfuse-3.22-reset: ## Delete Langfuse 3.22.0 volumes (CONFIRM=yes required; does not touch 3.1/v4)
	@if [ "$(CONFIRM)" != "yes" ]; then \
		echo "$(RED)Refusing to wipe Langfuse 3.22.0 volumes.$(RESET)"; \
		echo "  This removes ClickHouse/Postgres/MinIO/Redis data for project $(LANGFUSE_322_COMPOSE_PROJECT)."; \
		echo "  Re-run: $(GREEN)make langfuse-3.22-reset CONFIRM=yes$(RESET)"; \
		exit 1; \
	fi
	@echo "$(YELLOW)→ Removing Langfuse 3.22.0 containers and volumes$(RESET)"
	@cd $(DOCKER_DIR) && docker compose -f docker-compose.langfuse-3.22.yml --project-name $(LANGFUSE_322_COMPOSE_PROJECT) down -v
	@echo "$(GREEN)✓ Langfuse 3.22.0 volumes removed$(RESET)"

spec124-langfuse-3.22-e2e: ## Unfakable OTLP route+probe vs Langfuse 3.22.0 (starts 3.22.0)
	@$(MAKE) langfuse-3.22-up --no-print-directory
	@chmod +x $(ROOT_DIR)/scripts/spec124_langfuse_3_22_otlp_e2e.sh
	@LANGFUSE_322_PORT="$(LANGFUSE_322_PORT)" LANGFUSE_322_UI_URL="$(LANGFUSE_322_UI_URL)" \
		LANGFUSE_322_PK="$(LANGFUSE_322_PK)" LANGFUSE_322_SK="$(LANGFUSE_322_SK)" \
		LANGFUSE_322_PROJECT_ID="$(LANGFUSE_322_PROJECT_ID)" \
		$(ROOT_DIR)/scripts/spec124_langfuse_3_22_otlp_e2e.sh

langfuse-3.225-up: ## Start Langfuse 3.225.5 (UI http://localhost:3340) for OTLP persist E2E
	@echo "$(BOLD)$(BLUE)Starting Langfuse 3.225.5$(RESET)"
	@cd $(DOCKER_DIR) && LANGFUSE_3225_PORT="$(LANGFUSE_3225_PORT)" NEXTAUTH_URL="$(LANGFUSE_3225_UI_URL)" \
		docker compose -f docker-compose.langfuse-3.225.yml --project-name $(LANGFUSE_3225_COMPOSE_PROJECT) up -d
	@echo "$(YELLOW)→ Waiting for Langfuse 3.225.5 health (up to 180s)...$(RESET)"
	@ready=0; \
	for i in $$(seq 1 90); do \
		if curl -sf "$(LANGFUSE_3225_UI_URL)/api/public/health" >/dev/null 2>&1 \
			&& curl -sf "$(LANGFUSE_3225_UI_URL)/api/public/ready" >/dev/null 2>&1; then \
			ready=1; break; \
		fi; \
		printf "."; sleep 2; \
	done; \
	echo ""; \
	if [ "$$ready" != "1" ]; then \
		echo "$(RED)✗ Langfuse 3.225.5 did not become ready at $(LANGFUSE_3225_UI_URL)$(RESET)"; \
		cd $(DOCKER_DIR) && docker compose -f docker-compose.langfuse-3.225.yml --project-name $(LANGFUSE_3225_COMPOSE_PROJECT) ps; \
		exit 1; \
	fi
	@echo "$(GREEN)✓ Langfuse 3.225.5 ready at $(LANGFUSE_3225_UI_URL)$(RESET)"
	@echo "$(YELLOW)→ Restarting worker after web Prisma migrations$(RESET)"
	@cd $(DOCKER_DIR) && docker compose -f docker-compose.langfuse-3.225.yml --project-name $(LANGFUSE_3225_COMPOSE_PROJECT) restart langfuse-worker
	@sleep 5
	@echo "$(GREEN)✓ Langfuse 3.225.5 worker restarted$(RESET)"

langfuse-3.225-down: ## Stop Langfuse 3.225.5 stack (volumes kept)
	@cd $(DOCKER_DIR) && docker compose -f docker-compose.langfuse-3.225.yml --project-name $(LANGFUSE_3225_COMPOSE_PROJECT) down

langfuse-3.225-reset: ## Delete Langfuse 3.225.5 volumes (CONFIRM=yes required)
	@if [ "$(CONFIRM)" != "yes" ]; then \
		echo "$(RED)Refusing to wipe Langfuse 3.225.5 volumes.$(RESET)"; \
		echo "  This removes ClickHouse/Postgres/MinIO/Redis data for project $(LANGFUSE_3225_COMPOSE_PROJECT)."; \
		echo "  Re-run: $(GREEN)make langfuse-3.225-reset CONFIRM=yes$(RESET)"; \
		exit 1; \
	fi
	@echo "$(YELLOW)→ Removing Langfuse 3.225.5 containers and volumes$(RESET)"
	@cd $(DOCKER_DIR) && docker compose -f docker-compose.langfuse-3.225.yml --project-name $(LANGFUSE_3225_COMPOSE_PROJECT) down -v
	@echo "$(GREEN)✓ Langfuse 3.225.5 volumes removed$(RESET)"

spec124-langfuse-3.225-e2e: ## Unfakable OTLP persist E2E vs Langfuse 3.225.5 (starts 3.225.5)
	@$(MAKE) langfuse-3.225-up --no-print-directory
	@chmod +x $(ROOT_DIR)/scripts/spec124_langfuse_3_225_otlp_e2e.sh
	@LANGFUSE_3225_PORT="$(LANGFUSE_3225_PORT)" LANGFUSE_3225_UI_URL="$(LANGFUSE_3225_UI_URL)" \
		LANGFUSE_3225_PK="$(LANGFUSE_3225_PK)" LANGFUSE_3225_SK="$(LANGFUSE_3225_SK)" \
		LANGFUSE_3225_PROJECT_ID="$(LANGFUSE_3225_PROJECT_ID)" \
		$(ROOT_DIR)/scripts/spec124_langfuse_3_225_otlp_e2e.sh

spec124-langfuse-cloud-e2e: ## Unfakable OTLP persist E2E vs current Langfuse Cloud (keys from .env)
	@chmod +x $(ROOT_DIR)/scripts/spec124_langfuse_cloud_e2e.sh
	@$(ROOT_DIR)/scripts/spec124_langfuse_cloud_e2e.sh

spec124-langfuse-matrix: ## Unfakable 3.1.1 ingestion + 3.22.0 OTLP route + 3.225.5 persist + Cloud
	@$(MAKE) spec124-langfuse-3.1-e2e --no-print-directory
	@$(MAKE) spec124-langfuse-3.22-e2e --no-print-directory
	@$(MAKE) spec124-langfuse-3.225-e2e --no-print-directory
	@$(MAKE) spec124-langfuse-cloud-e2e --no-print-directory
	@echo "$(GREEN)✓ spec124-langfuse-matrix passed (3.1.1 + 3.22.0 + 3.225.5 + Cloud)$(RESET)"

spec124-langfuse-e2e: ## One-command live Settings + sessions E2E vs local Langfuse (starts stack)
	@$(MAKE) langfuse-up --no-print-directory
	@$(MAKE) dev-bg --no-print-directory WITH_LANGFUSE=1
	@$(MAKE) langfuse-smoke --no-print-directory
	@chmod +x $(ROOT_DIR)/scripts/spec124_langfuse_local_e2e.sh
	@$(APPLY_LANGFUSE_LOCAL_ENV); \
	LANGFUSE_PORT="$(LANGFUSE_PORT)" LANGFUSE_UI_URL="$(LANGFUSE_UI_URL)" \
		LANGFUSE_LOCAL_PK="$(LANGFUSE_LOCAL_PK)" LANGFUSE_LOCAL_SK="$(LANGFUSE_LOCAL_SK)" \
		LANGFUSE_LOCAL_PROJECT_ID="$(LANGFUSE_LOCAL_PROJECT_ID)" \
		BACKEND_URL="$(BACKEND_URL)" FRONTEND_URL="$(FRONTEND_URL)" \
		$(ROOT_DIR)/scripts/spec124_langfuse_local_e2e.sh

# ── SPEC-138 Kubernetes (EdgeQuake + in-cluster Langfuse) ─────────────────────
K8S_DIR := $(ROOT_DIR)/deploy/kubernetes
K8S_SCRIPTS := $(K8S_DIR)/scripts
K8S_HELM := $(K8S_DIR)/helm

.PHONY: k8s-prereqs k8s-kind-up k8s-kind-down k8s-install k8s-uninstall k8s-status spec138-kubernetes-proof spec138-helm-template

k8s-prereqs: ## SPEC-138: cert-manager + ClickHouse operator + nginx ingress
	@chmod +x $(K8S_SCRIPTS)/k8s_prereqs.sh
	@$(K8S_SCRIPTS)/k8s_prereqs.sh

k8s-kind-up: ## SPEC-138: create kind cluster (edgequake-spec138)
	@chmod +x $(K8S_SCRIPTS)/k8s_kind_up.sh
	@$(K8S_SCRIPTS)/k8s_kind_up.sh

k8s-kind-down: ## SPEC-138: delete kind cluster
	@chmod +x $(K8S_SCRIPTS)/k8s_kind_down.sh
	@$(K8S_SCRIPTS)/k8s_kind_down.sh

k8s-install: k8s-prereqs ## SPEC-138: install Langfuse + EdgeQuake on current cluster
	@chmod +x $(K8S_SCRIPTS)/k8s_install_stack.sh $(K8S_SCRIPTS)/k8s_wait_ready.sh
	@$(K8S_SCRIPTS)/k8s_install_stack.sh
	@$(K8S_SCRIPTS)/k8s_wait_ready.sh

k8s-uninstall: ## SPEC-138: uninstall Langfuse + EdgeQuake releases
	@chmod +x $(K8S_SCRIPTS)/k8s_uninstall_stack.sh
	@$(K8S_SCRIPTS)/k8s_uninstall_stack.sh

k8s-status: ## SPEC-138: show EdgeQuake + Langfuse pod status
	@chmod +x $(K8S_SCRIPTS)/k8s_context.sh
	@. $(K8S_SCRIPTS)/k8s_context.sh; \
	kubectl --context "$$KUBECTL_CONTEXT" get pods -n edgequake 2>/dev/null || echo "namespace edgequake not found"; \
	kubectl --context "$$KUBECTL_CONTEXT" get pods -n langfuse 2>/dev/null || echo "namespace langfuse not found"

spec138-helm-template: ## SPEC-138: render Helm charts (no cluster required)
	@helm dependency build $(K8S_HELM)/edgequake-stack
	@helm template edgequake-stack $(K8S_HELM)/edgequake-stack \
		-f $(K8S_HELM)/edgequake-stack/values-kind.yaml \
		--namespace edgequake > /dev/null
	@echo "$(GREEN)✓ Helm templates render OK$(RESET)"

spec138-kubernetes-proof: ## SPEC-138: full kind E2E — traces to Langfuse (requires kind, ~16GB RAM)
	@chmod +x $(K8S_SCRIPTS)/*.sh $(ROOT_DIR)/scripts/langfuse_e2e_common.sh
	@$(K8S_SCRIPTS)/spec138_kubernetes_e2e.sh

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

.PHONY: stack stack-down stack-logs stack-status stack-restart stack-pull \
	spec091-upgrade-soak spec091-gates \
	spec93-migration-assessment spec93-migration-assessment-pg16 \
	spec93-migration-assessment-pg17 spec93-migration-assessment-pg18 \
	spec137-migrate-025-026-proof spec139-migrate-engine-proof

# SPEC-091: v0.22.0 GHCR → HEAD smoke soak (tiny corpus; migrations 106–141 + confirm-drop).
# Formal realism matrix: make spec93-migration-assessment (see specs/93-migration-assessment/).
# See docs/operations/spec091-upgrade-from-v0.22.0.md
spec091-upgrade-soak: ## SPEC-091: smoke upgrade soak from published v0.22.0 (tiny multi-tenant)
	@chmod +x $(ROOT_DIR)/scripts/spec091_upgrade_soak.sh
	@SPEC93_PROFILE=smoke $(ROOT_DIR)/scripts/spec091_upgrade_soak.sh

# SPEC-93: realism corpus (5×3×40) × PG16/17/18 matrix from v0.22.0 → HEAD.
# Reports: specs/93-migration-assessment/reports/
spec93-migration-assessment: ## SPEC-93: v0.22.0→HEAD realism soak matrix (pg16+pg17+pg18)
	@chmod +x $(ROOT_DIR)/scripts/spec93_migration_assessment.sh
	@$(ROOT_DIR)/scripts/spec93_migration_assessment.sh

spec93-migration-assessment-pg16: ## SPEC-93: realism soak on PG16 only
	@chmod +x $(ROOT_DIR)/scripts/spec93_migration_assessment.sh
	@SPEC93_PG_PROFILE=pg16 $(ROOT_DIR)/scripts/spec93_migration_assessment.sh

spec93-migration-assessment-pg17: ## SPEC-93: realism soak on PG17 only
	@chmod +x $(ROOT_DIR)/scripts/spec93_migration_assessment.sh
	@SPEC93_PG_PROFILE=pg17 $(ROOT_DIR)/scripts/spec93_migration_assessment.sh

spec93-migration-assessment-pg18: ## SPEC-93: realism soak on PG18 only
	@chmod +x $(ROOT_DIR)/scripts/spec93_migration_assessment.sh
	@SPEC93_PG_PROFILE=pg18 $(ROOT_DIR)/scripts/spec93_migration_assessment.sh

# SPEC-091 IW0–IW5 local gate (mirrors .github/workflows/spec091-data-layer.yml::spec091-data-layer).
# Requires DATABASE_URL pointing at a Postgres with pgvector + AGE (make postgres-start).
# Soft-skips are disabled here (EDGEQUAKE_REQUIRE_POSTGRES_TESTS=1) so a missing DB fails loud.
spec103-llm-cache-proof: ## SPEC-103: unit + contract LLM cache proof (MemoryKV L2)
	@echo "$(BOLD)$(BLUE)SPEC-103 LLM cache proof$(RESET)"
	@cd $(ROOT_DIR)/edgequake && \
	  cargo test -p edgequake-query --lib cache::llm_response_cache -- --nocapture && \
	  cargo test -p edgequake-query --test contract_spec103_llm_cache -- --nocapture
	@echo "$(GREEN)SPEC-103 contract proof OK$(RESET) (optional postgres: cargo test -p edgequake-query --features postgres --test e2e_spec103_llm_cache_persist -- --ignored)"

spec109-reasoning-effort-proof: ## SPEC-109: reasoning effort clamp + API contract proof
	@echo "$(BOLD)$(BLUE)SPEC-109 reasoning effort proof$(RESET)"
	@if [ -d "$(ROOT_DIR)/../edgequake-llm" ]; then \
	  cd $(ROOT_DIR)/../edgequake-llm && \
	    cargo test reasoning_capabilities --lib -- --nocapture && \
	    cargo test test_reasoning_effort --lib -- --nocapture && \
	    cargo test test_apply_reasoning_effort --lib -- --nocapture; \
	else \
	  echo "$(YELLOW)skip sibling edgequake-llm unit tests (use crates.io 0.10.8)$(RESET)"; \
	fi
	@cd $(ROOT_DIR)/edgequake && \
	  cargo test -p edgequake-core --lib llm_roles:: -- --nocapture && \
	  cargo test -p edgequake-pipeline --lib completion_options:: -- --nocapture && \
	  cargo test -p edgequake-query --lib cache::llm_response_cache::tests::hash_query_prompt_includes_effort -- --nocapture && \
	  cargo test -p edgequake-api --test contract_spec109_reasoning_effort -- --nocapture
	@echo "$(GREEN)SPEC-109 contract proof OK$(RESET)"

spec110-migrate-118-proof: ## SPEC-110: wsdoc/injection ON CONFLICT dedup + checksum repair proof
	@echo "$(BOLD)$(BLUE)SPEC-110 migrate 118 proof$(RESET)"
	@chmod +x $(ROOT_DIR)/scripts/spec110_migrate_118_proof.sh
	@$(ROOT_DIR)/scripts/spec110_migrate_118_proof.sh
	@echo "$(GREEN)SPEC-110 proof OK$(RESET) — see specs/110-migration-issue/measurements/"

spec137-migrate-025-026-proof: ## SPEC-137: 0.25→0.26 migrate consent alias + abort class + 149
	@echo "$(BOLD)$(BLUE)SPEC-137 migrate 0.25→0.26 proof$(RESET)"
	@chmod +x $(ROOT_DIR)/scripts/spec137_migrate_025_026_proof.sh
	@$(ROOT_DIR)/scripts/spec137_migrate_025_026_proof.sh
	@echo "$(GREEN)SPEC-137 proof OK$(RESET) — see specs/137-issue-migration-25-to-26/measurements/"

spec139-migrate-engine-proof: ## SPEC-139: mid-cutover engine (iw2 21000, W3 coverage-sum, remainder)
	@echo "$(BOLD)$(BLUE)SPEC-139 migrate engine proof$(RESET)"
	@chmod +x $(ROOT_DIR)/scripts/spec139_migrate_engine_proof.sh
	@$(ROOT_DIR)/scripts/spec139_migrate_engine_proof.sh
	@echo "$(GREEN)SPEC-139 proof OK$(RESET) — see specs/139-issue-migration/measurements/"

spec109-e2e: dev-bg ## SPEC-109 reasoning effort UI E2E + measurement screenshots
	@echo "$(BLUE)SPEC-109 E2E → frontend $(FRONTEND_URL) backend $(BACKEND_URL)$(RESET)"
	@i=0; while [ $$i -lt 60 ]; do \
		if curl -sf "$(BACKEND_URL)/health" >/dev/null \
			&& curl -sf "$(BACKEND_URL)/api/v1/tenants" >/dev/null \
			&& curl -sf "$(FRONTEND_URL)/" 2>/dev/null | grep -qi 'EdgeQuake'; then \
			break; \
		fi; \
		i=$$((i+1)); sleep 2; \
	done
	@curl -sf "$(BACKEND_URL)/health" >/dev/null || { \
		echo "$(RED)✗ EdgeQuake backend not healthy at $(BACKEND_URL)$(RESET)"; exit 1; \
	}
	@curl -sf "$(BACKEND_URL)/api/v1/tenants" >/dev/null || { \
		echo "$(RED)✗ Tenants API not ready at $(BACKEND_URL)/api/v1/tenants$(RESET)"; exit 1; \
	}
	@curl -sf "$(FRONTEND_URL)/" 2>/dev/null | grep -qi 'EdgeQuake' || { \
		echo "$(RED)✗ Frontend not EdgeQuake at $(FRONTEND_URL)$(RESET)"; exit 1; \
	}
	@# Extra settle: Next.js first compile can still be blank for a few seconds
	@sleep 8
	@mkdir -p $(ROOT_DIR)/specs/109-configurable-reasoning-effort/measurements/e2e/screenshots
	@cd $(FRONTEND_DIR) && EQ_BACKEND_URL="$(BACKEND_URL)" E2E_BACKEND_URL="$(BACKEND_URL)" \
		EDGEQUAKE_API_URL="$(BACKEND_URL)" NEXT_PUBLIC_API_URL="$(BACKEND_URL)" \
		E2E_LIVE_STACK=1 PLAYWRIGHT_SKIP_STACK_CHECK=1 PLAYWRIGHT_BASE_URL="$(FRONTEND_URL)" \
		pnpm exec playwright test e2e/spec109-reasoning-effort.spec.ts --project=chromium --reporter=line
	@echo "$(GREEN)SPEC-109 E2E screenshots → specs/109-configurable-reasoning-effort/measurements/e2e/screenshots/$(RESET)"

spec091-gates: ## SPEC-091: run wired data-layer e2e + contracts (serial)
	@echo "$(BOLD)$(BLUE)SPEC-091 data-layer gates$(RESET) (serial; needs DATABASE_URL)"
	@test -f $(ROOT_DIR)/specs/091-simplify-data-layer/measurements/rm4-explain-hot-paths.md || \
	  (echo "$(RED)missing RM4 EXPLAIN artifact: measurements/rm4-explain-hot-paths.md$(RESET)" && exit 1)
	@$(LOAD_EFF_DB_URL); \
	export DATABASE_URL="$$_EFF_DB_URL"; \
	export EDGEQUAKE_REQUIRE_POSTGRES_TESTS=1; \
	echo "DATABASE_URL=$$DATABASE_URL"; \
	cd $(ROOT_DIR)/edgequake && \
	  cargo test -p edgequake-storage --features postgres --test e2e_spec091_wave_d -- --test-threads=1 && \
	  cargo test -p edgequake-storage --features postgres --test e2e_spec091_console -- --test-threads=1 && \
	  cargo test -p edgequake-storage --features postgres --test e2e_spec091_job_control -- --test-threads=1 && \
	  cargo test -p edgequake-storage --features postgres --test e2e_spec091_chunk_embeddings -- --test-threads=1 && \
	  cargo test -p edgequake-storage --features postgres --test e2e_spec091_vector_backfill -- --test-threads=1 && \
	  cargo test -p edgequake-storage --features postgres --test e2e_spec091_recall_parity -- --test-threads=1 && \
	  cargo test -p edgequake-storage --features postgres --test e2e_spec091_vector_backend_dual -- --test-threads=1 && \
	  cargo test -p edgequake-storage --features postgres --test contract_spec091_vector_write_stop -- --test-threads=1 && \
	  cargo test -p edgequake-storage --features postgres --test e2e_spec091_typed_only_ingest -- --test-threads=1 && \
	  cargo test -p edgequake-storage --features postgres --test e2e_spec091_vector_retire -- --test-threads=1 && \
	  cargo test -p edgequake-api --features postgres --test contract_spec091_boot_gate -- --test-threads=1 && \
	  cargo test -p edgequake --features postgres --test cli_migrate_console -- --test-threads=1 && \
	  cargo test -p edgequake-api --features postgres --test contract_spec091_strict_scope_headers -- --test-threads=1 && \
	  cargo test -p edgequake-api --features postgres --test contract_spec091_cqrs_batch_sink -- --test-threads=1 && \
	  cargo test -p edgequake-api --features postgres --test contract_spec091_outbox_ingest -- --test-threads=1 && \
	  cargo test -p edgequake-api --features postgres --test contract_spec091_outbox_drain -- --test-threads=1 && \
	  cargo test -p edgequake-pipeline --lib spec091 -- --test-threads=1 && \
	  cargo test -p edgequake-storage --features postgres --test contract_spec091_chunk_fts -- --test-threads=1 && \
	  cargo test -p edgequake-storage --features postgres --test contract_spec091_age_citation_indexes -- --test-threads=1 && \
	  cargo test -p edgequake-tasks --lib batch_estimate_ranks_pending_page -- --test-threads=1 && \
	  cargo test -p edgequake-pipeline --lib typed_authority_skips_legacy_chunk_vector_upsert -- --test-threads=1 && \
	  cargo test -p edgequake-storage --lib contract_spec091_serving_fence_default_on -- --test-threads=1 && \
	  cargo test -p edgequake-storage --features postgres --test contract_spec091_get_by_ids_typed -- --test-threads=1 && \
	  cargo test -p edgequake-storage --features postgres --test contract_spec091_unknown_family_loud -- --test-threads=1 && \
	  cargo test -p edgequake-storage --features postgres --test contract_spec091_llm_cache_scope -- --test-threads=1 && \
	  cargo test -p edgequake-storage --features postgres --test e2e_spec091_ingestion_p95_budget -- --test-threads=1 && \
	  cargo test -p edgequake-storage --features postgres --test e2e_spec091_retrieval_slo_protection -- --test-threads=1 && \
	  cargo test -p edgequake-storage --features postgres --test contract_spec091_shell_batch_write -- --test-threads=1 && \
	  cargo test -p edgequake-storage --features postgres --test e2e_spec091_hnsw_policy_converged -- --test-threads=1 && \
	  cargo test -p edgequake-storage --features postgres --test e2e_spec091_fleet_recall_parity -- --test-threads=1 && \
	  cargo test -p edgequake-storage --features postgres --test contract_spec091_zero_runtime_ddl -- --test-threads=1 && \
	  cargo test -p edgequake-storage --features postgres --test contract_spec091_kv_ping_short_circuits -- --test-threads=1 && \
	  cargo test -p edgequake-storage --features postgres --test e2e_spec091_hot_path_no_missing_kv_sql -- --test-threads=1 && \
	  cargo test -p edgequake-storage --features postgres --test contract_spec091_admission_stamps_track_id -- --test-threads=1 && \
	  cargo test -p edgequake-storage --features postgres --test contract_spec091_advisor_purge_aware_residue -- --test-threads=1 && \
	  cargo test -p edgequake-api --features postgres --test contract_spec091_health_chunk_text_ssot -- --test-threads=1 && \
	  cargo test -p edgequake-storage --test contract_spec091_no_kv_facade && \
	  cargo test -p edgequake-storage --test proptest_spec091_key_grammar && \
	  cargo test -p edgequake-storage --features postgres --test e2e_spec091_cross_tenant_graph_leak -- --test-threads=1 && \
	  cargo test -p edgequake-storage --features postgres --test e2e_spec091_cross_tenant_ann_leak -- --test-threads=1 && \
	  cargo test -p edgequake-storage --features postgres --test e2e_spec091_workspace_delete_zero_residue_1m -- --test-threads=1 && \
	  cargo test -p edgequake-tasks --features postgres --test contract_spec091_provider_budget -- --test-threads=1 && \
	  cargo test -p edgequake-tasks --test contract_spec091_fairness_release_before_materialize -- --test-threads=1 && \
	  cargo test -p edgequake-api --test contract_spec091_cancel_gates -- --test-threads=1 && \
	  cargo test -p edgequake-api --features postgres --test contract_spec091_checkpoint_typed_write_stop -- --test-threads=1
	@echo "$(GREEN)✓ SPEC-091 gates complete$(RESET)"

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
	@$(APPLY_LANGFUSE_ENV); \
	EDGEQUAKE_LLM_PROVIDER=$${EDGEQUAKE_LLM_PROVIDER:-$$([ -n "$(OPENAI_API_KEY)" ] && echo "openai" || echo "ollama")} \
	OPENAI_API_KEY="$(OPENAI_API_KEY)" \
	EDGEQUAKE_VERSION=$${EDGEQUAKE_VERSION:-latest} \
	docker compose -f $(QUICKSTART_COMPOSE) pull
	@echo ""
	@echo "$(YELLOW)→ Starting services...$(RESET)"
	@$(APPLY_LANGFUSE_ENV); \
	EDGEQUAKE_LLM_PROVIDER=$${EDGEQUAKE_LLM_PROVIDER:-$$([ -n "$(OPENAI_API_KEY)" ] && echo "openai" || echo "ollama")} \
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

# SPEC-122: bulk ingest measurement harness (LAW-122-5). Does not raise concurrency.
# Usage: make measure-bulk-ingest ARM=D N=5
# Optional: BASE_URL WORKSPACE_ID TIMEOUT_S EDGEQUAKE_TOKEN
measure-bulk-ingest: ## SPEC-122 measure bulk ingest (admit vs t_all; docs/min)
	@curl -sf "$(BACKEND_URL)/health" >/dev/null || { \
		echo "$(RED)✗ Backend not healthy at $(BACKEND_URL). Start with make backend-bg / make dev-bg.$(RESET)"; exit 1; \
	}
	@echo "$(BLUE)SPEC-122 measure ARM=$${ARM:-C} N=$${N:-1} → $(BACKEND_URL)$(RESET)"
	@BASE_URL="$(BACKEND_URL)" \
		WORKSPACE_ID="$${WORKSPACE_ID:-00000000-0000-0000-0000-000000000003}" \
		ARM="$${ARM:-C}" N="$${N:-1}" TIMEOUT_S="$${TIMEOUT_S:-1800}" \
		EDGEQUAKE_TOKEN="$${EDGEQUAKE_TOKEN:-}" \
		python3 $(ROOT_DIR)/specs/122-implementation/scripts/measure-bulk-ingest.py

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

spec114-e2e-mistral-extract: db-wait ## SPEC-114 live Mistral extract under KG schema (MISTRAL_API_KEY + PostgreSQL)
	@echo "$(BLUE)SPEC-114 live Mistral extract (PostgreSQL, mistral-small-latest)...$(RESET)"
	@if [ -z "$(MISTRAL_API_KEY)" ] && [ -z "$$MISTRAL_API_KEY" ]; then \
		echo "$(RED)✗ MISTRAL_API_KEY required for spec114-e2e-mistral-extract$(RESET)"; exit 1; \
	fi
	@_DB=$$(cat /tmp/edgequake-db-url 2>/dev/null); \
	[ -n "$$_DB" ] || _DB="$(DATABASE_URL)"; \
	[ -n "$$_DB" ] || { echo "$(RED)✗ DATABASE_URL required$(RESET)"; exit 1; }; \
	cd $(BACKEND_DIR) && DATABASE_URL="$$_DB" MISTRAL_API_KEY="$${MISTRAL_API_KEY:-$(MISTRAL_API_KEY)}" \
		EDGEQUAKE_LLM_PROVIDER=mistral \
		EDGEQUAKE_EXTRACT_REASONING_EFFORT=none \
		cargo test -p edgequake-api --features postgres \
		--test e2e_spec114_mistral_extract -- --ignored --nocapture --test-threads=1

spec114-e2e-ollama-extract: db-wait ## SPEC-114 live Ollama extract (qwen3.6:35b-a3b + PostgreSQL)
	@echo "$(BLUE)SPEC-114 live Ollama extract (PostgreSQL, qwen3.6:35b-a3b)...$(RESET)"
	@_DB=$$(cat /tmp/edgequake-db-url 2>/dev/null); \
	[ -n "$$_DB" ] || _DB="$(DATABASE_URL)"; \
	[ -n "$$_DB" ] || { echo "$(RED)✗ DATABASE_URL required$(RESET)"; exit 1; }; \
	_HOST="$${OLLAMA_HOST:-http://localhost:11434}"; \
	curl -sf "$$_HOST/api/tags" >/dev/null 2>&1 || { \
		echo "$(RED)✗ Ollama not reachable at $$_HOST$(RESET)"; exit 1; \
	}; \
	cd $(BACKEND_DIR) && DATABASE_URL="$$_DB" OLLAMA_HOST="$$_HOST" \
		EDGEQUAKE_LLM_PROVIDER=ollama \
		EDGEQUAKE_EMBEDDING_PROVIDER=ollama \
		EDGEQUAKE_LLM_MODEL=qwen3.6:35b-a3b \
		EDGEQUAKE_EXTRACT_REASONING_EFFORT=none \
		EDGEQUAKE_REASONING_EFFORT=none \
		cargo test -p edgequake-api --features postgres \
		--test e2e_spec114_ollama_extract -- --ignored --nocapture --test-threads=1

spec114-e2e-live-extract: db-wait ## SPEC-114 live extract — Mistral and/or Ollama (skip missing)
	@echo "$(BLUE)SPEC-114 live extract matrix (Mistral + Ollama)...$(RESET)"
	@if [ -n "$(MISTRAL_API_KEY)" ] || [ -n "$$MISTRAL_API_KEY" ]; then \
		$(MAKE) spec114-e2e-mistral-extract --no-print-directory; \
	else \
		echo "$(YELLOW)→ Skipping Mistral live extract (MISTRAL_API_KEY not set)$(RESET)"; \
	fi
	@_HOST="$${OLLAMA_HOST:-http://localhost:11434}"; \
	if curl -sf "$$_HOST/api/tags" >/dev/null 2>&1; then \
		$(MAKE) spec114-e2e-ollama-extract --no-print-directory; \
	else \
		echo "$(YELLOW)→ Skipping Ollama live extract (Ollama not reachable at $$_HOST)$(RESET)"; \
	fi
	@echo "$(GREEN)✓ SPEC-114 live extract gate finished$(RESET)"

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
		--test e2e_spec013_github_issues -- $(SPEC013_CARGO_TEST_ARGS) --nocapture && \
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
		TEST_DATABASE_URL="postgresql://app_user:app_password_123@localhost:$${POSTGRES_TEST_PORT:-5433}/edgequake_test" \
		ADMIN_DATABASE_URL="postgresql://edgequake_test:test_password_123@localhost:$${POSTGRES_TEST_PORT:-5433}/edgequake_test" \
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

.PHONY: spec020-qc-proof observability-proof observability-jaeger resource-proof resource-proof-postgres release-gates spec124-proof spec124-langfuse-e2e spec124-langfuse-3.1-e2e spec124-langfuse-3.22-e2e spec124-langfuse-3.225-e2e spec124-langfuse-cloud-e2e spec124-langfuse-matrix spec125-proof spec128-proof spec145-proof spec145-langfuse-e2e

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

release-gates: ## Pre-release gate: fmt, workspace clippy, SPEC-006/018, WebUI, version parity
	@chmod +x scripts/release_gates.sh
	@./scripts/release_gates.sh

.PHONY: git-hygiene
git-hygiene: ## SPEC-097 / GH-351: block fat bench artifacts and >50MiB tip blobs
	@chmod +x tools/git-hygiene/check_no_fat_artifacts.sh
	@./tools/git-hygiene/check_no_fat_artifacts.sh

.PHONY: spec090-perf-smoke
spec090-perf-smoke: ## SPEC-090 falsifiable scaling smoke (needs DATABASE_URL)
	@chmod +x scripts/perf/spec090_scaling_smoke.sh
	@./scripts/perf/spec090_scaling_smoke.sh

observability-proof: ## Run SPEC-018 observability proof suite (Rust + WebUI)
	@./specs/018-observability/e2e/run_observability_proof.sh

spec124-proof: ## SPEC-124 Langfuse CI-unfakable proofs (InMemory OTEL + contracts; no keys)
	@echo "$(BOLD)SPEC-124 proof$(RESET)"
	@cd $(BACKEND_DIR) && cargo test -p edgequake-observability --lib inmemory_otel
	@cd $(BACKEND_DIR) && cargo test -p edgequake-observability --lib langfuse_
	@cd $(BACKEND_DIR) && cargo test -p edgequake-observability --lib rag_span
	@cd $(BACKEND_DIR) && cargo test -p edgequake-query --lib spec124_stream_genai
	@cd $(BACKEND_DIR) && cargo test -p edgequake-query --lib spec124_pipeline_meta
	@cd $(BACKEND_DIR) && cargo test -p edgequake-pipeline --lib gleaning_source_wraps_llm_generation
	@cd $(BACKEND_DIR) && cargo test -p edgequake-pipeline --lib spec124_ingest_stages
	@cd $(BACKEND_DIR) && cargo test -p edgequake-api --lib spec124_ingest_converting
	@echo "$(GREEN)✓ SPEC-124 proof passed$(RESET)"

spec145-proof: ## SPEC-145 Complete Langfuse I/O (InMemory + io_policy + stream contract; no keys)
	@echo "$(BOLD)SPEC-145 proof$(RESET)"
	@chmod +x scripts/spec145_langfuse_io_e2e.sh
	@./scripts/spec145_langfuse_io_e2e.sh
	@echo "$(GREEN)✓ SPEC-145 proof passed$(RESET)"

spec145-langfuse-e2e: ## SPEC-145 live Complete I/O vs Langfuse 3.225.5 (starts stack; OTLP persist)
	@echo "$(BOLD)SPEC-145 live Langfuse I/O$(RESET)"
	@$(MAKE) langfuse-3.225-up --no-print-directory
	@chmod +x $(ROOT_DIR)/scripts/spec145_langfuse_io_e2e.sh
	@LANGFUSE_SPEC145_E2E=1 \
		LANGFUSE_BASE_URL="$(LANGFUSE_3225_UI_URL)" \
		LANGFUSE_OTLP_E2E_BASE="$(LANGFUSE_3225_UI_URL)" \
		LANGFUSE_PUBLIC_KEY="$(LANGFUSE_3225_PK)" \
		LANGFUSE_SECRET_KEY="$(LANGFUSE_3225_SK)" \
		$(ROOT_DIR)/scripts/spec145_langfuse_io_e2e.sh
	@echo "$(GREEN)✓ SPEC-145 live Langfuse I/O passed$(RESET)"

spec125-proof: ## SPEC-125 markdown pack proofs (heading-dense fixture + Acc geometry + ingest.chunking distribution)
	@echo "$(BOLD)SPEC-125 proof$(RESET)"
	@cd $(BACKEND_DIR) && cargo test -p edgequake-pipeline --lib markdown_pack
	@cd $(BACKEND_DIR) && cargo test -p edgequake-pipeline --test contract_spec125_markdown_pack
	@cd $(BACKEND_DIR) && cargo test -p edgequake-pipeline --test e2e_spec125_markdown_pack
	@cd $(BACKEND_DIR) && cargo test -p edgequake-pipeline --test contract_spec026_recursive_chunking
	@cd $(BACKEND_DIR) && cargo test -p edgequake-pipeline --test e2e_spec116_chunk_geometry
	@cd $(BACKEND_DIR) && cargo test -p edgequake-observability --lib inmemory_ingest_chunking
	@echo "$(GREEN)✓ SPEC-125 proof passed$(RESET)"

spec128-proof: ## SPEC-128 figure prune SSOT + layout overlay contracts (unfakable)
	@echo "$(BOLD)SPEC-128 proof$(RESET)"
	@rg -n "apply_filter_result_or_keep|apply_filter_to_figure_map" edgequake/crates/edgequake-pdf/src/backend/vision.rs >/dev/null \
		|| { echo "$(RED)G-prune missing: vision.rs must prune figure_map after filter$(RESET)"; exit 1; }
	@rg -n "attach_figure_filter_if_enabled" edgequake/crates/edgequake-api/src/processor/pdf_processing.rs >/dev/null \
		|| { echo "$(RED)WP-1 missing: ingest must attach figure_filter_provider$(RESET)"; exit 1; }
	@rg -n "PdfPageOverlay" edgequake_webui/src/components/documents/pdf-viewer.tsx >/dev/null \
		|| { echo "$(RED)overlay missing: pdf-viewer.tsx must host PdfPageOverlay$(RESET)"; exit 1; }
	@rg -n "pdf-layout-overlay" edgequake_webui/src/components/documents/pdf-page-overlay.tsx >/dev/null \
		|| { echo "$(RED)overlay missing: pdf-page-overlay.tsx must render pdf-layout-overlay$(RESET)"; exit 1; }
	@cd $(BACKEND_DIR) && cargo clippy -p edgequake-pdf --lib --no-deps -- -D warnings
	@cd $(BACKEND_DIR) && cargo clippy -p edgequake-storage --lib --no-deps -- -D warnings
	@cd $(BACKEND_DIR) && cargo test -p edgequake-pdf --lib figure_filter::
	@cd $(BACKEND_DIR) && cargo test -p edgequake-pdf --lib page_layout::
	@cd $(BACKEND_DIR) && cargo test -p edgequake-storage --lib page_layout_storage::
	@cd $(BACKEND_DIR) && cargo test -p edgequake-api --lib document_page_layout_persist::
	@cd $(BACKEND_DIR) && cargo test -p edgequake-observability --lib spec128_page_layout
	@cd $(BACKEND_DIR) && cargo test -p edgequake-api --lib attach_figure_filter_honors_env
	@cd $(BACKEND_DIR) && cargo test -p edgequake-api --test contract_spec049_figure_filter
	@cd $(BACKEND_DIR) && cargo test -p edgequake-api --features postgres --test contract_spec128_page_layout
	@if [ -f "$(ROOT_DIR)/../edgequake-pdf2md/src/pipeline/visual/text_blocks.rs" ]; then \
		cargo test --manifest-path "$(ROOT_DIR)/../edgequake-pdf2md/Cargo.toml" --lib text_blocks:: ; \
	else \
		echo "$(YELLOW)skip pdf2md text_blocks (sibling crate missing)$(RESET)"; \
	fi
	@node $(FRONTEND_DIR)/scripts/copy-pdf-worker.mjs
	@# Mocked overlay suite uses Playwright webServer (do not reuse a stale make-dev FE).
	@cd $(FRONTEND_DIR) && PLAYWRIGHT_SKIP_STACK_CHECK=1 pnpm exec playwright test e2e/spec128-layout-overlay.spec.ts --project=chromium --grep-invert "live " --retries=1
	@if curl -fsS "$(BACKEND_URL)/health" >/dev/null 2>&1 && curl -fsS "$(FRONTEND_URL)" 2>/dev/null | grep -qi 'EdgeQuake'; then \
		echo "$(YELLOW)→ SPEC-128 live overlay (persisted layout)$(RESET)"; \
		cd $(FRONTEND_DIR) && E2E_LIVE_STACK=1 EQ_BACKEND_URL="$(BACKEND_URL)" EDGEQUAKE_API_URL="$(BACKEND_URL)" PLAYWRIGHT_BASE_URL="$(FRONTEND_URL)" pnpm exec playwright test e2e/spec128-layout-overlay.spec.ts --project=chromium --grep "live overlay on persisted"; \
		if [ -n "$$MISTRAL_API_KEY" ]; then \
			echo "$(YELLOW)→ SPEC-128 live mistral-small on pdf_data$(RESET)"; \
			cd $(FRONTEND_DIR) && E2E_LIVE_STACK=1 EQ_BACKEND_URL="$(BACKEND_URL)" EDGEQUAKE_API_URL="$(BACKEND_URL)" PLAYWRIGHT_BASE_URL="$(FRONTEND_URL)" pnpm exec playwright test e2e/spec128-layout-overlay.spec.ts --project=chromium --grep "live mistral"; \
		else \
			echo "$(YELLOW)skip SPEC-128 live mistral (MISTRAL_API_KEY unset)$(RESET)"; \
		fi; \
	else \
		echo "$(YELLOW)skip SPEC-128 live overlay (stack not up)$(RESET)"; \
	fi
	@echo "$(GREEN)✓ SPEC-128 proof passed$(RESET)"

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
	@echo "$(BOLD)Langfuse (optional local Docker):$(RESET)"
	@if curl -sf "$(LANGFUSE_UI_URL)/api/public/health" >/dev/null 2>&1; then \
		echo "  $(GREEN)Running at $(LANGFUSE_UI_URL)$(RESET)"; \
	else \
		echo "  $(YELLOW)Not running — make langfuse-up (UI $(LANGFUSE_UI_URL))$(RESET)"; \
	fi
	@echo ""


# ============================================================================
# SPEC-047 — MMLongBench-Doc RAG evaluation (tools/bench047)
# ============================================================================

.PHONY: bench047-install bench047-doctor bench047-freeze-smoke bench047-smoke bench047-smoke-vision-medium bench047-core bench047-full bench047-freeze-core bench047-phase-b-core

bench047-install: ## Install SPEC-047 Python harness (editable)
	@cd tools/bench047 && pip3 install -e . -q
	@echo "$(GREEN)✓ bench047 installed$(RESET)"

bench047-doctor: bench047-install ## Check API + Mistral profile for SPEC-047
	@set -a && [ -f "$(DEV_PORTS_ENV)" ] && . "$(DEV_PORTS_ENV)"; set +a; \
	EDGEQUAKE_API_URL="$${EDGEQUAKE_API_URL:-$(BACKEND_URL)}" \
	python3 -m bench047.cli doctor --api "$${EDGEQUAKE_API_URL:-$(BACKEND_URL)}" \
		--profile "$${BENCH047_PROFILE:-P0_mm_ite}"

bench047-freeze-smoke: bench047-install ## Freeze stratified 10-doc smoke fixture
	@python3 -m bench047.cli download-qa
	@python3 -m bench047.cli freeze-smoke
	@python3 -m bench047.cli download-pdfs

bench047-smoke: bench047-install ## SPEC-047 chart-8 smoke (locked Acc physics: P0_mm_ite + dscope + Small)
	@set -a && [ -f "$(DEV_PORTS_ENV)" ] && . "$(DEV_PORTS_ENV)"; set +a; \
	export EDGEQUAKE_API_URL="$${EDGEQUAKE_API_URL:-$(BACKEND_URL)}"; \
	export MISTRAL_MODEL="$${MISTRAL_MODEL:-mistral-small-latest}"; \
	export EDGEQUAKE_LLM_PROVIDER=mistral EDGEQUAKE_LLM_MODEL=mistral-small-latest; \
	export EDGEQUAKE_EMBEDDING_PROVIDER=mistral MISTRAL_EMBEDDING_MODEL=mistral-embed; \
	export EDGEQUAKE_VISION_PROVIDER=mistral EDGEQUAKE_VISION_MODEL=mistral-small-latest; \
	export VLM_PROCESS_ENABLE=true; \
	export EDGEQUAKE_BENCH_FIXTURE="$${EDGEQUAKE_BENCH_FIXTURE:-smoke_chart_doc_ids_v1.txt}"; \
	export BENCH047_WORKERS="$${BENCH047_WORKERS:-2}"; \
	chmod +x tools/bench047/scripts/ensure_backend_small.sh tools/bench047/scripts/run_chart8_smoke.sh; \
	tools/bench047/scripts/run_chart8_smoke.sh; \
	echo "$(GREEN)→ SUMMARY:$(RESET) specs/047-rag-evaluation/e2e/artifacts/smoke/SUMMARY.md"

bench047-smoke-vision-medium: bench047-install ## Chart-8 Acc physics + mistral-medium-3-5 vision (025 W1)
	@set -a && [ -f "$(DEV_PORTS_ENV)" ] && . "$(DEV_PORTS_ENV)"; set +a; \
	export EDGEQUAKE_API_URL="$${EDGEQUAKE_API_URL:-$(BACKEND_URL)}"; \
	export MISTRAL_MODEL="$${MISTRAL_MODEL:-mistral-small-latest}"; \
	export EDGEQUAKE_LLM_PROVIDER=mistral EDGEQUAKE_LLM_MODEL=mistral-small-latest; \
	export EDGEQUAKE_EMBEDDING_PROVIDER=mistral MISTRAL_EMBEDDING_MODEL=mistral-embed; \
	export EDGEQUAKE_VISION_PROVIDER=mistral EDGEQUAKE_VISION_MODEL=mistral-medium-3-5; \
	export VLM_PROCESS_ENABLE=true; \
	export EDGEQUAKE_BENCH_FIXTURE="$${EDGEQUAKE_BENCH_FIXTURE:-smoke_chart_doc_ids_v1.txt}"; \
	export BENCH047_WORKERS="$${BENCH047_WORKERS:-2}"; \
	chmod +x tools/bench047/scripts/ensure_backend_small.sh tools/bench047/scripts/run_chart8_vision_medium.sh; \
	tools/bench047/scripts/run_chart8_vision_medium.sh; \
	echo "$(GREEN)→ SUMMARY:$(RESET) specs/047-rag-evaluation/e2e/artifacts/smoke/SUMMARY.md"

bench047-core: bench047-install ## Run SPEC-047 core (~40 docs) — requires --i-accept-cost
	@set -a && [ -f "$(DEV_PORTS_ENV)" ] && . "$(DEV_PORTS_ENV)"; set +a; \
	export EDGEQUAKE_API_URL="$${EDGEQUAKE_API_URL:-$(BACKEND_URL)}"; \
	python3 -m bench047.cli core --api "$$EDGEQUAKE_API_URL" --profile P0_mm_ite --document-scope --i-accept-cost

bench047-freeze-core: bench047-install ## Freeze ~40-doc core fixture (Phase B)
	@python3 -m bench047.cli download-qa
	@python3 -m bench047.cli freeze-core -n 40
	@python3 -m bench047.cli download-pdfs --fixture core_doc_ids_v1.txt

bench047-phase-b-core: bench047-install ## Phase B core bench + checkpoint assess every 5 docs
	@set -a && [ -f "$(DEV_PORTS_ENV)" ] && . "$(DEV_PORTS_ENV)"; set +a; \
	export EDGEQUAKE_API_URL="$${EDGEQUAKE_API_URL:-$(BACKEND_URL)}"; \
	export MISTRAL_MODEL="$${MISTRAL_MODEL:-mistral-small-latest}"; \
	export EDGEQUAKE_LLM_PROVIDER=mistral EDGEQUAKE_LLM_MODEL=mistral-small-latest; \
	export EDGEQUAKE_EMBEDDING_PROVIDER=mistral MISTRAL_EMBEDDING_MODEL=mistral-embed; \
	export EDGEQUAKE_VISION_PROVIDER=mistral EDGEQUAKE_VISION_MODEL=mistral-small-latest; \
	export VLM_PROCESS_ENABLE=true; \
	chmod +x tools/bench047/scripts/ensure_backend_small.sh tools/bench047/scripts/run_phase_b_core.sh; \
	tools/bench047/scripts/run_phase_b_core.sh; \
	echo "$(GREEN)→ checkpoints:$(RESET) specs/047-rag-evaluation/e2e/artifacts/core-checkpoints/"

bench047-full: bench047-install ## SPEC-047 full (135 docs) ≥10 parallel ingest + Small pins
	@set -a && [ -f "$(DEV_PORTS_ENV)" ] && . "$(DEV_PORTS_ENV)"; set +a; \
	export EDGEQUAKE_API_URL="$${EDGEQUAKE_API_URL:-$(BACKEND_URL)}"; \
	export MISTRAL_MODEL="$${MISTRAL_MODEL:-mistral-small-latest}"; \
	export EDGEQUAKE_LLM_PROVIDER=mistral EDGEQUAKE_LLM_MODEL=mistral-small-latest; \
	export EDGEQUAKE_EMBEDDING_PROVIDER=mistral MISTRAL_EMBEDDING_MODEL=mistral-embed; \
	export EDGEQUAKE_VISION_PROVIDER=mistral EDGEQUAKE_VISION_MODEL=mistral-small-latest; \
	export VLM_PROCESS_ENABLE=true; \
	export BENCH047_INGEST_WORKERS="$${BENCH047_INGEST_WORKERS:-10}"; \
	export BENCH047_WORKERS="$${BENCH047_WORKERS:-4}"; \
	chmod +x tools/bench047/scripts/ensure_backend_small.sh tools/bench047/scripts/run_full_parallel.sh; \
	tools/bench047/scripts/run_full_parallel.sh; \
	echo "$(GREEN)→ SUMMARY:$(RESET) specs/047-rag-evaluation/e2e/artifacts/full/SUMMARY.md"
# ---------------------------------------------------------------------------
# SPEC-001 — EdgeQuake HybridRAG vs LightRAG (GraphRAG-Bench)
# ---------------------------------------------------------------------------
.PHONY: bench bench-warm bench001-medical-mid-eq-llm-cache-warm bench001-install bench001-doctor bench001-freeze-smoke bench001-smoke bench001-smoke-fast bench001-smoke-fast-large bench001-smoke-fast-acc bench001-smoke-acc bench001-medical-mid bench001-full bench001 bench001-acc-canary bench001-smoke-paper bench001-core bench001-acc-backend bench001-watch bench001-f1a bench001-f2a bench001-f3a bench001-f4a bench001-p0 bench001-p1a bench001-p1b bench001-p2a bench001-p2b bench001-p3a bench001-p3b bench001-p4 bench001-p5 bench001-q0 bench001-q1 bench001-q2 bench001-q3 bench001-q4 bench001-r0 bench001-r1 bench001-r2 bench001-r3 bench001-s0 bench001-s1 bench001-t0 bench001-t0b bench001-t0c bench001-t0d bench001-t1 bench001-a0 bench001-a1 bench001-a2 bench001-a3 bench001-a4 bench001-lr-identity bench001-lr-pack-bm25 bench001-lr-identity-fact-l2 bench001-medical-mid-lr-identity-fact-l2 bench001-lr-nf-fact-l2 bench001-medical-mid-lr-nf-fact-l2 bench001-lr-dense-fact-l2 bench001-medical-mid-lr-dense-fact-l2 bench001-lr-occ-fact-l2 bench001-medical-mid-lr-occ-fact-l2 bench001-lr-posttrunc-fact-l2 bench001-medical-mid-lr-posttrunc-fact-l2 bench001-medical-full-lr-occ-fact-l2 bench001-medical-full-p0 bench001-b1-audit bench001-b2-reingest bench001-b3-reingest bench001-b3b-reingest bench001-b5-reingest bench001-b6-reingest bench001-b7-reingest bench001-b8-reingest bench001-b9-reingest bench001-b10-reingest bench001-c1a bench001-c1b bench001-c1d bench001-c1e bench001-c1cold bench001-lr-unify-fact-l2 bench001-medical-mid-lr-unify-fact-l2 bench001-medical-full-lr-unify-fact-l2 bench001-lr-intent-w-fact-l2 bench001-medical-mid-lr-intent-w-fact-l2 bench001-lr-relsel-fact-l2 bench001-d0-forensics

bench001-install: ## Install SPEC-001 Python harness (editable)
	@cd tools/bench001 && pip3 install -e . -q
	@echo "$(GREEN)✓ bench001 installed$(RESET)"

bench001-watch: ## Refresh LIVE.md progress board (STAGE=smoke-fast; INTERVAL=2)
	@STAGE="$${STAGE:-smoke-fast}"; INTERVAL="$${INTERVAL:-2}"; \
	echo "$(BLUE)→ Watching specs/001-benchmark/e2e/artifacts/$$STAGE/LIVE.md (every $${INTERVAL}s)$(RESET)"; \
	echo "  also: specs/001-benchmark/e2e/artifacts/LIVE.md"; \
	export PYTHONPATH="tools/bench001:$${PYTHONPATH}"; \
	while true; do \
	  clear 2>/dev/null || true; \
	  date -u +"UTC %Y-%m-%dT%H:%M:%SZ"; \
	  python3 -m bench001.cli live "$$STAGE" 2>/dev/null || \
	    python3 -m bench001.cli live 2>/dev/null || \
	    echo "(no LIVE.md yet — start a run first)"; \
	  sleep "$$INTERVAL"; \
	done

bench001-doctor: bench001-install ## Preflight: fixtures, EQ health, keys, LightRAG
	@set -a && [ -f "$(DEV_PORTS_ENV)" ] && . "$(DEV_PORTS_ENV)"; set +a; \
	EDGEQUAKE_API_URL="$${EDGEQUAKE_API_URL:-$(BACKEND_URL)}" \
	python3 -m bench001.cli doctor --api "$${EDGEQUAKE_API_URL:-$(BACKEND_URL)}"

bench001-freeze-smoke: bench001-install ## Download GraphRAG-Bench + verify smoke fixture IDs
	@python3 -m bench001.cli freeze-smoke

bench001-smoke: bench001-install ## SPEC-001 dual-SUT smoke (default mistral-small-latest + mistral-embed); BENCH001_DRY_RUN=1 offline
	@set -a && [ -f "$(DEV_PORTS_ENV)" ] && . "$(DEV_PORTS_ENV)"; set +a; \
	export EDGEQUAKE_API_URL="$${EDGEQUAKE_API_URL:-$(BACKEND_URL)}"; \
	export EDGEQUAKE_LLM_PROVIDER="$${EDGEQUAKE_LLM_PROVIDER:-$${BENCH001_LLM_PROVIDER:-mistral}}"; \
	export EDGEQUAKE_LLM_MODEL="$${EDGEQUAKE_LLM_MODEL:-$${BENCH001_LLM_MODEL:-$${MISTRAL_MODEL:-mistral-small-latest}}}"; \
	export MISTRAL_MODEL="$$EDGEQUAKE_LLM_MODEL"; \
	export EDGEQUAKE_EMBEDDING_PROVIDER="$${EDGEQUAKE_EMBEDDING_PROVIDER:-$${BENCH001_EMBEDDING_PROVIDER:-mistral}}"; \
	export MISTRAL_EMBEDDING_MODEL="$${MISTRAL_EMBEDDING_MODEL:-$${BENCH001_EMBEDDING_MODEL:-mistral-embed}}"; \
	export EDGEQUAKE_VISION_PROVIDER="$${EDGEQUAKE_VISION_PROVIDER:-$${BENCH001_VISION_PROVIDER:-mistral}}"; \
	export EDGEQUAKE_VISION_MODEL="$${EDGEQUAKE_VISION_MODEL:-$${BENCH001_VISION_MODEL:-mistral-small-latest}}"; \
	export VLM_PROCESS_ENABLE=true; \
	export BENCH001_QUERY_CONCURRENCY="$${BENCH001_QUERY_CONCURRENCY:-8}"; \
	export BENCH001_EVAL_CONCURRENCY="$${BENCH001_EVAL_CONCURRENCY:-8}"; \
	if [ "$${BENCH001_DRY_RUN:-0}" = "1" ]; then \
	  python3 -m bench001.cli smoke --dry-run --query-concurrency "$$BENCH001_QUERY_CONCURRENCY" --eval-concurrency "$$BENCH001_EVAL_CONCURRENCY"; \
	else \
	  python3 -m bench001.cli smoke --api "$$EDGEQUAKE_API_URL" --query-concurrency "$$BENCH001_QUERY_CONCURRENCY" --eval-concurrency "$$BENCH001_EVAL_CONCURRENCY"; \
	fi; \
	echo "$(GREEN)→ SUMMARY:$(RESET) specs/001-benchmark/e2e/artifacts/smoke/SUMMARY.md"

bench001-smoke-fast: bench001-install ## Fast smoke gate (8 Qs, query-only, concurrency 12); needs warm smoke indexes
	@set -a && [ -f "$(DEV_PORTS_ENV)" ] && . "$(DEV_PORTS_ENV)"; set +a; \
	export EDGEQUAKE_API_URL="$${EDGEQUAKE_API_URL:-$(BACKEND_URL)}"; \
	export EDGEQUAKE_LLM_PROVIDER="$${EDGEQUAKE_LLM_PROVIDER:-$${BENCH001_LLM_PROVIDER:-mistral}}"; \
	export EDGEQUAKE_LLM_MODEL="$${EDGEQUAKE_LLM_MODEL:-$${BENCH001_LLM_MODEL:-$${MISTRAL_MODEL:-mistral-small-latest}}}"; \
	export MISTRAL_MODEL="$$EDGEQUAKE_LLM_MODEL"; \
	export EDGEQUAKE_EMBEDDING_PROVIDER="$${EDGEQUAKE_EMBEDDING_PROVIDER:-$${BENCH001_EMBEDDING_PROVIDER:-mistral}}"; \
	export MISTRAL_EMBEDDING_MODEL="$${MISTRAL_EMBEDDING_MODEL:-$${BENCH001_EMBEDDING_MODEL:-mistral-embed}}"; \
	export EDGEQUAKE_VISION_PROVIDER="$${EDGEQUAKE_VISION_PROVIDER:-$${BENCH001_VISION_PROVIDER:-mistral}}"; \
	export EDGEQUAKE_VISION_MODEL="$${EDGEQUAKE_VISION_MODEL:-$${BENCH001_VISION_MODEL:-mistral-small-latest}}"; \
	export VLM_PROCESS_ENABLE=true; \
	export BENCH001_QUERY_CONCURRENCY="$${BENCH001_QUERY_CONCURRENCY:-12}"; \
	export BENCH001_EVAL_CONCURRENCY="$${BENCH001_EVAL_CONCURRENCY:-16}"; \
	if [ -n "$${BENCH001_EQ_WORKSPACE_ID:-}" ]; then export BENCH001_EQ_WORKSPACE_ID; fi; \
	export LLM_API_KEY="$${LLM_API_KEY:-$$MISTRAL_API_KEY}"; \
	python3 -m bench001.cli smoke-fast --api "$$EDGEQUAKE_API_URL" --query-only \
	  --query-concurrency "$$BENCH001_QUERY_CONCURRENCY" --eval-concurrency "$$BENCH001_EVAL_CONCURRENCY"; \
	echo "$(GREEN)→ SUMMARY:$(RESET) specs/001-benchmark/e2e/artifacts/smoke-fast/SUMMARY.md"

# Ablation targets: force high judge fan-out (ignore stale BENCH001_EVAL_CONCURRENCY in shell).
BENCH001_LARGE_EVAL_CONCURRENCY ?= 24
BENCH001_LARGE_QUERY_CONCURRENCY ?= 4
BENCH001_ACC_EVAL_CONCURRENCY ?= 24
BENCH001_ACC_QUERY_CONCURRENCY ?= 4
# Acc-lift SUT+judge: mistral-small-latest (fast/reliable Acc loop; medium optional override).
BENCH001_ACC_LLM_MODEL ?= mistral-small-latest
BENCH001_ACC_JUDGE_MODEL ?= mistral-small-latest
# LR-like Mix: always-on local+global+naive (must also be on the EQ *server*).
BENCH001_EQ_MIX_ARM_GATE ?= false
# Fair Acc ingest chunk pin (LightRAG CHUNK_SIZE=1200 parity). Used by backend-lrlike + Acc targets.
BENCH001_EQ_ADAPTIVE_CHUNKING ?= 0
BENCH001_EQ_CHUNK_SIZE ?= 1200
BENCH001_EQ_CHUNK_OVERLAP ?= 100
# Cap corpus chars on smoke-fast Acc force-ingest (full medical ~1.05MB → hour+ merge risk).
# Set 0 for full corpus. Isolates EQ workspace / LR dir as *-c{N}.
BENCH001_INGEST_MAX_CHARS ?= 100000
BENCH001_INGEST_TIMEOUT_S ?= 1800
# Acc backend: detached double-fork (survives agent shell cleanup). Set 0 to use make backend-bg.
BENCH001_ACC_DAEMON ?= 1
BENCH001_ACC_EXTRACT_CONCURRENCY ?= 8

.PHONY: bench001-acc-backend
bench001-acc-backend: ## Detach Acc-pinned release backend (small + fair-1200 + rrf + extract∥)
	@set -a && [ -f "$(DEV_PORTS_ENV)" ] && . "$(DEV_PORTS_ENV)"; set +a; \
	if [ "$${BENCH001_SKIP_BACKEND_RESTART:-0}" = "1" ]; then \
	  echo "$(YELLOW)→ Skipping Acc backend restart (BENCH001_SKIP_BACKEND_RESTART=1)$(RESET)"; \
	  _H=$$(curl -sf "$${EDGEQUAKE_API_URL:-$(BACKEND_URL)}/health" || true); \
	  if [ -z "$$_H" ]; then echo "$(RED)→ Backend unhealthy while skip-restart set$(RESET)"; exit 1; fi; \
	  PYTHONPATH="tools/bench001:$${PYTHONPATH}" python3 -c \
	    "import json,sys; from bench001.acc_env import backend_pin_mismatches; \
h=json.loads(sys.argv[1]); bad=backend_pin_mismatches(h); \
print('acc pins OK' if not bad else 'pin mismatch: '+'; '.join(bad)); \
sys.exit(1 if bad else 0)" "$$_H" \
	    || { echo "$(RED)→ Acc pin mismatch — unset BENCH001_SKIP_BACKEND_RESTART and restart$(RESET)"; exit 1; }; \
	  exit 0; \
	fi; \
	if [ "$${BENCH001_ACC_DAEMON:-$(BENCH001_ACC_DAEMON)}" = "1" ]; then \
	  echo "$(YELLOW)→ Acc detached backend (port $${BACKEND_PORT:-$(BACKEND_PORT)})$(RESET)"; \
	  PYTHONPATH="tools/bench001:$${PYTHONPATH}" python3 tools/bench001/scripts/start_acc_backend.py \
	    --port "$${BACKEND_PORT:-$(BACKEND_PORT)}" --wait 90; \
	else \
	  $(MAKE) bench001-backend-lrlike --no-print-directory \
	    EDGEQUAKE_MIX_ARM_GATE="$(BENCH001_EQ_MIX_ARM_GATE)" \
	    EDGEQUAKE_ADAPTIVE_CHUNKING="$(BENCH001_EQ_ADAPTIVE_CHUNKING)" \
	    EDGEQUAKE_CHUNK_SIZE="$(BENCH001_EQ_CHUNK_SIZE)" \
	    EDGEQUAKE_CHUNK_OVERLAP="$(BENCH001_EQ_CHUNK_OVERLAP)" \
	    EDGEQUAKE_LLM_MODEL="$(BENCH001_ACC_LLM_MODEL)" \
	    EDGEQUAKE_MAX_CONCURRENT_EXTRACTIONS="$(BENCH001_ACC_EXTRACT_CONCURRENCY)" \
	    EDGEQUAKE_MIX_FUSION=round_robin; \
	fi

.PHONY: bench001-backend-lrlike
bench001-backend-lrlike: ## Ensure backend runs with EDGEQUAKE_MIX_ARM_GATE=false (LR-like Mix arms)
	@set -a && [ -f "$(DEV_PORTS_ENV)" ] && . "$(DEV_PORTS_ENV)"; set +a; \
	if [ "$${BENCH001_SKIP_BACKEND_RESTART:-0}" = "1" ]; then \
	  echo "$(YELLOW)→ Skipping backend restart (BENCH001_SKIP_BACKEND_RESTART=1)$(RESET)"; \
	  exit 0; \
	fi; \
	if curl -sf "$${EDGEQUAKE_API_URL:-$(BACKEND_URL)}/health" >/dev/null; then \
	  if ps eww -p $$(lsof -nP -iTCP:$${BACKEND_PORT:-$(BACKEND_PORT)} -sTCP:LISTEN -t 2>/dev/null | head -1) 2>/dev/null \
	    | tr ' ' '\n' | grep -q '^EDGEQUAKE_MIX_ARM_GATE=$(BENCH001_EQ_MIX_ARM_GATE)$$'; then \
	    echo "$(GREEN)→ Backend already healthy with EDGEQUAKE_MIX_ARM_GATE=$(BENCH001_EQ_MIX_ARM_GATE)$(RESET)"; \
	    exit 0; \
	  fi; \
	  if grep -q 'EDGEQUAKE_MIX_ARM_GATE="$(BENCH001_EQ_MIX_ARM_GATE)"' /tmp/edgequake-start.sh 2>/dev/null \
	    || grep -q 'EDGEQUAKE_MIX_ARM_GATE=$(BENCH001_EQ_MIX_ARM_GATE)' /tmp/eq-bench-start-trap.sh 2>/dev/null; then \
	    echo "$(GREEN)→ Backend healthy; start script pins Mix arm gate=$(BENCH001_EQ_MIX_ARM_GATE)$(RESET)"; \
	    exit 0; \
	  fi; \
	fi; \
	echo "$(YELLOW)→ Restart backend on port $${BACKEND_PORT:-$(BACKEND_PORT)} with EDGEQUAKE_MIX_ARM_GATE=$(BENCH001_EQ_MIX_ARM_GATE)$(RESET)"; \
	if [ -f /tmp/edgequake-backend.pid ]; then kill $$(cat /tmp/edgequake-backend.pid) 2>/dev/null || true; fi; \
	if [ -f /tmp/eq-bench-backend.pid ]; then kill $$(cat /tmp/eq-bench-backend.pid) 2>/dev/null || true; fi; \
	for BPID in $$(lsof -nP -iTCP:$${BACKEND_PORT:-$(BACKEND_PORT)} -sTCP:LISTEN -t 2>/dev/null || true); do \
	  kill "$$BPID" 2>/dev/null || true; \
	done; \
	rm -f /tmp/edgequake-backend.pid /tmp/eq-bench-backend.pid; \
	sleep 1; \
	$(MAKE) backend-bg --no-print-directory \
	  EDGEQUAKE_MIX_ARM_GATE="$(BENCH001_EQ_MIX_ARM_GATE)" \
	  EDGEQUAKE_ADAPTIVE_CHUNKING="$(BENCH001_EQ_ADAPTIVE_CHUNKING)" \
	  EDGEQUAKE_CHUNK_SIZE="$(BENCH001_EQ_CHUNK_SIZE)" \
	  EDGEQUAKE_CHUNK_OVERLAP="$(BENCH001_EQ_CHUNK_OVERLAP)"; \
	for i in $$(seq 1 60); do \
	  if curl -sf "$${EDGEQUAKE_API_URL:-$(BACKEND_URL)}/health" >/dev/null; then \
	    echo "$(GREEN)→ Backend healthy with Mix arm gate=$(BENCH001_EQ_MIX_ARM_GATE)$(RESET)"; \
	    grep -q 'EDGEQUAKE_MIX_ARM_GATE="$(BENCH001_EQ_MIX_ARM_GATE)"' /tmp/edgequake-start.sh \
	      && echo "$(GREEN)→ Confirmed /tmp/edgequake-start.sh exports EDGEQUAKE_MIX_ARM_GATE=$(BENCH001_EQ_MIX_ARM_GATE)$(RESET)" \
	      || echo "$(YELLOW)→ Warn: start.sh missing expected MIX_ARM_GATE pin$(RESET)"; \
	    exit 0; \
	  fi; \
	  sleep 2; \
	done; \
	echo "$(RED)→ Backend health check failed — verify port / logs$(RESET)"; exit 1

bench001-smoke-fast-large: bench001-install ## smoke-fast with mistral-large SUT+judge, high eval parallelism
	@set -a && [ -f "$(DEV_PORTS_ENV)" ] && . "$(DEV_PORTS_ENV)"; set +a; \
	export EDGEQUAKE_API_URL="$${EDGEQUAKE_API_URL:-$(BACKEND_URL)}"; \
	export EDGEQUAKE_MIX_ARM_GATE="$(BENCH001_EQ_MIX_ARM_GATE)"; \
	export BENCH001_LLM_PROVIDER=mistral; \
	export BENCH001_LLM_MODEL="$${BENCH001_LLM_MODEL:-mistral-large-latest}"; \
	export BENCH001_JUDGE_PROVIDER=mistral; \
	export BENCH001_JUDGE_MODEL="$${BENCH001_JUDGE_MODEL:-mistral-large-latest}"; \
	export BENCH001_ANSWER_STYLE="$${BENCH001_ANSWER_STYLE:-gold}"; \
	export BENCH001_PUBLISH_FAIRNESS=1; \
	export BENCH001_QUERY_CONCURRENCY="$(BENCH001_LARGE_QUERY_CONCURRENCY)"; \
	export BENCH001_LR_QUERY_CONCURRENCY="$${BENCH001_LR_QUERY_CONCURRENCY:-1}"; \
	export BENCH001_EVAL_CONCURRENCY="$(BENCH001_LARGE_EVAL_CONCURRENCY)"; \
	if [ -n "$${BENCH001_EQ_WORKSPACE_ID:-}" ]; then export BENCH001_EQ_WORKSPACE_ID; fi; \
	export LLM_API_KEY="$${LLM_API_KEY:-$$MISTRAL_API_KEY}"; \
	export PYTHONPATH="tools/bench001:$${PYTHONPATH}"; \
	python3 -m bench001.cli smoke-fast --api "$$EDGEQUAKE_API_URL" --query-only \
	  --llm-provider mistral --llm-model "$$BENCH001_LLM_MODEL" \
	  --judge-provider mistral --judge-model "$$BENCH001_JUDGE_MODEL" \
	  --answer-style "$$BENCH001_ANSWER_STYLE" \
	  --profile-id P0_mistral_large_mix_v2 \
	  --query-concurrency "$(BENCH001_LARGE_QUERY_CONCURRENCY)" \
	  --eval-concurrency "$(BENCH001_LARGE_EVAL_CONCURRENCY)"; \
	echo "$(GREEN)→ SUMMARY:$(RESET) specs/001-benchmark/e2e/artifacts/smoke-fast/SUMMARY.md"

bench001-smoke-fast-acc: bench001-install bench001-acc-backend ## Acc-lift smoke-fast: gold + small + LR-like Mix + fair chunk 1200
	@set -a && [ -f "$(DEV_PORTS_ENV)" ] && . "$(DEV_PORTS_ENV)"; set +a; \
	export EDGEQUAKE_API_URL="$${EDGEQUAKE_API_URL:-$(BACKEND_URL)}"; \
	export EDGEQUAKE_MIX_ARM_GATE="$(BENCH001_EQ_MIX_ARM_GATE)"; \
	export EDGEQUAKE_ADAPTIVE_CHUNKING="$(BENCH001_EQ_ADAPTIVE_CHUNKING)"; \
	export EDGEQUAKE_CHUNK_SIZE="$(BENCH001_EQ_CHUNK_SIZE)"; \
	export EDGEQUAKE_CHUNK_OVERLAP="$(BENCH001_EQ_CHUNK_OVERLAP)"; \
	export EDGEQUAKE_MIX_FUSION="$${EDGEQUAKE_MIX_FUSION:-round_robin}"; \
	export BENCH001_ALLOW_ROUND_ROBIN="$${BENCH001_ALLOW_ROUND_ROBIN:-1}"; \
	export BENCH001_INGEST_MAX_CHARS="$${BENCH001_INGEST_MAX_CHARS:-$(BENCH001_INGEST_MAX_CHARS)}"; \
	export BENCH001_INGEST_TIMEOUT_S="$${BENCH001_INGEST_TIMEOUT_S:-$(BENCH001_INGEST_TIMEOUT_S)}"; \
	export BENCH001_LLM_PROVIDER=mistral; \
	export BENCH001_LLM_MODEL="$(BENCH001_ACC_LLM_MODEL)"; \
	export BENCH001_JUDGE_PROVIDER=mistral; \
	export BENCH001_JUDGE_MODEL="$(BENCH001_ACC_JUDGE_MODEL)"; \
	export BENCH001_ANSWER_STYLE=gold; \
	export BENCH001_PUBLISH_FAIRNESS=1; \
	export BENCH001_QUERY_CONCURRENCY="$(BENCH001_ACC_QUERY_CONCURRENCY)"; \
	export BENCH001_LR_QUERY_CONCURRENCY="$${BENCH001_LR_QUERY_CONCURRENCY:-1}"; \
	export BENCH001_EVAL_CONCURRENCY="$(BENCH001_ACC_EVAL_CONCURRENCY)"; \
	if [ -n "$${BENCH001_EQ_WORKSPACE_ID:-}" ]; then export BENCH001_EQ_WORKSPACE_ID; fi; \
	case "$${LLM_API_KEY:-}" in FAKE*|fake*) unset LLM_API_KEY ;; esac; \
	case "$${MISTRAL_API_KEY:-}" in FAKE*|fake*) unset MISTRAL_API_KEY ;; esac; \
	export LLM_API_KEY="$${LLM_API_KEY:-$$MISTRAL_API_KEY}"; \
	export PYTHONPATH="tools/bench001:$${PYTHONPATH}"; \
	export PYTHONUNBUFFERED=1; \
	_QONLY_FLAG="--query-only"; \
	if [ "$${BENCH001_FORCE_INGEST:-0}" = "1" ]; then _QONLY_FLAG="--force-ingest"; fi; \
	echo "$(YELLOW)→ Acc smoke-fast: api=$$EDGEQUAKE_API_URL model=$(BENCH001_ACC_LLM_MODEL) ingest_max_chars=$$BENCH001_INGEST_MAX_CHARS force=$${BENCH001_FORCE_INGEST:-0}$(RESET)"; \
	python3 -m bench001.cli smoke-fast --api "$$EDGEQUAKE_API_URL" $$_QONLY_FLAG \
	  --llm-provider mistral --llm-model "$(BENCH001_ACC_LLM_MODEL)" \
	  --vision-provider mistral --vision-model "$(BENCH001_ACC_LLM_MODEL)" \
	  --embedding-provider mistral --embedding-model mistral-embed --embedding-dim 1024 \
	  --judge-provider mistral --judge-model "$(BENCH001_ACC_JUDGE_MODEL)" \
	  --judge-embedding-model mistral-embed \
	  --answer-style gold \
	  --profile-id P0_mistral_small_mix_chunk1200_v1 \
	  --query-concurrency "$(BENCH001_ACC_QUERY_CONCURRENCY)" \
	  --eval-concurrency "$(BENCH001_ACC_EVAL_CONCURRENCY)"; \
	echo "$(GREEN)→ SUMMARY:$(RESET) specs/001-benchmark/e2e/artifacts/smoke-fast/SUMMARY.md"

bench001-smoke-acc: bench001-install bench001-acc-backend ## Acc-lift smoke n=40: gold + small + 086 E2-occ Mix + fair chunk 1200
	@set -a && [ -f "$(DEV_PORTS_ENV)" ] && . "$(DEV_PORTS_ENV)"; set +a; \
	export EDGEQUAKE_API_URL="$${EDGEQUAKE_API_URL:-$(BACKEND_URL)}"; \
	export BENCH001_PUBLICATION=1; \
	export BENCH001_FULL_ACC=1; \
	export EDGEQUAKE_MIX_ARM_GATE="$(BENCH001_EQ_MIX_ARM_GATE)"; \
	export EDGEQUAKE_ADAPTIVE_CHUNKING=0; \
	export EDGEQUAKE_CHUNK_SIZE=1200; \
	export EDGEQUAKE_CHUNK_OVERLAP=100; \
	export EDGEQUAKE_MIX_FUSION=round_robin; \
	export EDGEQUAKE_HYBRID_FUSION=round_robin; \
	export BENCH001_ALLOW_ROUND_ROBIN=1; \
	export BENCH001_EQ_ENABLE_RERANK=0; \
	export EDGEQUAKE_GRAPH_WALK=bfs; \
	export EDGEQUAKE_ENTITY_RANK=retrieval; \
	export EDGEQUAKE_KG_CHUNK_PICK_LR_BUDGET=1; \
	export EDGEQUAKE_KG_CHUNK_OCCURRENCE_SORT=1; \
	export EDGEQUAKE_L2_BM25_UNION=1; \
	export EDGEQUAKE_L2_BM25_MODE=fact_replace; \
	export EDGEQUAKE_LLM_PROVIDER=mistral; \
	export EDGEQUAKE_LLM_MODEL="$(BENCH001_ACC_LLM_MODEL)"; \
	export EDGEQUAKE_VISION_PROVIDER=mistral; \
	export EDGEQUAKE_VISION_MODEL="$(BENCH001_ACC_LLM_MODEL)"; \
	export EDGEQUAKE_EMBEDDING_PROVIDER=mistral; \
	export EDGEQUAKE_EMBEDDING_MODEL=mistral-embed; \
	export MISTRAL_MODEL="$(BENCH001_ACC_LLM_MODEL)"; \
	export MISTRAL_EMBEDDING_MODEL=mistral-embed; \
	export BENCH001_LLM_PROVIDER=mistral; \
	export BENCH001_LLM_MODEL="$(BENCH001_ACC_LLM_MODEL)"; \
	export BENCH001_VISION_PROVIDER=mistral; \
	export BENCH001_VISION_MODEL="$(BENCH001_ACC_LLM_MODEL)"; \
	export BENCH001_EMBEDDING_PROVIDER=mistral; \
	export BENCH001_EMBEDDING_MODEL=mistral-embed; \
	export BENCH001_EMBEDDING_DIM=1024; \
	export BENCH001_JUDGE_PROVIDER=mistral; \
	export BENCH001_JUDGE_MODEL="$(BENCH001_ACC_JUDGE_MODEL)"; \
	export BENCH001_JUDGE_EMBEDDING_MODEL=mistral-embed; \
	export BENCH001_ANSWER_STYLE=gold; \
	export BENCH001_PUBLISH_FAIRNESS=1; \
	export BENCH001_QUERY_CONCURRENCY="$(BENCH001_ACC_QUERY_CONCURRENCY)"; \
	export BENCH001_LR_QUERY_CONCURRENCY="$${BENCH001_LR_QUERY_CONCURRENCY:-1}"; \
	export BENCH001_EVAL_CONCURRENCY="$(BENCH001_ACC_EVAL_CONCURRENCY)"; \
	export BENCH001_INGEST_MAX_CHARS=0; \
	export BENCH001_INGEST_TIMEOUT_S="$${BENCH001_INGEST_TIMEOUT_S:-7200}"; \
	case "$${BENCH001_EQ_WORKSPACE_ID:-}" in *c100000*|*c10000*) unset BENCH001_EQ_WORKSPACE_ID ;; esac; \
	case "$${LLM_API_KEY:-}" in FAKE*|fake*) unset LLM_API_KEY ;; esac; \
	case "$${MISTRAL_API_KEY:-}" in FAKE*|fake*) unset MISTRAL_API_KEY ;; esac; \
	export LLM_API_KEY="$${LLM_API_KEY:-$$MISTRAL_API_KEY}"; \
	export PYTHONPATH="tools/bench001:$${PYTHONPATH}"; \
	export PYTHONUNBUFFERED=1; \
	_QONLY_FLAG="--force-ingest"; \
	if [ "$${BENCH001_QUERY_ONLY:-0}" = "1" ]; then _QONLY_FLAG="--query-only"; fi; \
	echo "$(YELLOW)→ Acc smoke n=40 PUBLICATION: api=$$EDGEQUAKE_API_URL$(RESET)"; \
	echo "$(YELLOW)  llm/vision/judge=$(BENCH001_ACC_LLM_MODEL) embed=mistral-embed chunk=1200/100 corpus=FULL$(RESET)"; \
	echo "$(BLUE)  monitor: make bench001-watch STAGE=smoke$(RESET)"; \
	python3 -m bench001.cli smoke --api "$$EDGEQUAKE_API_URL" $$_QONLY_FLAG \
	  --llm-provider mistral --llm-model "$(BENCH001_ACC_LLM_MODEL)" \
	  --vision-provider mistral --vision-model "$(BENCH001_ACC_LLM_MODEL)" \
	  --embedding-provider mistral --embedding-model mistral-embed --embedding-dim 1024 \
	  --judge-provider mistral --judge-model "$(BENCH001_ACC_JUDGE_MODEL)" \
	  --judge-embedding-model mistral-embed \
	  --answer-style gold \
	  --profile-id ACC_E2OCC_086_v1 \
	  --query-concurrency "$(BENCH001_ACC_QUERY_CONCURRENCY)" \
	  --eval-concurrency "$(BENCH001_ACC_EVAL_CONCURRENCY)"; \
	echo "$(GREEN)→ SUMMARY:$(RESET) specs/001-benchmark/e2e/artifacts/smoke/SUMMARY.md"

bench001-medical-mid: bench001-install bench001-acc-backend ## Acc medical-mid n=200 (086 E2-occ Acc law; SKIP publish/latest unless ALLOW)
	@set -a && [ -f "$(DEV_PORTS_ENV)" ] && . "$(DEV_PORTS_ENV)"; set +a; \
	export EDGEQUAKE_API_URL="$${EDGEQUAKE_API_URL:-$(BACKEND_URL)}"; \
	export BENCH001_PUBLICATION=1; \
	export BENCH001_FULL_ACC=1; \
	export EDGEQUAKE_MIX_ARM_GATE="$(BENCH001_EQ_MIX_ARM_GATE)"; \
	export EDGEQUAKE_ADAPTIVE_CHUNKING=0; \
	export EDGEQUAKE_CHUNK_SIZE="$${BENCH001_EQ_CHUNK_SIZE:-1200}"; \
	export EDGEQUAKE_CHUNK_OVERLAP="$${BENCH001_EQ_CHUNK_OVERLAP:-100}"; \
	export EDGEQUAKE_MIX_FUSION=round_robin; \
	export EDGEQUAKE_HYBRID_FUSION=round_robin; \
	export BENCH001_ALLOW_ROUND_ROBIN=1; \
	export BENCH001_EQ_ENABLE_RERANK=0; \
	export EDGEQUAKE_GRAPH_WALK=bfs; \
	export EDGEQUAKE_ENTITY_RANK=retrieval; \
	export EDGEQUAKE_KG_CHUNK_PICK=vector; \
	export EDGEQUAKE_KG_CHUNK_PICK_LR_BUDGET=1; \
	export EDGEQUAKE_KG_CHUNK_OCCURRENCE_SORT=1; \
	export EDGEQUAKE_BM25_RETRIEVAL=1; \
	export EDGEQUAKE_L2_BM25_UNION=1; \
	export EDGEQUAKE_L2_BM25_MODE=fact_replace; \
	export EDGEQUAKE_L2_BM25_MIX_TOP_K=30; \
	if [ "$${BENCH001_ALLOW_PUBLISH_LATEST:-0}" != "1" ]; then export BENCH001_SKIP_PUBLISH_LATEST=1; fi; \
	export EDGEQUAKE_LLM_PROVIDER=mistral; \
	export EDGEQUAKE_LLM_MODEL="$(BENCH001_ACC_LLM_MODEL)"; \
	export EDGEQUAKE_VISION_PROVIDER=mistral; \
	export EDGEQUAKE_VISION_MODEL="$(BENCH001_ACC_LLM_MODEL)"; \
	export EDGEQUAKE_EMBEDDING_PROVIDER=mistral; \
	export EDGEQUAKE_EMBEDDING_MODEL=mistral-embed; \
	export MISTRAL_MODEL="$(BENCH001_ACC_LLM_MODEL)"; \
	export MISTRAL_EMBEDDING_MODEL=mistral-embed; \
	export BENCH001_LLM_PROVIDER=mistral; \
	export BENCH001_LLM_MODEL="$(BENCH001_ACC_LLM_MODEL)"; \
	export BENCH001_VISION_PROVIDER=mistral; \
	export BENCH001_VISION_MODEL="$(BENCH001_ACC_LLM_MODEL)"; \
	export BENCH001_EMBEDDING_PROVIDER=mistral; \
	export BENCH001_EMBEDDING_MODEL=mistral-embed; \
	export BENCH001_EMBEDDING_DIM=1024; \
	export BENCH001_JUDGE_PROVIDER=mistral; \
	export BENCH001_JUDGE_MODEL="$(BENCH001_ACC_JUDGE_MODEL)"; \
	export BENCH001_JUDGE_EMBEDDING_MODEL=mistral-embed; \
	export BENCH001_ANSWER_STYLE=gold; \
	export BENCH001_PUBLISH_FAIRNESS=1; \
	export BENCH001_QUERY_CONCURRENCY="$(BENCH001_ACC_QUERY_CONCURRENCY)"; \
	export BENCH001_LR_QUERY_CONCURRENCY="$${BENCH001_LR_QUERY_CONCURRENCY:-1}"; \
	export BENCH001_EVAL_CONCURRENCY="$(BENCH001_ACC_EVAL_CONCURRENCY)"; \
	export BENCH001_INGEST_MAX_CHARS=0; \
	export BENCH001_INGEST_TIMEOUT_S="$${BENCH001_INGEST_TIMEOUT_S:-7200}"; \
	case "$${BENCH001_EQ_WORKSPACE_ID:-}" in *c100000*|*c10000*) unset BENCH001_EQ_WORKSPACE_ID ;; esac; \
	case "$${LLM_API_KEY:-}" in FAKE*|fake*) unset LLM_API_KEY ;; esac; \
	case "$${MISTRAL_API_KEY:-}" in FAKE*|fake*) unset MISTRAL_API_KEY ;; esac; \
	export LLM_API_KEY="$${LLM_API_KEY:-$$MISTRAL_API_KEY}"; \
	export PYTHONPATH="tools/bench001:$${PYTHONPATH}"; \
	export PYTHONUNBUFFERED=1; \
	_QONLY_FLAG="--force-ingest"; \
	if [ "$${BENCH001_QUERY_ONLY:-0}" = "1" ]; then _QONLY_FLAG="--query-only"; fi; \
	echo "$(YELLOW)→ Acc medical-mid n=200 PUBLICATION (086 E2-occ): api=$$EDGEQUAKE_API_URL$(RESET)"; \
	echo "$(YELLOW)  llm/vision/judge=$(BENCH001_ACC_LLM_MODEL) embed=mistral-embed chunk=1200/100 corpus=FULL$(RESET)"; \
	echo "$(YELLOW)  mix=round_robin · rerank=0 · bfs · occ_sort · LR_BUDGET · Fact L2 fact_replace$(RESET)"; \
	if [ "$${BENCH001_SKIP_PUBLISH_LATEST:-0}" = "1" ]; then echo "$(YELLOW)  SKIP publish/latest (set BENCH001_ALLOW_PUBLISH_LATEST=1 to replace)$(RESET)"; fi; \
	echo "$(BLUE)  monitor: make bench001-watch STAGE=medical-mid$(RESET)"; \
	python3 -m bench001.cli medical-mid --api "$$EDGEQUAKE_API_URL" $$_QONLY_FLAG \
	  --llm-provider mistral --llm-model "$(BENCH001_ACC_LLM_MODEL)" \
	  --vision-provider mistral --vision-model "$(BENCH001_ACC_LLM_MODEL)" \
	  --embedding-provider mistral --embedding-model mistral-embed --embedding-dim 1024 \
	  --judge-provider mistral --judge-model "$(BENCH001_ACC_JUDGE_MODEL)" \
	  --judge-embedding-model mistral-embed \
	  --answer-style gold \
	  --profile-id ACC_E2OCC_086_v1 \
	  --query-concurrency "$(BENCH001_ACC_QUERY_CONCURRENCY)" \
	  --eval-concurrency "$(BENCH001_ACC_EVAL_CONCURRENCY)"; \
	echo "$(GREEN)→ SUMMARY:$(RESET) specs/001-benchmark/e2e/artifacts/medical-mid/SUMMARY.md"

# SPEC-086 Phase A: labeled medical-mid under Acc E2-occ law; never overwrite publish/latest.
bench001-086-phase-a: ## 086 Phase A: Acc-law E2-occ medical-mid (query-only; SKIP publish/latest)
	@set -e; \
	if [ -z "$${BENCH001_EQ_WORKSPACE_ID:-}" ]; then \
	  BENCH001_EQ_WORKSPACE_ID="$$(cd tools/bench001 && PYTHONPATH=. python3 -m bench001.cli resolve-warm-workspace)"; \
	  echo "$(GREEN)086-phase-a: warm workspace $${BENCH001_EQ_WORKSPACE_ID}$(RESET)"; \
	fi; \
	export BENCH001_EQ_WORKSPACE_ID; \
	export BENCH001_QUERY_ONLY=1; \
	export BENCH001_SKIP_PUBLISH_LATEST=1; \
	export BENCH001_PUBLISH_PEER="$${BENCH001_PUBLISH_PEER:-ACC_E2OCC_086_v1}"; \
	export BENCH001_ALLOW_PUBLISH_LATEST=0; \
	$(MAKE) bench001-medical-mid --no-print-directory; \
	echo "$(GREEN)→ Phase A peer: specs/001-benchmark/e2e/artifacts/publish/peers/ACC_E2OCC_086_v1/ (if published)$(RESET)"; \
	echo "$(GREEN)→ Gates: ctx≥0.48 · Fact ER≥0.90 · Acc CI not LR-ahead$(RESET)"

# ---------------------------------------------------------------------------
# Primary stakeholder entry: make bench
# Fair GraphRAG-Bench Acc (EQ Mix vs LightRAG Mix, n=200 medical-mid) + business publish pack.
# SPEC-086: skips publish/latest unless BENCH001_ALLOW_PUBLISH_LATEST=1 (Beat promote only).
# Chain: install → Acc backend → doctor → medical-mid → BUSINESS_REPORT in publish/latest/
# Mandatory local pre-tag gate (not in release_gates.sh / CI) — see docs/operations/release-and-cd.md
# ---------------------------------------------------------------------------
bench: bench001-install ## Acc dual-SUT n=200 (086 E2-occ); publish/latest only if ALLOW_PUBLISH_LATEST=1
	@echo "$(BLUE)→ make bench: Acc backend → doctor → dual-SUT Acc (n=200 medical-mid)$(RESET)"
	@$(MAKE) bench001-acc-backend --no-print-directory
	@set -a && [ -f "$(DEV_PORTS_ENV)" ] && . "$(DEV_PORTS_ENV)"; set +a; \
	export EDGEQUAKE_API_URL="$${EDGEQUAKE_API_URL:-$(BACKEND_URL)}"; \
	export PYTHONPATH="tools/bench001:$${PYTHONPATH}"; \
	python3 -m bench001.cli doctor --api "$$EDGEQUAKE_API_URL" || exit 1
	@BENCH001_SKIP_BACKEND_RESTART=1 $(MAKE) bench001-medical-mid --no-print-directory
	@echo ""
	@echo "$(GREEN)✓ Bench finished$(RESET)"
	@echo "$(GREEN)  Technical:$(RESET) specs/001-benchmark/e2e/artifacts/medical-mid/SUMMARY.md"
	@echo "$(GREEN)  Business:$(RESET)  specs/001-benchmark/e2e/artifacts/publish/latest/BUSINESS_REPORT.md"
	@echo "$(GREEN)  Exec blurb:$(RESET) specs/001-benchmark/e2e/artifacts/publish/latest/EXEC_SUMMARY.txt"
	@if [ -f specs/001-benchmark/e2e/artifacts/publish/latest/EXEC_SUMMARY.txt ]; then \
	  echo ""; \
	  cat specs/001-benchmark/e2e/artifacts/publish/latest/EXEC_SUMMARY.txt; \
	fi

bench-warm: ## Same as make bench but query-only (defaults to latest warm EQ workspace)
	@set -e; \
	if [ -z "$${BENCH001_EQ_WORKSPACE_ID:-}" ]; then \
	  BENCH001_EQ_WORKSPACE_ID="$$(cd tools/bench001 && PYTHONPATH=. python3 -m bench001.cli resolve-warm-workspace)"; \
	  echo "$(GREEN)bench-warm: using warm workspace $${BENCH001_EQ_WORKSPACE_ID}$(RESET)"; \
	else \
	  echo "$(GREEN)bench-warm: using BENCH001_EQ_WORKSPACE_ID=$${BENCH001_EQ_WORKSPACE_ID}$(RESET)"; \
	fi; \
	export BENCH001_EQ_WORKSPACE_ID; \
	export BENCH001_QUERY_ONLY=1; \
	$(MAKE) bench --no-print-directory

# SPEC-103 labeled latency peer: Acc law + EQ response cache ON (not Acc Beat / not publish/latest).
# Pass 1 fills public.llm_cache; pass 2 measures warm EQ vs warm LR (BENCH001_LR_ENABLE_LLM_CACHE default 1).
bench001-medical-mid-eq-llm-cache-warm: ## SPEC-103: medical-mid fill+warm with EDGEQUAKE_LLM_CACHE=1 (SKIP publish/latest)
	@set -e; \
	export EDGEQUAKE_LLM_CACHE=1; \
	if [ -z "$${BENCH001_EQ_WORKSPACE_ID:-}" ]; then \
	  BENCH001_EQ_WORKSPACE_ID="$$(cd tools/bench001 && PYTHONPATH=. python3 -m bench001.cli resolve-warm-workspace)"; \
	  echo "$(GREEN)eq-llm-cache-warm: workspace $${BENCH001_EQ_WORKSPACE_ID}$(RESET)"; \
	fi; \
	export BENCH001_EQ_WORKSPACE_ID; \
	echo "$(YELLOW)→ Acc backend with EDGEQUAKE_LLM_CACHE=1 (labeled peer; Acc headline stays CACHE=0)$(RESET)"; \
	$(MAKE) bench001-acc-backend --no-print-directory; \
	grep -q 'EDGEQUAKE_LLM_CACHE="1"' /tmp/edgequake-start.sh \
	  || { echo "$(RED)→ start.sh did not pin EDGEQUAKE_LLM_CACHE=1$(RESET)"; exit 1; }; \
	export BENCH001_SKIP_BACKEND_RESTART=1; \
	export BENCH001_QUERY_ONLY=1; \
	export BENCH001_SKIP_PUBLISH_LATEST=1; \
	export BENCH001_ALLOW_PUBLISH_LATEST=0; \
	export BENCH001_LR_ENABLE_LLM_CACHE="$${BENCH001_LR_ENABLE_LLM_CACHE:-1}"; \
	echo "$(BLUE)→ Pass 1/2: fill EQ llm_cache (no peer publish)$(RESET)"; \
	unset BENCH001_PUBLISH_PEER; \
	$(MAKE) bench001-medical-mid --no-print-directory; \
	echo "$(BLUE)→ Pass 2/2: warm measure → peer EQ_LLM_CACHE_WARM_v1$(RESET)"; \
	export BENCH001_PUBLISH_PEER="$${BENCH001_PUBLISH_PEER:-EQ_LLM_CACHE_WARM_v1}"; \
	$(MAKE) bench001-medical-mid --no-print-directory; \
	echo "$(GREEN)→ Peer: specs/001-benchmark/e2e/artifacts/publish/peers/EQ_LLM_CACHE_WARM_v1/$(RESET)"; \
	echo "$(GREEN)→ SUMMARY: specs/001-benchmark/e2e/artifacts/medical-mid/SUMMARY.md$(RESET)"

# Legacy aliases → make bench
bench001-full: bench ## Alias: full Acc benchmark (n=200 medical-mid; mistral-small + mistral-embed)
bench001: bench ## Alias: launch publish Acc benchmark (n=200 medical-mid)

# 021 F1–F4 labeled Acc ladder (requires BENCH001_EQ_WORKSPACE_ID + DASHSCOPE for S1 CE).
bench001-f1a: ## 021 F1a: S1 CE+protect + Summarize truncation floor (warm query-only)
	@chmod +x tools/bench001/scripts/run_f_ladder_acc.sh
	@./tools/bench001/scripts/run_f_ladder_acc.sh f1a

bench001-f2a: ## 021 F2a: path_pack_v1 (CONTEXT_FORMAT=path + soft path prune)
	@chmod +x tools/bench001/scripts/run_f_ladder_acc.sh
	@./tools/bench001/scripts/run_f_ladder_acc.sh f2a

bench001-f3a: ## 021 F3a: latency stage timing + arm concurrency remeasure
	@chmod +x tools/bench001/scripts/run_f_ladder_acc.sh
	@./tools/bench001/scripts/run_f_ladder_acc.sh f3a

bench001-f4a: ## 021 F4a: labeled PASSAGE_PACK=1 (HippoRAG2-style chunks-first)
	@chmod +x tools/bench001/scripts/run_f_ladder_acc.sh
	@./tools/bench001/scripts/run_f_ladder_acc.sh f4a

# 022 P0–P5 Acc recovery ladder (warm query-only; auto-resolves warm workspace).
bench001-p0: ## 022 P0: PATH_PRUNE=0 BM25 Acc restore
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@./tools/bench001/scripts/run_p_ladder_acc.sh p0

bench001-p1a: ## 022 P1a: graph-walk compress on BM25
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@./tools/bench001/scripts/run_p_ladder_acc.sh p1a

bench001-p1b: ## 022 P1b: graph-walk compress on S1 CE+protect (needs DASHSCOPE)
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@./tools/bench001/scripts/run_p_ladder_acc.sh p1b

bench001-p2a: ## 022 P2a: MIX_FUSION=round_robin Acc ablation
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@./tools/bench001/scripts/run_p_ladder_acc.sh p2a

bench001-p2b: ## 022 P2b: lr_pack_v1 on S1 (retrieval + path + headings)
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@./tools/bench001/scripts/run_p_ladder_acc.sh p2b

bench001-p3a: ## 022 P3a: intent truncation audit (query_intent on predictions)
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@./tools/bench001/scripts/run_p_ladder_acc.sh p3a

bench001-p3b: ## 022 P3b: keyword lexical boost + popular-node fallback off
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@./tools/bench001/scripts/run_p_ladder_acc.sh p3b

bench001-p4: ## 022 P4: Acc CI decision package (promote only if gates green)
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@./tools/bench001/scripts/run_p_ladder_acc.sh p4

bench001-p5: ## 022 P5: latency arm concurrency 24 + stage remeasure
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@./tools/bench001/scripts/run_p_ladder_acc.sh p5

# 024 Acc parity / beat ladder (warm query-only).
bench001-q0: ## 024 Q0: P2b stability Acc
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@./tools/bench001/scripts/run_p_ladder_acc.sh q0

bench001-q1: ## 024 Q1: occurrence-sort on P0 BM25
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@./tools/bench001/scripts/run_p_ladder_acc.sh q1

bench001-q2: ## 024 Q2: VECTOR LR budget on P0 BM25
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@./tools/bench001/scripts/run_p_ladder_acc.sh q2

bench001-q3: ## 024 Q3: Fact winner on P2b (BENCH001_Q3_FACT_KNOB=occurrence|lr_budget)
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@./tools/bench001/scripts/run_p_ladder_acc.sh q3

bench001-q4: ## 024 Q4: Acc CI decision (BENCH001_Q4_PACKAGE=p2b|occurrence|lr_budget)
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@./tools/bench001/scripts/run_p_ladder_acc.sh q4

bench001-r0: ## 025 R0: P2b + PROTECT_FIRST=20 (CE recall)
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@./tools/bench001/scripts/run_p_ladder_acc.sh r0

bench001-r1: ## 025 R1: P2b + MIN_RERANK_SCORE=0
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@./tools/bench001/scripts/run_p_ladder_acc.sh r1

bench001-r2: ## 025 R2: P2b + MIN_CHUNK_BUDGET_RATIO=0.55
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@./tools/bench001/scripts/run_p_ladder_acc.sh r2

bench001-r3: ## 025 R3: Acc CI decision (BENCH001_R3_PACKAGE=protect20|min_rerank0|chunk055|p2b)
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@./tools/bench001/scripts/run_p_ladder_acc.sh r3

bench001-s0: ## 026 S0: P2b + L2_SOURCES_UNION=1 (Mix∪CE citations)
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@./tools/bench001/scripts/run_p_ladder_acc.sh s0

bench001-s1: ## 026 S1: Acc CI decision (BENCH001_S1_PACKAGE=union|p2b)
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@./tools/bench001/scripts/run_p_ladder_acc.sh s1

bench001-t0: ## 027 T0: P2b + Fact→BM25 on prompt (Acc tax; prefer t0b)
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@./tools/bench001/scripts/run_p_ladder_acc.sh t0

bench001-t0b: ## 027 T0b: P2b + L2 BM25-first ∪ CE (CE prompt)
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@./tools/bench001/scripts/run_p_ladder_acc.sh t0b

bench001-t0c: ## 027 T0c: P2b + L2 BM25 replace sources (CE prompt)
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@./tools/bench001/scripts/run_p_ladder_acc.sh t0c

bench001-t0d: ## 027 T0d: P2b + L2 FactReplace (Fact BM25 L2; else CE)
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@./tools/bench001/scripts/run_p_ladder_acc.sh t0d

bench001-t1: ## 027 T1: Acc CI (BENCH001_T1_PACKAGE=fact_replace|replace|union|p2b)
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@./tools/bench001/scripts/run_p_ladder_acc.sh t1

bench001-a0: ## 028 A0: P2b baseline Acc
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@./tools/bench001/scripts/run_p_ladder_acc.sh a0

bench001-a1: ## 028 A1: P2b + CONTEXT_FORMAT=rr_cer (relation-first)
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@./tools/bench001/scripts/run_p_ladder_acc.sh a1

bench001-lr-identity: ## 074 LR-identity: RR fuse · rerank off · VECTOR+LR budget · retrieval rank (not Acc Beat)
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@./tools/bench001/scripts/run_p_ladder_acc.sh lr-identity

bench001-lr-pack-bm25: ## 075 L1: LR packing + BM25 on (Fact ER recovery; not Acc Beat)
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@./tools/bench001/scripts/run_p_ladder_acc.sh lr-pack-bm25

bench001-lr-identity-fact-l2: ## 075 L1.5: L0 identity + Fact BM25 L2 citations (not Acc Beat)
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@./tools/bench001/scripts/run_p_ladder_acc.sh lr-identity-fact-l2

bench001-medical-mid-lr-identity-fact-l2: ## 076 medical-mid n=200 peer under LR_IDENTITY_FACT_L2_v1 (not Acc headline)
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@BENCH001_LADDER_STAGE=medical-mid \
		BENCH001_QUERY_ONLY=1 \
		BENCH001_SKIP_PUBLISH_LATEST=1 \
		BENCH001_PUBLISH_PEER=LR_IDENTITY_FACT_L2_v1 \
		./tools/bench001/scripts/run_p_ladder_acc.sh lr-identity-fact-l2

bench001-lr-nf-fact-l2: ## 076 Phase4: L1.5 + naive-first RR (smoke; not Acc Beat)
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@./tools/bench001/scripts/run_p_ladder_acc.sh lr-nf-fact-l2

bench001-medical-mid-lr-nf-fact-l2: ## 076 medical-mid peer under LR_NF_FACT_L2_v1 (not Acc headline)
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@BENCH001_LADDER_STAGE=medical-mid \
		BENCH001_QUERY_ONLY=1 \
		BENCH001_SKIP_PUBLISH_LATEST=1 \
		BENCH001_PUBLISH_PEER=LR_NF_FACT_L2_v1 \
		./tools/bench001/scripts/run_p_ladder_acc.sh lr-nf-fact-l2

bench001-lr-dense-fact-l2: ## 077 E1: L1.5 + dense-only Mix arms (BM25_RETRIEVAL=0; not Acc Beat)
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@./tools/bench001/scripts/run_p_ladder_acc.sh lr-dense-fact-l2

bench001-medical-mid-lr-dense-fact-l2: ## 077 E1 medical-mid peer under LR_DENSE_FACT_L2_v1 (not Acc headline)
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@BENCH001_LADDER_STAGE=medical-mid \
		BENCH001_QUERY_ONLY=1 \
		BENCH001_SKIP_PUBLISH_LATEST=1 \
		BENCH001_PUBLISH_PEER=LR_DENSE_FACT_L2_v1 \
		./tools/bench001/scripts/run_p_ladder_acc.sh lr-dense-fact-l2

bench001-lr-occ-fact-l2: ## 077 E2: L1.5 + occurrence_sort (smoke; not Acc Beat)
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@./tools/bench001/scripts/run_p_ladder_acc.sh lr-occ-fact-l2

bench001-medical-mid-lr-occ-fact-l2: ## 077 E2 medical-mid peer under LR_OCC_FACT_L2_v1 (not Acc headline)
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@BENCH001_LADDER_STAGE=medical-mid \
		BENCH001_QUERY_ONLY=1 \
		BENCH001_SKIP_PUBLISH_LATEST=1 \
		BENCH001_PUBLISH_PEER=LR_OCC_FACT_L2_v1 \
		./tools/bench001/scripts/run_p_ladder_acc.sh lr-occ-fact-l2

bench001-lr-posttrunc-fact-l2: ## 078 R3: E2 + post_truncate KG pick (smoke; not Acc Beat)
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@./tools/bench001/scripts/run_p_ladder_acc.sh lr-posttrunc-fact-l2

bench001-medical-mid-lr-posttrunc-fact-l2: ## 078 R3 medical-mid peer under LR_POSTTRUNC_FACT_L2_v1 (not Acc headline)
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@BENCH001_LADDER_STAGE=medical-mid \
		BENCH001_QUERY_ONLY=1 \
		BENCH001_SKIP_PUBLISH_LATEST=1 \
		BENCH001_PUBLISH_PEER=LR_POSTTRUNC_FACT_L2_v1 \
		./tools/bench001/scripts/run_p_ladder_acc.sh lr-posttrunc-fact-l2

bench001-medical-full-lr-occ-fact-l2: ## 079 medical-full n≈2062 E2 keep peer (not Acc headline)
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@BENCH001_LADDER_STAGE=medical-full \
		BENCH001_QUERY_ONLY=1 \
		BENCH001_SKIP_PUBLISH_LATEST=1 \
		BENCH001_PUBLISH_PEER=LR_OCC_FACT_L2_medical_full_v1 \
		./tools/bench001/scripts/run_p_ladder_acc.sh lr-occ-fact-l2

bench001-medical-full-p0: ## 079 medical-full n≈2062 P0 pack peer (not Acc headline / skip publish/latest)
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@BENCH001_LADDER_STAGE=medical-full \
		BENCH001_QUERY_ONLY=1 \
		BENCH001_SKIP_PUBLISH_LATEST=1 \
		BENCH001_PUBLISH_PEER=P0_MEDICAL_FULL_v1 \
		./tools/bench001/scripts/run_p_ladder_acc.sh p0

bench001-lr-unify-fact-l2: ## 080 D1 R6: E2 + Acc/L2 unify (smoke; not Acc Beat)
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@./tools/bench001/scripts/run_p_ladder_acc.sh lr-unify-fact-l2

bench001-medical-mid-lr-unify-fact-l2: ## 080 D1 medical-mid peer under LR_UNIFY_FACT_L2_v1
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@BENCH001_LADDER_STAGE=medical-mid \
		BENCH001_QUERY_ONLY=1 \
		BENCH001_SKIP_PUBLISH_LATEST=1 \
		BENCH001_PUBLISH_PEER=LR_UNIFY_FACT_L2_v1 \
		./tools/bench001/scripts/run_p_ladder_acc.sh lr-unify-fact-l2

bench001-medical-full-lr-unify-fact-l2: ## 080 D1 medical-full peer under LR_UNIFY_FACT_L2_medical_full_v1
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@BENCH001_LADDER_STAGE=medical-full \
		BENCH001_QUERY_ONLY=1 \
		BENCH001_SKIP_PUBLISH_LATEST=1 \
		BENCH001_PUBLISH_PEER=LR_UNIFY_FACT_L2_medical_full_v1 \
		./tools/bench001/scripts/run_p_ladder_acc.sh lr-unify-fact-l2

bench001-lr-intent-w-fact-l2: ## 080 D2: E2 + MIX_INTENT_WEIGHTS (smoke; not Acc Beat)
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@./tools/bench001/scripts/run_p_ladder_acc.sh lr-intent-w-fact-l2

bench001-medical-mid-lr-intent-w-fact-l2: ## 080 D2 medical-mid peer under LR_INTENT_W_FACT_L2_v1
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@BENCH001_LADDER_STAGE=medical-mid \
		BENCH001_QUERY_ONLY=1 \
		BENCH001_SKIP_PUBLISH_LATEST=1 \
		BENCH001_PUBLISH_PEER=LR_INTENT_W_FACT_L2_v1 \
		./tools/bench001/scripts/run_p_ladder_acc.sh lr-intent-w-fact-l2

bench001-lr-relsel-fact-l2: ## 080 D3 last-resort RELATION_SELECT=lightrag (smoke; high-risk Acc)
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@./tools/bench001/scripts/run_p_ladder_acc.sh lr-relsel-fact-l2

bench001-d0-forensics: ## 080 D0 failure slice on E2 mid + full archives
	@chmod +x tools/bench001/scripts/failure_slice_eq_lr.py
	@PYTHONPATH=tools/bench001 python3 tools/bench001/scripts/failure_slice_eq_lr.py \
		--archive specs/001-benchmark/e2e/artifacts/history/medical-mid-20260722T133053Z \
		--out specs/001-benchmark/e2e/artifacts/forensics/d0-e2-mid
	@PYTHONPATH=tools/bench001 python3 tools/bench001/scripts/failure_slice_eq_lr.py \
		--archive specs/001-benchmark/e2e/artifacts/history/medical-full-20260722T171906Z \
		--out specs/001-benchmark/e2e/artifacts/forensics/d0-e2-full

bench001-a2: ## 028 A2: A1 + INTENT_FACTUAL_BIAS=1
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@./tools/bench001/scripts/run_p_ladder_acc.sh a2

bench001-a3: ## 028 A3: A2 + ANSWER_PROMPT=lightrag
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@./tools/bench001/scripts/run_p_ladder_acc.sh a3

bench001-a4: ## 028 A4: Acc CI (BENCH001_A4_PACKAGE=a3|a2|a1|a0)
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@./tools/bench001/scripts/run_p_ladder_acc.sh a4

bench001-b1-audit: ## 028/029 B1: EQ↔LR extract/source_id audit (warm WS)
	@chmod +x tools/bench001/scripts/audit_eq_lr_ingest.py
	@PYTHONPATH=tools/bench001 python3 tools/bench001/scripts/audit_eq_lr_ingest.py

bench001-b2-reingest: ## 030 B2: new WS markdown+gleaning force-ingest then A1 Acc
	@chmod +x tools/bench001/scripts/run_b2_reingest_acc.sh
	@./tools/bench001/scripts/run_b2_reingest_acc.sh

bench001-b3-reingest: ## 031 B3a: FAQ induce+markdown+glean force-ingest then A1 Acc
	@chmod +x tools/bench001/scripts/run_b3_reingest_acc.sh
	@./tools/bench001/scripts/run_b3_reingest_acc.sh

bench001-b3b-reingest: ## 032 B3b: ws-scoped AGE node ids + markdown+glean (no FAQ) then A1 Acc
	@chmod +x tools/bench001/scripts/run_b3b_reingest_acc.sh
	@./tools/bench001/scripts/run_b3b_reingest_acc.sh

bench001-b5-reingest: ## 044 B5: placeholder provenance inherit + md+glean then a1fp Acc
	@chmod +x tools/bench001/scripts/run_b5_reingest_acc.sh
	@./tools/bench001/scripts/run_b5_reingest_acc.sh

bench001-b6-reingest: ## 049 B6: relation dedupe source-chunk union + md+glean then a1fp Acc
	@chmod +x tools/bench001/scripts/run_b6_reingest_acc.sh
	@./tools/bench001/scripts/run_b6_reingest_acc.sh

bench001-b7-reingest: ## 050 B7: placeholder entity VDB parity + md+glean then a1fp Acc
	@chmod +x tools/bench001/scripts/run_b7_reingest_acc.sh
	@./tools/bench001/scripts/run_b7_reingest_acc.sh

bench001-b8-reingest: ## 053 B8: entity types LR parity (no DATE) + md+glean then a1fp Acc
	@chmod +x tools/bench001/scripts/run_b8_reingest_acc.sh
	@./tools/bench001/scripts/run_b8_reingest_acc.sh

bench001-b9-reingest: ## 054 B9: extract caps LR parity (40/100) + md+glean then a1fp Acc
	@chmod +x tools/bench001/scripts/run_b9_reingest_acc.sh
	@./tools/bench001/scripts/run_b9_reingest_acc.sh

bench001-b10-reingest: ## 056/081 B10: naming identity + md+glean then E2 medical-mid (not Acc headline)
	@chmod +x tools/bench001/scripts/run_b10_reingest_acc.sh
	@./tools/bench001/scripts/run_b10_reingest_acc.sh

bench001-c1a: ## 058 C1a: Fact CE-skip latency peer (not Acc promote); needs warm Acc WS
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@./tools/bench001/scripts/run_p_ladder_acc.sh c1a

bench001-c1b: ## 059 C1b: BM25-all (no CE) latency peer + keyword/embed split (not Acc promote)
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@./tools/bench001/scripts/run_p_ladder_acc.sh c1b

bench001-c1d: ## 060 C1d: BM25-all + heuristic keywords (skip keyword LLM); not Acc promote
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@./tools/bench001/scripts/run_p_ladder_acc.sh c1d

bench001-c1e: ## 062 C1e: BM25-all + fast KEYWORD LLM (ministral-3b); not Acc promote
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@./tools/bench001/scripts/run_p_ladder_acc.sh c1e

bench001-c1cold: ## 063 C1cold: C1b + LR LLM cache OFF (fair cold EQ/LR latency); not Acc promote
	@chmod +x tools/bench001/scripts/run_p_ladder_acc.sh
	@./tools/bench001/scripts/run_p_ladder_acc.sh c1cold

bench001-smoke-fast-acc-rr: bench001-install ## Optional P3: smoke-fast Acc with EDGEQUAKE_MIX_FUSION=round_robin (query-only; warm WS)
	@set -a && [ -f "$(DEV_PORTS_ENV)" ] && . "$(DEV_PORTS_ENV)"; set +a; \
	export EDGEQUAKE_API_URL="$${EDGEQUAKE_API_URL:-$(BACKEND_URL)}"; \
	export EDGEQUAKE_MIX_ARM_GATE="$(BENCH001_EQ_MIX_ARM_GATE)"; \
	export EDGEQUAKE_MIX_FUSION=round_robin; \
	export BENCH001_PUBLISH_FAIRNESS=1; \
	export LLM_API_KEY="$${LLM_API_KEY:-$$MISTRAL_API_KEY}"; \
	export PYTHONPATH="tools/bench001:$${PYTHONPATH}"; \
	export BENCH001_LR_QUERY_CONCURRENCY="$${BENCH001_LR_QUERY_CONCURRENCY:-1}"; \
	if [ -z "$${BENCH001_EQ_WORKSPACE_ID:-}" ]; then echo "$(RED)BENCH001_EQ_WORKSPACE_ID required for query-only RR ablation$(RESET)"; exit 1; fi; \
	python3 -m bench001.cli smoke-fast --api "$$EDGEQUAKE_API_URL" --query-only \
	  --llm-provider mistral --llm-model "$(BENCH001_ACC_LLM_MODEL)" \
	  --judge-provider mistral --judge-model "$(BENCH001_ACC_JUDGE_MODEL)" \
	  --answer-style gold \
	  --profile-id P0_mistral_medium_mix_rr_fusion_v2 \
	  --query-concurrency "$(BENCH001_ACC_QUERY_CONCURRENCY)" \
	  --eval-concurrency "$(BENCH001_ACC_EVAL_CONCURRENCY)"; \
	echo "$(GREEN)→ SUMMARY:$(RESET) specs/001-benchmark/e2e/artifacts/smoke-fast/SUMMARY.md"

bench001-acc-canary: bench001-install ## Acc instrument canaries (judge-only; paraphrase/wrong-fact)
	@set -a && [ -f "$(DEV_PORTS_ENV)" ] && . "$(DEV_PORTS_ENV)"; set +a; \
	export BENCH001_JUDGE_PROVIDER="$${BENCH001_JUDGE_PROVIDER:-mistral}"; \
	export BENCH001_JUDGE_MODEL="$${BENCH001_JUDGE_MODEL:-mistral-small-latest}"; \
	export BENCH001_EVAL_CONCURRENCY="$(BENCH001_ACC_EVAL_CONCURRENCY)"; \
	export LLM_API_KEY="$${LLM_API_KEY:-$$MISTRAL_API_KEY}"; \
	export PYTHONPATH="tools/bench001:$${PYTHONPATH}"; \
	python3 -m bench001.cli acc-canary \
	  --judge-provider "$$BENCH001_JUDGE_PROVIDER" \
	  --judge-model "$$BENCH001_JUDGE_MODEL" \
	  --eval-concurrency "$(BENCH001_ACC_EVAL_CONCURRENCY)"; \
	echo "$(GREEN)→ SUMMARY:$(RESET) specs/001-benchmark/e2e/artifacts/acc-canary/SUMMARY.md"

bench001-smoke-paper: bench001-install ## Paper-track Acc rescore (GPT-4o-mini + BGE) on frozen smoke predictions
	@set -a && [ -f "$(DEV_PORTS_ENV)" ] && . "$(DEV_PORTS_ENV)"; set +a; \
	export PYTHONPATH="tools/bench001:$${PYTHONPATH}"; \
	export BENCH001_PUBLISH_FAIRNESS=1; \
	export BENCH001_EVAL_CONCURRENCY="$(BENCH001_ACC_EVAL_CONCURRENCY)"; \
	export BENCH001_JUDGE_EMBED_BACKEND=hf_bge; \
	export LLM_API_KEY="$${LLM_API_KEY:-$$OPENAI_API_KEY}"; \
	test -n "$$OPENAI_API_KEY" || { echo "$(RED)OPENAI_API_KEY required for P0_paper$(RESET)"; exit 2; }; \
	python3 -m bench001.cli rescore --source smoke \
	  --profile-id P0_paper \
	  --judge-provider openai --judge-model gpt-4o-mini \
	  --judge-base-url https://api.openai.com/v1 \
	  --judge-embedding-model BAAI/bge-large-en-v1.5 \
	  --judge-embed-backend hf_bge \
	  --eval-concurrency "$(BENCH001_ACC_EVAL_CONCURRENCY)"; \
	echo "$(GREEN)→ SUMMARY:$(RESET) specs/001-benchmark/e2e/artifacts/smoke-paper/SUMMARY.md"

bench001-core: bench001-install ## SPEC-001 core (default mistral-small-latest + mistral-embed; cost-gated)
	@set -a && [ -f "$(DEV_PORTS_ENV)" ] && . "$(DEV_PORTS_ENV)"; set +a; \
	export EDGEQUAKE_API_URL="$${EDGEQUAKE_API_URL:-$(BACKEND_URL)}"; \
	export EDGEQUAKE_LLM_PROVIDER="$${EDGEQUAKE_LLM_PROVIDER:-$${BENCH001_LLM_PROVIDER:-mistral}}"; \
	export EDGEQUAKE_LLM_MODEL="$${EDGEQUAKE_LLM_MODEL:-$${BENCH001_LLM_MODEL:-$${MISTRAL_MODEL:-mistral-small-latest}}}"; \
	export MISTRAL_MODEL="$$EDGEQUAKE_LLM_MODEL"; \
	export EDGEQUAKE_EMBEDDING_PROVIDER="$${EDGEQUAKE_EMBEDDING_PROVIDER:-$${BENCH001_EMBEDDING_PROVIDER:-mistral}}"; \
	export MISTRAL_EMBEDDING_MODEL="$${MISTRAL_EMBEDDING_MODEL:-$${BENCH001_EMBEDDING_MODEL:-mistral-embed}}"; \
	export EDGEQUAKE_VISION_PROVIDER="$${EDGEQUAKE_VISION_PROVIDER:-$${BENCH001_VISION_PROVIDER:-mistral}}"; \
	export EDGEQUAKE_VISION_MODEL="$${EDGEQUAKE_VISION_MODEL:-$${BENCH001_VISION_MODEL:-mistral-small-latest}}"; \
	export VLM_PROCESS_ENABLE=true; \
	export BENCH001_QUERY_CONCURRENCY="$${BENCH001_QUERY_CONCURRENCY:-8}"; \
	export BENCH001_EVAL_CONCURRENCY="$${BENCH001_EVAL_CONCURRENCY:-8}"; \
	python3 -m bench001.cli core --api "$$EDGEQUAKE_API_URL" --i-accept-cost --query-concurrency "$$BENCH001_QUERY_CONCURRENCY" --eval-concurrency "$$BENCH001_EVAL_CONCURRENCY"; \
	echo "$(GREEN)→ SUMMARY:$(RESET) specs/001-benchmark/e2e/artifacts/core/SUMMARY.md"
