#!/usr/bin/env bash
# SPEC-017 LaTeX markdown E2E proof runner
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../../../../.." && pwd)"
WEBUI="${ROOT}/edgequake_webui"

export PLAYWRIGHT_SKIP_STACK_CHECK="${PLAYWRIGHT_SKIP_STACK_CHECK:-1}"
export PLAYWRIGHT_BASE_URL="${PLAYWRIGHT_BASE_URL:-http://localhost:3001}"

cd "${WEBUI}"
bunx playwright test e2e/spec017-markdown-latex.spec.ts "$@"
