#!/usr/bin/env sh
# EdgeQuake — Interactive Setup Wizard
#
# Usage (no git clone required):
#   curl -fsSL https://raw.githubusercontent.com/raphaelmansuy/edgequake/edgequake-main/quickstart.sh | sh
#
# Or with a pinned version:
#   EDGEQUAKE_VERSION=0.9.12 curl -fsSL ... | sh
#
# Prerequisites: Docker  (https://docs.docker.com/get-docker/)
#
# Design decisions: see specs/install_script/

set -e

# ════════════════════════════════════════════════════════════════════════════
# § Configurable defaults  (all overridable via environment)
# ════════════════════════════════════════════════════════════════════════════
EDGEQUAKE_VERSION="${EDGEQUAKE_VERSION:-latest}"
EDGEQUAKE_PORT="${EDGEQUAKE_PORT:-8080}"
FRONTEND_PORT="${FRONTEND_PORT:-3000}"
COMPOSE_FILE="${COMPOSE_FILE:-docker-compose.quickstart.yml}"
RAW_BASE="https://raw.githubusercontent.com/raphaelmansuy/edgequake/edgequake-main"

# Runtime state — populated by wizard, never read from env
LLM_PROVIDER=""
LLM_MODEL=""
EMBED_PROVIDER=""
EMBED_MODEL=""
COMPOSE_CMD=""

# ════════════════════════════════════════════════════════════════════════════
# § Colour / TUI design tokens  (ADR-005)
# Only emit ANSI when stdout is a real TTY; collapses to "" otherwise
# ════════════════════════════════════════════════════════════════════════════
if [ -t 1 ]; then
  C_BOLD="\033[1m"
  C_DIM="\033[2m"
  C_RESET="\033[0m"
  C_GREEN="\033[32m"
  C_YELLOW="\033[33m"
  C_RED="\033[31m"
  C_BLUE="\033[34m"
  C_CYAN="\033[36m"
else
  C_BOLD="" C_DIM="" C_RESET="" C_GREEN="" C_YELLOW="" C_RED="" C_BLUE="" C_CYAN=""
fi

# ════════════════════════════════════════════════════════════════════════════
# § UI components  (ADR-005)
# ════════════════════════════════════════════════════════════════════════════

ui_banner() {
  printf "\n${C_BOLD}${C_BLUE}"
  printf "  ╔══════════════════════════════════════════════╗\n"
  printf "  ║   EdgeQuake Setup Wizard    %-16s║\n" "v${EDGEQUAKE_VERSION}"
  printf "  ╚══════════════════════════════════════════════╝\n"
  printf "${C_RESET}\n"
}

# Section header with a dim rule underneath
ui_section() {
  printf "\n${C_BOLD}${C_CYAN}  ▸ %s${C_RESET}\n" "$1"
  printf "  ${C_DIM}──────────────────────────────────────────────────────────${C_RESET}\n\n"
}

ui_ok()    { printf "  ${C_GREEN}✓${C_RESET}  %s\n"         "$1"; }
ui_info()  { printf "  ${C_BLUE}→${C_RESET}  %s\n"          "$1"; }
ui_warn()  { printf "  ${C_YELLOW}⚠${C_RESET}  %s\n"        "$1"; }
ui_fail()  { printf "  ${C_RED}✗${C_RESET}  %s\n" "$1" >&2; }
ui_blank() { printf "\n"; }

# ════════════════════════════════════════════════════════════════════════════
# § TTY I/O primitives  (ADR-004)
# All interactive reads go through /dev/tty so the wizard works when stdin
# is a pipe (curl | sh), which is the primary install method.
# ════════════════════════════════════════════════════════════════════════════

# Read one line from /dev/tty.  Result in $_TTY_INPUT.
_tty_read() {
  if [ -e /dev/tty ]; then
    read -r _TTY_INPUT < /dev/tty
  else
    _TTY_INPUT=""
  fi
}

# Read a secret (no echo) from /dev/tty.  Result in $_TTY_INPUT.
_tty_read_secret() {
  if [ -e /dev/tty ]; then
    stty -echo < /dev/tty 2>/dev/null || true
    read -r _TTY_INPUT < /dev/tty
    stty echo  < /dev/tty 2>/dev/null || true
    printf '\n'
  else
    _TTY_INPUT=""
  fi
}

# ════════════════════════════════════════════════════════════════════════════
# § Numbered menu  (ADR-005)
#
# ui_menu "Prompt text" "option 1" "option 2" ...
# Result stored in $MENU_RESULT (1-indexed integer).
# Loops until the user enters a valid number.
# ════════════════════════════════════════════════════════════════════════════
ui_menu() {
  _um_prompt="$1"; shift
  _um_total=$#
  _um_i=1

  printf "\n  ${C_BOLD}%s${C_RESET}\n\n" "$_um_prompt"
  for _um_opt in "$@"; do
    printf "    ${C_CYAN}[%d]${C_RESET}  %s\n" "$_um_i" "$_um_opt"
    _um_i=$((_um_i + 1))
  done

  while true; do
    printf "\n  ${C_BOLD}Enter choice (1-%d): ${C_RESET}" "$_um_total"
    _tty_read
    MENU_RESULT="${_TTY_INPUT:-}"
    case "$MENU_RESULT" in
      ''|*[!0-9]*) ;;   # not a number -> stay in loop
      *)
        if [ "$MENU_RESULT" -ge 1 ] && [ "$MENU_RESULT" -le "$_um_total" ] 2>/dev/null; then
          printf "\n"
          return 0
        fi
        ;;
    esac
    ui_warn "Please enter a number between 1 and ${_um_total}."
  done
}

# Yes/No confirm.  $1 = prompt, $2 = default ("y" or "n").
# Returns 0 for yes, 1 for no.
ui_confirm() {
  _uc_def="${2:-n}"
  case "$_uc_def" in y|Y) _uc_opts="[Y/n]" ;; *) _uc_opts="[y/N]" ;; esac
  printf "  ${C_BOLD}%s${C_RESET} %s: " "$1" "$_uc_opts"
  _tty_read
  _uc_ans="${_TTY_INPUT:-}"
  [ -z "$_uc_ans" ] && _uc_ans="$_uc_def"
  case "$_uc_ans" in y|Y|yes|YES) return 0 ;; *) return 1 ;; esac
}

# ════════════════════════════════════════════════════════════════════════════
# § HTTP download helper (curl or wget)
# ════════════════════════════════════════════════════════════════════════════
_http_get() {
  # $1 = URL   $2 = destination file
  if command -v curl > /dev/null 2>&1; then
    curl -fsSL "$1" -o "$2"
  elif command -v wget > /dev/null 2>&1; then
    wget -qO "$2" "$1"
  else
    ui_fail "curl or wget is required to download the compose file."
    exit 1
  fi
}

# ════════════════════════════════════════════════════════════════════════════
# § Management footer (printed after a user chooses "Quit")
# ════════════════════════════════════════════════════════════════════════════
_print_mgmt_footer() {
  ui_blank
  printf "  ${C_BOLD}Management commands:${C_RESET}\n"
  printf "    Logs:   %s -f %s logs -f\n"  "$COMPOSE_CMD" "$COMPOSE_FILE"
  printf "    Stop:   %s -f %s down\n"     "$COMPOSE_CMD" "$COMPOSE_FILE"
  printf "    Update: sh quickstart.sh\n"
  ui_blank
}

# ════════════════════════════════════════════════════════════════════════════
# § Step 1 — Pre-flight checks
# ════════════════════════════════════════════════════════════════════════════
check_prerequisites() {
  ui_section "Pre-flight Checks"

  # Docker binary
  if ! command -v docker > /dev/null 2>&1; then
    ui_fail "Docker is not installed."
    ui_fail "Install from: https://docs.docker.com/get-docker/"
    exit 1
  fi
  ui_ok "Docker: $(docker --version | head -1)"

  # docker compose plugin (v2) or standalone docker-compose (v1)
  if docker compose version > /dev/null 2>&1; then
    COMPOSE_CMD="docker compose"
  elif command -v docker-compose > /dev/null 2>&1; then
    COMPOSE_CMD="docker-compose"
  else
    ui_fail "docker compose (v2 plugin) or docker-compose (v1) is required."
    ui_fail "Install: https://docs.docker.com/compose/install/"
    exit 1
  fi
  ui_ok "Compose: $($COMPOSE_CMD version --short 2>/dev/null || echo 'v1')"

  # /dev/tty — required for the interactive wizard  (ADR-004)
  if [ ! -e /dev/tty ]; then
    ui_blank
    ui_fail "No interactive terminal detected (/dev/tty is not accessible)."
    ui_info  "The setup wizard requires an interactive terminal."
    ui_info  "For automated installs, use environment variables directly:"
    ui_blank
    printf   '    EDGEQUAKE_LLM_PROVIDER=openai \\\n'
    printf   '    OPENAI_API_KEY=sk-... \\\n'
    printf   "    %s -f %s up -d\n" "$COMPOSE_CMD" "$COMPOSE_FILE"
    ui_blank
    exit 1
  fi
}

# ════════════════════════════════════════════════════════════════════════════
# § Step 2 — Download / refresh compose file
# Always fetch a fresh copy so new env vars and service definitions are
# picked up on re-runs without manual cache clearing. (ADR-002)
# ════════════════════════════════════════════════════════════════════════════
download_compose() {
  ui_section "Compose File"

  if [ -f "$COMPOSE_FILE" ]; then
    cp "$COMPOSE_FILE" "${COMPOSE_FILE}.bak"
    ui_info "Backed up existing file -> ${COMPOSE_FILE}.bak"
  fi

  ui_info "Downloading latest compose file..."
  _http_get "${RAW_BASE}/docker-compose.quickstart.yml" "$COMPOSE_FILE"
  ui_ok "Compose file ready: ./${COMPOSE_FILE}"
}

# ════════════════════════════════════════════════════════════════════════════
# § Step 3 — Existing installation detection  (ADR-002)
# ════════════════════════════════════════════════════════════════════════════

# Called when the user chose "Fresh Start". Requires "DELETE" confirmation.
_confirm_fresh_start() {
  ui_blank
  ui_warn "This will permanently delete ALL EdgeQuake data (PostgreSQL volumes, graph)."
  ui_blank
  printf "  Type ${C_BOLD}DELETE${C_RESET} to confirm, or press Enter to cancel: "
  _tty_read
  if [ "${_TTY_INPUT:-}" = "DELETE" ]; then
    ui_info "Removing containers and volumes..."
    $COMPOSE_CMD -f "$COMPOSE_FILE" down -v --remove-orphans 2>/dev/null || \
      docker volume rm edgequake-pg-data 2>/dev/null || true
    ui_ok "All data removed. Continuing with a fresh installation."
  else
    ui_info "Fresh start cancelled — your data is preserved."
  fi
}

handle_existing_install() {
  ui_section "Installation Check"

  # WHY wc -l: grep -c exits 1 on no matches (outputting "0") and then a
  # "|| echo 0" in the same subshell would produce "0\n0" — two copies — which
  # breaks integer comparisons.  wc -l always exits 0 and outputs a single
  # clean integer, regardless of whether the upstream grep/docker had matches.
  _eq_running=$(docker ps \
    --filter "name=edgequake-api" --filter "status=running" \
    --format "{{.Names}}" 2>/dev/null \
    | grep "edgequake" 2>/dev/null | wc -l | tr -d ' ')

  _eq_stopped=$(docker ps -a \
    --filter "name=edgequake" \
    --format "{{.Names}}" 2>/dev/null \
    | grep "edgequake" 2>/dev/null | wc -l | tr -d ' ')

  _eq_volumes=$(docker volume ls \
    --filter "name=edgequake" \
    --format "{{.Name}}" 2>/dev/null \
    | grep "edgequake" 2>/dev/null | wc -l | tr -d ' ')

  # -- Nothing found -> fresh install ----------------------------------------
  if [ "$_eq_running" -eq 0 ] && [ "$_eq_stopped" -eq 0 ] && [ "$_eq_volumes" -eq 0 ]; then
    ui_ok "No existing installation found — starting fresh."
    return 0
  fi

  # -- Running containers ----------------------------------------------------
  if [ "$_eq_running" -gt 0 ]; then
    ui_warn "EdgeQuake is currently running:"
    ui_blank
    docker ps --filter "name=edgequake" \
      --format "    • {{.Names}}  [{{.Status}}]" 2>/dev/null
    ui_blank

    ui_menu "What would you like to do?" \
      "Update & Reconfigure  — pull latest images, choose new provider + model" \
      "Quit                  — leave the existing installation unchanged"

    case "$MENU_RESULT" in
      2)
        ui_ok "Leaving installation unchanged."
        _print_mgmt_footer
        exit 0
        ;;
    esac
    # Choice 1 -> fall through to the provider/model wizard
    return 0
  fi

  # -- Stopped containers or orphaned volumes --------------------------------
  if [ "$_eq_stopped" -gt 0 ]; then
    ui_warn "Stopped EdgeQuake containers:"
    ui_blank
    docker ps -a --filter "name=edgequake" \
      --format "    • {{.Names}}  [{{.Status}}]" 2>/dev/null
    ui_blank
  fi

  if [ "$_eq_volumes" -gt 0 ]; then
    ui_info "Existing data volumes (your knowledge graph is preserved):"
    docker volume ls --filter "name=edgequake" \
      --format "    • {{.Name}}" 2>/dev/null
    ui_blank
  fi

  ui_menu "What would you like to do?" \
    "Restart & Reconfigure  — choose provider/model, then start  (data preserved)" \
    "Fresh Start            — WARNING: DELETE all data and start over (irreversible)" \
    "Quit                   — do nothing"

  case "$MENU_RESULT" in
    2) _confirm_fresh_start ;;
    3)
      ui_ok "No changes made."
      exit 0
      ;;
  esac
  # Choice 1 (or 2 after cancelled fresh-start) -> fall through to wizard
}

# ════════════════════════════════════════════════════════════════════════════
# § Step 4 — Provider wizard  (ADR-001)
# Always asks; never auto-detects.  Informational hint only for API key.
# ════════════════════════════════════════════════════════════════════════════
choose_provider() {
  ui_section "LLM Provider"

  if [ -n "${OPENAI_API_KEY:-}" ]; then
    ui_info "OPENAI_API_KEY is set in your environment."
  else
    ui_info "Tip: export OPENAI_API_KEY=sk-... before running to use OpenAI."
  fi
  ui_blank

  ui_menu "Which LLM provider do you want to use?" \
    "OpenAI   — cloud API (GPT-5.4 family) · requires OPENAI_API_KEY" \
    "Ollama   — fully local, free to run   · requires Ollama daemon on port 11434"

  case "$MENU_RESULT" in
    1) LLM_PROVIDER="openai" ;;
    2) LLM_PROVIDER="ollama" ;;
  esac

  ui_ok "Provider: ${C_BOLD}${LLM_PROVIDER}${C_RESET}"
}

# ════════════════════════════════════════════════════════════════════════════
# § Step 5 — Model wizard  (ADR-003)
# Presents LLM model then embedding model, both scoped to the chosen provider.
# ════════════════════════════════════════════════════════════════════════════
choose_models() {
  ui_section "Model Selection"

  if [ "$LLM_PROVIDER" = "openai" ]; then

    ui_menu "Which OpenAI model for LLM inference?" \
      "gpt-5.4-mini   Recommended — fast, affordable, reliable JSON output   (in:\$0.75 out:\$4.50 per MTok)" \
      "gpt-5.4-nano     Ultra-cheap, great for testing, direct output         (in:\$0.20 out:\$1.25 per MTok)" \
      "gpt-5.4          Premium quality, large context                        (in:\$2.50 out:\$15.00 per MTok)" \
      "gpt-5.4-mini     Fast with larger context window                       (in:\$0.75 out:\$4.50 per MTok)"
    # NOTE: gpt-5-* and gpt-5-nano/-mini are reasoning-only models; they
    # consume all completion tokens for chain-of-thought, leaving none for
    # JSON output. Always prefer gpt-5.4-* models for entity extraction.
    case "$MENU_RESULT" in
      1) LLM_MODEL="gpt-5.4-mini" ;;
      2) LLM_MODEL="gpt-5.4-nano" ;;
      3) LLM_MODEL="gpt-5.4"      ;;
      4) LLM_MODEL="gpt-5.4-mini" ;;
    esac

    ui_menu "Which OpenAI model for embeddings?" \
      "text-embedding-3-small   Recommended — fast, 1536 dims" \
      "text-embedding-3-large   Higher quality, 3072 dims"
    case "$MENU_RESULT" in
      1) EMBED_MODEL="text-embedding-3-small" ;;
      2) EMBED_MODEL="text-embedding-3-large" ;;
    esac

    EMBED_PROVIDER="openai"

  else  # ollama

    ui_menu "Which Ollama model for LLM inference?" \
      "gemma4:e4b       Recommended — balanced quality/size (9.6 GB)" \
      "gemma4:e2b         Lighter, faster startup           (7.2 GB)" \
      "gemma4:26b         Large MoE, best quality           (requires 16+ GB RAM)" \
      "qwen2.5:latest     Strong at structured/JSON tasks   (~5 GB)" \
      "llama3.2:latest    Meta general-purpose model        (~2 GB)"
    case "$MENU_RESULT" in
      1) LLM_MODEL="gemma4:e4b"      ;;
      2) LLM_MODEL="gemma4:e2b"      ;;
      3) LLM_MODEL="gemma4:26b"      ;;
      4) LLM_MODEL="qwen2.5:latest"  ;;
      5) LLM_MODEL="llama3.2:latest" ;;
    esac

    ui_menu "Which Ollama model for embeddings?" \
      "embeddinggemma:latest    Recommended — fast, high quality" \
      "nomic-embed-text:latest  Alternative — well-tested"
    case "$MENU_RESULT" in
      1) EMBED_MODEL="embeddinggemma:latest"   ;;
      2) EMBED_MODEL="nomic-embed-text:latest" ;;
    esac

    EMBED_PROVIDER="ollama"

  fi

  ui_ok "LLM model:       ${C_BOLD}${LLM_MODEL}${C_RESET}"
  ui_ok "Embedding model: ${C_BOLD}${EMBED_MODEL}${C_RESET}"
}

# ════════════════════════════════════════════════════════════════════════════
# § Step 6 — Provider validation
# OpenAI: ensure API key is available (prompt if missing, mask input).
# Ollama: ping /api/tags; check if the chosen model is pulled.
# ════════════════════════════════════════════════════════════════════════════
validate_provider() {
  ui_section "Provider Validation"

  if [ "$LLM_PROVIDER" = "openai" ]; then

    if [ -z "${OPENAI_API_KEY:-}" ]; then
      ui_warn "OPENAI_API_KEY is not set."
      printf "  Enter your OpenAI API key (input hidden): "
      _tty_read_secret
      OPENAI_API_KEY="${_TTY_INPUT:-}"
      if [ -z "$OPENAI_API_KEY" ]; then
        ui_fail "No API key provided."
        ui_fail "Re-run with: export OPENAI_API_KEY=sk-... && sh quickstart.sh"
        exit 1
      fi
    fi
    ui_ok "OpenAI API key is set."

  else  # ollama

    _ollama_host="${OLLAMA_HOST:-http://localhost:11434}"

    if curl -sf "${_ollama_host}/api/tags" > /dev/null 2>&1; then
      ui_ok "Ollama is reachable at ${_ollama_host}"

      # Non-critical: check if chosen model is already pulled
      if curl -sf "${_ollama_host}/api/tags" 2>/dev/null | grep -q "\"${LLM_MODEL}\"" 2>/dev/null; then
        ui_ok "Model '${LLM_MODEL}' is available in Ollama."
      else
        ui_warn "Model '${LLM_MODEL}' may not be pulled yet."
        ui_info "Run after startup: ollama pull ${LLM_MODEL}"
      fi

    else
      ui_warn "Ollama is not reachable at ${_ollama_host}"
      ui_blank
      printf "  ${C_DIM}To start Ollama:${C_RESET}\n"
      printf "    ollama serve &\n"
      printf "    ollama pull %s\n" "$LLM_MODEL"
      ui_blank

      if ! ui_confirm "Continue without Ollama running?" "n"; then
        ui_fail "Aborted. Start Ollama and re-run."
        exit 1
      fi
      ui_warn "Remember to start Ollama before uploading documents."
    fi

  fi
}

# ════════════════════════════════════════════════════════════════════════════
# § Step 7 — Pull images and start the stack
# ════════════════════════════════════════════════════════════════════════════
start_stack() {
  ui_section "Starting EdgeQuake"

  # Export resolved configuration for docker compose interpolation
  export EDGEQUAKE_VERSION
  export EDGEQUAKE_PORT
  export FRONTEND_PORT
  export EDGEQUAKE_LLM_PROVIDER="$LLM_PROVIDER"
  export EDGEQUAKE_LLM_MODEL="$LLM_MODEL"
  export EDGEQUAKE_EMBEDDING_PROVIDER="$EMBED_PROVIDER"
  export EDGEQUAKE_EMBEDDING_MODEL="$EMBED_MODEL"

  # WHY: Only export OPENAI_API_KEY for OpenAI mode.
  # For Ollama mode, unset it to prevent an empty string reaching the container
  # (Docker Compose maps unset -> "" via ${VAR:-}; the API strips empty env vars
  # at startup, but defence-in-depth is better).
  if [ "$LLM_PROVIDER" = "openai" ]; then
    export OPENAI_API_KEY
  else
    unset OPENAI_API_KEY 2>/dev/null || true
  fi

  # Always unset OPENAI_BASE_URL unless the user has explicitly set it to a
  # non-empty value (e.g. for an OpenAI-compatible endpoint).
  if [ -z "${OPENAI_BASE_URL:-}" ]; then
    unset OPENAI_BASE_URL 2>/dev/null || true
  fi

  ui_info "Pulling images (version: ${EDGEQUAKE_VERSION})..."
  $COMPOSE_CMD -f "$COMPOSE_FILE" pull

  ui_info "Starting services (detached)..."
  $COMPOSE_CMD -f "$COMPOSE_FILE" up -d --force-recreate --remove-orphans

  # Health polling — up to 90 seconds
  ui_info "Waiting for API health check (up to 90s)..."
  _sq_i=0
  _sq_healthy=0
  while [ "$_sq_i" -lt 45 ]; do
    if curl -sf "http://localhost:${EDGEQUAKE_PORT}/health" > /dev/null 2>&1; then
      _sq_healthy=1
      break
    fi
    printf "."
    sleep 2
    _sq_i=$((_sq_i + 1))
  done
  printf "\n"

  if [ "$_sq_healthy" -eq 0 ]; then
    ui_fail "API did not become healthy within 90 seconds."
    ui_info "Check logs: $COMPOSE_CMD -f $COMPOSE_FILE logs -f api"
    exit 1
  fi

  ui_ok "API is healthy!"
}

# ════════════════════════════════════════════════════════════════════════════
# § Step 8 — Success summary
# ════════════════════════════════════════════════════════════════════════════
print_summary() {
  printf "\n${C_BOLD}${C_GREEN}"
  printf "  ══════════════════════════════════════════\n"
  printf "  EdgeQuake is running!\n"
  printf "  ══════════════════════════════════════════${C_RESET}\n\n"

  printf "  Web UI:    ${C_BOLD}http://localhost:${FRONTEND_PORT}${C_RESET}\n"
  printf "  API:       ${C_BOLD}http://localhost:${EDGEQUAKE_PORT}${C_RESET}\n"
  printf "  Swagger:   ${C_BOLD}http://localhost:${EDGEQUAKE_PORT}/swagger-ui${C_RESET}\n"
  printf "  Health:    ${C_BOLD}http://localhost:${EDGEQUAKE_PORT}/health${C_RESET}\n\n"

  if [ "$LLM_PROVIDER" = "openai" ]; then
    printf "  Provider:  ${C_BOLD}OpenAI${C_RESET}\n"
  else
    printf "  Provider:  ${C_BOLD}Ollama${C_RESET}\n"
  fi
  printf "  LLM:       ${C_BOLD}%s${C_RESET}\n"   "$LLM_MODEL"
  printf "  Embedding: ${C_BOLD}%s${C_RESET}\n\n"  "$EMBED_MODEL"

  if [ "$LLM_PROVIDER" = "ollama" ]; then
    printf "  ${C_YELLOW}->  If not done yet: ${C_BOLD}ollama pull %s${C_RESET}\n\n" "$LLM_MODEL"
  fi

  printf "  ${C_BOLD}Next steps:${C_RESET}\n"
  printf "    1. Open ${C_BOLD}http://localhost:${FRONTEND_PORT}${C_RESET} in your browser\n"
  printf "    2. Upload a PDF or paste text to build your knowledge graph\n"
  printf "    3. Ask questions — EdgeQuake retrieves graph-aware answers\n\n"

  _print_mgmt_footer
}

# ════════════════════════════════════════════════════════════════════════════
# § Main
# ════════════════════════════════════════════════════════════════════════════
main() {
  ui_banner
  check_prerequisites
  download_compose
  handle_existing_install
  choose_provider
  choose_models
  validate_provider
  start_stack
  print_summary
}

main
