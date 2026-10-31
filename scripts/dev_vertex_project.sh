#!/usr/bin/env bash
# SPEC-180: pick the Google Cloud project Vertex image embedding uses — ask once, save it.
#
# Why this exists
#   `gcp-use-mobioos` sets PROJECT_ID, not GOOGLE_CLOUD_PROJECT, while a developer shell may
#   export another project globally (`saas-app-001`). That one answers 403 for
#   gemini-embedding-2, so an image workspace failed at the last wizard step with a raw
#   Vertex error. The project is a property of this checkout, not of the caller's shell.
#
# What it does
#   - Non-interactive (CI, `make dev-bg`, piped input): never prompts. Resolve from
#     GOOGLE_CLOUD_PROJECT, then PROJECT_ID, then the saved file.
#   - Interactive on a terminal with no saved project: asks once, validates the answer by
#     minting a gcloud token, and saves it to .gcloud_active_project (gitignored).
#   - Already saved: prints it and exits. `make gcp-use PROJECT=<id>` changes it.
#
# The token is never printed, logged or stored: only its exit status is checked.
#
# Usage: scripts/dev_vertex_project.sh [--prompt] [--print] [--forget]
set -euo pipefail

_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
_file="$_root/.gcloud_active_project"

_cyan=$'\033[36m'; _green=$'\033[32m'; _yellow=$'\033[33m'; _red=$'\033[31m'; _dim=$'\033[2m'; _reset=$'\033[0m'

# Everything a human reads goes to stderr; stdout carries the project id only, so a caller
# can do `project=$(script --prompt)` and still see the question and the result.
say() { printf '%s\n' "$*" >&2; }
info() { printf '%s\n' "${_yellow}→${_reset} $*" >&2; }
ok() { printf '%s\n' "${_green}✓${_reset} $*" >&2; }
warn() { printf '%s\n' "${_yellow}⚠${_reset} $*" >&2; }
err() { printf '%s\n' "${_red}✗${_reset} $*" >&2; }

_saved() {
  [ -f "$_file" ] || return 1
  local value
  value="$(tr -d '[:space:]' < "$_file")"
  [ -n "$value" ] || return 1
  printf '%s' "$value"
}

# A project id is lowercase letters, digits, hyphens; 6..30 chars; starts with a letter.
_valid_id() {
  printf '%s' "$1" | grep -Eq '^[a-z][a-z0-9-]{4,28}[a-z0-9]$'
}

# Can this shell mint a Vertex token at all? Cheap, and it is the real precondition.
_identity_ok() {
  command -v gcloud >/dev/null 2>&1 || return 1
  gcloud auth print-access-token >/dev/null 2>&1
}

# Interactive only when a human can answer: a tty on stdin AND stdout.
_interactive() {
  [ -t 0 ] && [ -t 1 ]
}

_prompt() {
  local current="${1:-}" answer=""
  say ""
  say "${_cyan}Vertex AI project${_reset} ${_dim}(image embedding for Normal workspaces)${_reset}"
  say "  A workspace that embeds images needs a Google Cloud project whose identity may call"
  say "  aiplatform.googleapis.com. This is asked once and saved for this checkout."
  if ! _identity_ok; then
    warn "gcloud cannot mint a token right now. Run 'gcloud auth login' (or 'gcp-use-mobioos'),"
    warn "or set GOOGLE_ACCESS_TOKEN. Vertex image embedding will stay unavailable until then."
  fi
  local hint="${current:-}"
  if [ -z "$hint" ]; then
    hint="$(gcloud config get-value project 2>/dev/null | tr -d '[:space:]')"
    [ "$hint" = "(unset)" ] && hint=""
  fi
  if [ -n "$hint" ]; then
    printf '  Project id %s[%s]%s: ' "$_dim" "$hint" "$_reset" >&2
  else
    printf '  Project id: ' >&2
  fi
  read -r answer || true
  [ -z "$answer" ] && answer="$hint"
  if [ -z "$answer" ]; then
    err "No project id given. Vertex image embedding stays off; other providers are unaffected."
    return 1
  fi
  if ! _valid_id "$answer"; then
    err "'$answer' does not look like a Google Cloud project id."
    return 1
  fi
  printf '%s\n' "$answer" > "$_file"
  ok "Saved Vertex project: ${_cyan}$answer${_reset} ${_dim}(.gcloud_active_project)${_reset}"
  say "  Change it any time with: ${_cyan}make gcp-use PROJECT=<id>${_reset}"
  printf '%s' "$answer"
}

case "${1:-}" in
  --forget)
    rm -f "$_file"
    ok "Forgot the saved Vertex project. The next interactive 'make dev' will ask again."
    exit 0
    ;;
  --print)
    if value="$(_saved)"; then printf '%s\n' "$value"; else exit 1; fi
    exit 0
    ;;
  --prompt) : ;;
  "") : ;;
  *)
    err "unknown argument: $1"
    exit 2
    ;;
esac

# 1. Saved project wins: it was chosen deliberately for this checkout.
if value="$(_saved)"; then
  info "Vertex project: ${_cyan}$value${_reset} ${_dim}(saved)${_reset}"
  printf '%s' "$value"
  exit 0
fi

# 2. Explicit environment, then what `gcp-use-mobioos` exports.
#
# WHY the conflict warning: a `.zshrc` export is ambient, while `PROJECT_ID` is set by an
# explicit `gcp-use-mobioos`. They disagree often, and silently choosing one is how an image
# workspace ended up calling a project that answers 403. Say it out loud, then save a choice
# (`make gcp-use`) so the question stops coming back.
if [ -n "${GOOGLE_CLOUD_PROJECT:-}" ] && [ -n "${PROJECT_ID:-}" ] \
  && [ "$GOOGLE_CLOUD_PROJECT" != "$PROJECT_ID" ]; then
  warn "Two Vertex projects are set: GOOGLE_CLOUD_PROJECT=$GOOGLE_CLOUD_PROJECT and PROJECT_ID=$PROJECT_ID."
  warn "Using GOOGLE_CLOUD_PROJECT. Pin one for good with: make gcp-use PROJECT=<id>"
fi
if [ -n "${GOOGLE_CLOUD_PROJECT:-}" ]; then
  info "Vertex project: ${_cyan}$GOOGLE_CLOUD_PROJECT${_reset} ${_dim}(from GOOGLE_CLOUD_PROJECT)${_reset}"
  printf '%s' "$GOOGLE_CLOUD_PROJECT"
  exit 0
fi
if [ -n "${PROJECT_ID:-}" ]; then
  info "Vertex project: ${_cyan}$PROJECT_ID${_reset} ${_dim}(from PROJECT_ID)${_reset}"
  printf '%s' "$PROJECT_ID"
  exit 0
fi

# 3. First run. Ask the human, but only when a human is there.
if _interactive; then
  if value="$(_prompt)"; then
    printf '%s' "$value"
    exit 0
  fi
  exit 1
fi

warn "Vertex project: not set (image embedding unavailable; run: make gcp-use PROJECT=<id>)"
exit 0
