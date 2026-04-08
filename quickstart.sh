#!/usr/bin/env sh
# EdgeQuake — One-Command Quickstart
#
# Usage (no git clone required):
#   curl -fsSL https://raw.githubusercontent.com/raphaelmansuy/edgequake/edgequake-main/quickstart.sh | sh
#
# Or with a pinned version:
#   curl -fsSL https://raw.githubusercontent.com/raphaelmansuy/edgequake/edgequake-main/quickstart.sh | \
#     EDGEQUAKE_VERSION=0.9.4 sh
#
# Prerequisites: Docker (https://docs.docker.com/get-docker/)

set -e

# ── Configurable defaults ──────────────────────────────────────────────────────
EDGEQUAKE_VERSION="${EDGEQUAKE_VERSION:-latest}"
EDGEQUAKE_PORT="${EDGEQUAKE_PORT:-8080}"
FRONTEND_PORT="${FRONTEND_PORT:-3000}"
COMPOSE_FILE="${COMPOSE_FILE:-docker-compose.quickstart.yml}"
RAW_BASE="https://raw.githubusercontent.com/raphaelmansuy/edgequake/edgequake-main"

# ── Colour helpers (disabled when not a TTY) ──────────────────────────────────
if [ -t 1 ]; then
  BOLD="\033[1m"; RESET="\033[0m"; GREEN="\033[32m"; YELLOW="\033[33m"; RED="\033[31m"; BLUE="\033[34m"
else
  BOLD=""; RESET=""; GREEN=""; YELLOW=""; RED=""; BLUE=""
fi

header() { printf "\n${BOLD}${BLUE}%s${RESET}\n\n" "$1"; }
ok()     { printf "  ${GREEN}✓${RESET} %s\n" "$1"; }
info()   { printf "  ${YELLOW}→${RESET} %s\n" "$1"; }
fail()   { printf "  ${RED}✗${RESET} %s\n" "$1" >&2; }

# ── Pre-flight checks ──────────────────────────────────────────────────────────
header "EdgeQuake Quickstart"

# Docker
if ! command -v docker > /dev/null 2>&1; then
  fail "Docker is not installed. Install it from https://docs.docker.com/get-docker/ and re-run."
  exit 1
fi
ok "Docker found: $(docker --version | head -1)"

# docker compose (v2 plugin or standalone v1)
if docker compose version > /dev/null 2>&1; then
  COMPOSE_CMD="docker compose"
elif command -v docker-compose > /dev/null 2>&1; then
  COMPOSE_CMD="docker-compose"
else
  fail "docker compose (v2 plugin) or docker-compose (v1) is required."
  fail "Install: https://docs.docker.com/compose/install/"
  exit 1
fi
ok "Compose found: $($COMPOSE_CMD version --short 2>/dev/null || echo 'v1')"

# ── Download compose file if not present ─────────────────────────────────────
if [ ! -f "$COMPOSE_FILE" ]; then
  info "Downloading compose file..."
  if command -v curl > /dev/null 2>&1; then
    curl -fsSL "${RAW_BASE}/docker-compose.quickstart.yml" -o "$COMPOSE_FILE"
  elif command -v wget > /dev/null 2>&1; then
    wget -qO "$COMPOSE_FILE" "${RAW_BASE}/docker-compose.quickstart.yml"
  else
    fail "curl or wget is required to download the compose file."
    exit 1
  fi
  ok "Compose file saved to ./${COMPOSE_FILE}"
else
  ok "Using existing ./${COMPOSE_FILE}"
fi

# ── LLM provider auto-detection ───────────────────────────────────────────────
if [ -n "$OPENAI_API_KEY" ]; then
  EDGEQUAKE_LLM_PROVIDER="${EDGEQUAKE_LLM_PROVIDER:-openai}"
  ok "OpenAI API key detected — using OpenAI provider"
else
  EDGEQUAKE_LLM_PROVIDER="${EDGEQUAKE_LLM_PROVIDER:-ollama}"
  info "No API key found — using Ollama (must be running on port 11434)"
  info "  To use OpenAI: export OPENAI_API_KEY=sk-... && sh quickstart.sh"
fi

# ── Pull images ───────────────────────────────────────────────────────────────
info "Pulling EdgeQuake images (version: ${EDGEQUAKE_VERSION})..."
EDGEQUAKE_VERSION="$EDGEQUAKE_VERSION" \
EDGEQUAKE_LLM_PROVIDER="$EDGEQUAKE_LLM_PROVIDER" \
OPENAI_API_KEY="${OPENAI_API_KEY:-}" \
EDGEQUAKE_PORT="$EDGEQUAKE_PORT" \
FRONTEND_PORT="$FRONTEND_PORT" \
  $COMPOSE_CMD -f "$COMPOSE_FILE" pull

# ── Start stack ───────────────────────────────────────────────────────────────
info "Starting all services (detached)..."
EDGEQUAKE_VERSION="$EDGEQUAKE_VERSION" \
EDGEQUAKE_LLM_PROVIDER="$EDGEQUAKE_LLM_PROVIDER" \
OPENAI_API_KEY="${OPENAI_API_KEY:-}" \
EDGEQUAKE_PORT="$EDGEQUAKE_PORT" \
FRONTEND_PORT="$FRONTEND_PORT" \
  $COMPOSE_CMD -f "$COMPOSE_FILE" up -d

# ── Wait for API health ───────────────────────────────────────────────────────
info "Waiting for API to be healthy (up to 90s)..."
i=0
while [ $i -lt 45 ]; do
  if curl -sf "http://localhost:${EDGEQUAKE_PORT}/health" > /dev/null 2>&1; then
    ok "API is healthy!"
    break
  fi
  printf "."
  sleep 2
  i=$((i + 1))
done
printf "\n"

if ! curl -sf "http://localhost:${EDGEQUAKE_PORT}/health" > /dev/null 2>&1; then
  fail "API did not become healthy within 90s."
  info "Check logs: $COMPOSE_CMD -f $COMPOSE_FILE logs -f api"
  exit 1
fi

# ── Done ──────────────────────────────────────────────────────────────────────
printf "\n${BOLD}${GREEN}✅  EdgeQuake is running!${RESET}\n\n"
printf "  🌐  Web UI:  ${BOLD}http://localhost:${FRONTEND_PORT}${RESET}\n"
printf "  🔗  API:     ${BOLD}http://localhost:${EDGEQUAKE_PORT}${RESET}\n"
printf "  📚  Swagger: ${BOLD}http://localhost:${EDGEQUAKE_PORT}/swagger-ui${RESET}\n"
printf "  🏥  Health:  ${BOLD}http://localhost:${EDGEQUAKE_PORT}/health${RESET}\n"
printf "\n"
printf "${BOLD}Next steps:${RESET}\n"
printf "  1. Open ${BOLD}http://localhost:${FRONTEND_PORT}${RESET} in your browser\n"
printf "  2. Upload a PDF or paste text to build your knowledge graph\n"
printf "  3. Ask questions — EdgeQuake retrieves graph-aware answers\n"
printf "\n"
printf "${YELLOW}Management:${RESET}\n"
printf "  Logs:   $COMPOSE_CMD -f $COMPOSE_FILE logs -f\n"
printf "  Status: $COMPOSE_CMD -f $COMPOSE_FILE ps\n"
printf "  Stop:   $COMPOSE_CMD -f $COMPOSE_FILE down\n"
printf "\n"
