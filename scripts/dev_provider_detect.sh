#!/usr/bin/env bash
# Detect which LLM provider `make dev` should use, from the environment alone.
#
# Why this exists
#   `make dev` used to check only OPENAI_API_KEY (and MISTRAL_API_KEY in one status
#   block), so a developer who exported GEMINI_API_KEY, ANTHROPIC_API_KEY or
#   XAI_API_KEY was silently dropped to Ollama — and then saw a first-run wizard
#   offering models whose provider had no key. The backend has always honoured all
#   five keys (see `edgequake-llm` factory + `providers/credentials.rs`); the Makefile
#   was the only place that did not.
#
#   One script, one precedence order, used by every dev target, so the greeting and
#   the status table cannot disagree again.
#
# Precedence (first match wins), mirroring `ProviderFactory::from_env`:
#   1. EDGEQUAKE_LLM_PROVIDER, when set explicitly — the operator knows best.
#   2. OPENAI_API_KEY      -> openai
#   3. ANTHROPIC_API_KEY   -> anthropic
#   4. GEMINI_API_KEY|GOOGLE_API_KEY -> gemini
#   5. XAI_API_KEY         -> xai
#   6. MISTRAL_API_KEY     -> mistral
#   7. otherwise           -> ollama (local, no key needed)
#
# Output: one line, `<provider> <model> <origin>`, so a shell caller can read all
# three without re-deriving them. `origin` is `env` or `fallback`.
#
# Usage: scripts/dev_provider_detect.sh [--explain]
#   --explain  print a human sentence to stderr as well.

set -euo pipefail

_explain=0
[ "${1:-}" = "--explain" ] && _explain=1

_say() { [ "$_explain" = "1" ] && printf '%s\n' "$*" >&2; return 0; }

# A key counts only when it is set and not blank; an exported empty string is a
# common shell artefact and must not select a provider.
_has() {
  local v="${!1:-}"
  [ -n "${v//[[:space:]]/}" ]
}

_explicit_model() {
  local v="${1:-}"
  printf '%s' "${v//[[:space:]]/}"
}

# Defaults are the same ones the Makefile uses; overridable so a developer can pin
# a different model without editing this script.
_llm_default() {
  local provider="$1"
  case "$provider" in
    openai)    _explicit_model "${EDGEQUAKE_DEFAULT_LLM_MODEL_OPENAI:-gpt-6-luna}" ;;
    anthropic) _explicit_model "${EDGEQUAKE_DEFAULT_LLM_MODEL_ANTHROPIC:-claude-sonnet-5-5}" ;;
    gemini)    _explicit_model "${EDGEQUAKE_DEFAULT_LLM_MODEL_GEMINI:-gemini-3.8-flash}" ;;
    xai)       _explicit_model "${EDGEQUAKE_DEFAULT_LLM_MODEL_XAI:-grok-4.7}" ;;
    mistral)   _explicit_model "${EDGEQUAKE_DEFAULT_LLM_MODEL_MISTRAL:-mistral-large-latest}" ;;
    *)         _explicit_model "${OLLAMA_MODEL:-${EDGEQUAKE_DEFAULT_LLM_MODEL:-gemma4:latest}}" ;;
  esac
}

_choose() {
  local forced="${EDGEQUAKE_LLM_PROVIDER:-}"
  if [ -n "${forced//[[:space:]]/}" ]; then
    printf '%s env' "$forced"
    return
  fi
  if _has OPENAI_API_KEY;    then printf 'openai env';    return; fi
  if _has ANTHROPIC_API_KEY; then printf 'anthropic env'; return; fi
  if _has GEMINI_API_KEY || _has GOOGLE_API_KEY; then printf 'gemini env'; return; fi
  if _has XAI_API_KEY;       then printf 'xai env';       return; fi
  if _has MISTRAL_API_KEY;   then printf 'mistral env';   return; fi
  printf 'ollama fallback'
}

read -r provider origin <<<"$(_choose)"
model="$(_llm_default "$provider")"

case "$provider" in
  ollama)
    _say "No provider key found, so local Ollama is used."
    _say "  Export any of OPENAI_API_KEY, ANTHROPIC_API_KEY, GEMINI_API_KEY, XAI_API_KEY"
    _say "  (or MISTRAL_API_KEY) and rerun to use a hosted model instead."
    ;;
  *)
    _say "Using $provider (detected from the environment)."
    ;;
esac

printf '%s %s %s\n' "$provider" "$model" "$origin"
