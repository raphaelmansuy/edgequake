#!/usr/bin/env bash
# SPEC-148 deploy: pull images, LD-15 migrate, compose up, HTTPS + \dx gates.
set -euo pipefail

ROOT="${STACK_ROOT:-/opt/edgequake}"
COMPOSE_DIR="${ROOT}/compose"
ENV_FILE="${ROOT}/.env"
LOG="${STACK_LOG:-/var/log/edgequake-deploy.log}"

log() { echo "[$(date -Is)] $*" | tee -a "${LOG}"; }

if [[ "${EUID}" -ne 0 ]]; then
  echo "deploy.sh must run as root (sudo)" >&2
  exit 1
fi

mkdir -p "$(dirname "${LOG}")"
cd "${COMPOSE_DIR}"

if [[ ! -f "${ENV_FILE}" ]]; then
  log "missing ${ENV_FILE}; running render-env.sh"
  # shellcheck disable=SC1091
  source "${ROOT}/scripts/load-metadata.sh"
  STACK_ENV_FILE="${ENV_FILE}" "${ROOT}/scripts/render-env.sh"
fi

set -a
# shellcheck disable=SC1090
source "${ENV_FILE}"
set +a

if [[ -n "${EDGEQUAKE_PIN_VERSION:-}" ]]; then
  EDGEQUAKE_VERSION="${EDGEQUAKE_PIN_VERSION}"
  EDGEQUAKE_POSTGRES_TAG="${EDGEQUAKE_PIN_VERSION}"
  sed -i "s/^EDGEQUAKE_VERSION=.*/EDGEQUAKE_VERSION=${EDGEQUAKE_PIN_VERSION}/" "${ENV_FILE}"
  sed -i "s/^EDGEQUAKE_POSTGRES_TAG=.*/EDGEQUAKE_POSTGRES_TAG=${EDGEQUAKE_PIN_VERSION}/" "${ENV_FILE}"
fi

if [[ -n "${EDGEQUAKE_HOSTNAME:-}" ]]; then
  RESOLVED="$(python3 - <<'PY' "${EDGEQUAKE_HOSTNAME}"
import socket, sys
host = sys.argv[1]
try:
    print(socket.getaddrinfo(host, 443, type=socket.SOCK_STREAM)[0][4][0])
except OSError:
    print("")
PY
)"
  if [[ -n "${RESOLVED}" && "${RESOLVED}" == "${EDGEQUAKE_TLS_HOST}" ]]; then
    cp -f "${COMPOSE_DIR}/Caddyfile.hostname" "${COMPOSE_DIR}/Caddyfile.runtime"
    CADDY_MODE="le"
    log "Caddy Let's Encrypt for ${EDGEQUAKE_HOSTNAME} (${RESOLVED})"
  else
    cp -f "${COMPOSE_DIR}/Caddyfile" "${COMPOSE_DIR}/Caddyfile.runtime"
    CADDY_MODE="ip"
    log "DNS for ${EDGEQUAKE_HOSTNAME} is '${RESOLVED:-unresolved}', expected ${EDGEQUAKE_TLS_HOST}; keeping IP certificate"
  fi
else
  cp -f "${COMPOSE_DIR}/Caddyfile" "${COMPOSE_DIR}/Caddyfile.runtime"
  CADDY_MODE="ip"
fi

log "mint IP-SAN TLS cert (empty SNI + Docker NAT; caddy#6344)"
STACK_CERT_DIR="${COMPOSE_DIR}/certs" "${ROOT}/scripts/generate-tls.sh"

export COMPOSE_PROJECT_NAME=edgequake
COMPOSE=(docker compose --env-file "${ENV_FILE}" -f "${COMPOSE_DIR}/docker-compose.yml")

# 20G boot disk: reclaim unused layers + build cache before pull.
# Do not prune named volumes (postgres data lives on /mnt).
log "docker reclaim (images + build cache + stopped containers)"
docker container prune -f 2>&1 | tee -a "${LOG}" || true
docker builder prune -af 2>&1 | tee -a "${LOG}" || true
docker image prune -af 2>&1 | tee -a "${LOG}" || true
# Drop leftover extract snapshots from failed pulls (containerd overlayfs).
if command -v ctr >/dev/null 2>&1; then
  ctr -n moby content prune --keep=false 2>&1 | tee -a "${LOG}" || true
fi
df -h / /var/lib/containerd 2>&1 | tee -a "${LOG}" || true
df -i / 2>&1 | tee -a "${LOG}" || true

log "compose pull (EDGEQUAKE_VERSION=${EDGEQUAKE_VERSION})"
"${COMPOSE[@]}" pull

wait_postgres() {
  for i in $(seq 1 60); do
    if "${COMPOSE[@]}" exec -T postgres pg_isready -U edgequake -d edgequake >/dev/null 2>&1; then
      return 0
    fi
    if [[ "${i}" -eq 60 ]]; then
      log "postgres not ready"
      "${COMPOSE[@]}" logs postgres | tail -50 | tee -a "${LOG}"
      exit 1
    fi
    sleep 2
  done
}

# 0.35.5 postgres images have no age.so. DROP EXTENSION on that image fails
# with "could not access file age" when the volume still has the extension.
# Drop it on the last image that ships the library, then open the new image.
postgres_has_no_age_library() {
  python3 - <<'PY' "${EDGEQUAKE_VERSION}"
import sys
parts = tuple(int(x) for x in sys.argv[1].split("."))
raise SystemExit(0 if parts >= (0, 35, 5) else 1)
PY
}

log "start postgres"
"${COMPOSE[@]}" up -d postgres

log "wait for postgres healthy"
wait_postgres

if postgres_has_no_age_library; then
  age_installed="$("${COMPOSE[@]}" exec -T postgres psql -U edgequake -d edgequake -tAc \
    "SELECT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age')")"
  age_installed="$(printf '%s' "${age_installed}" | tr -d '[:space:]')"
  if [[ "${age_installed}" == "t" ]]; then
    log "dropping Apache AGE with the 0.35.4 image before ${EDGEQUAKE_VERSION} opens this volume"
    docker pull ghcr.io/raphaelmansuy/edgequake-postgres:0.35.4
    docker pull ghcr.io/raphaelmansuy/edgequake:0.35.4
    EDGEQUAKE_POSTGRES_TAG=0.35.4 EDGEQUAKE_VERSION=0.35.4 "${COMPOSE[@]}" up -d postgres
    wait_postgres
    EDGEQUAKE_POSTGRES_TAG=0.35.4 EDGEQUAKE_VERSION=0.35.4 \
      "${COMPOSE[@]}" run --rm --no-deps -T api migrate --drop-age
    log "Apache AGE dropped; starting ${EDGEQUAKE_VERSION} postgres"
    "${COMPOSE[@]}" up -d postgres
    wait_postgres
  fi
fi

log "LD-15 migrate dry-run"
"${COMPOSE[@]}" run --rm --no-deps -T api migrate dry-run || true

log "LD-15 migrate apply"
"${COMPOSE[@]}" run --rm --no-deps -T api migrate

log "compose up"
"${COMPOSE[@]}" up -d

# File binds keep the inode from container start; install-release replaces
# compose/ so Caddy must be recreated to load the new snippets.caddy.
log "recreate Caddy (bind-mount config refresh)"
"${COMPOSE[@]}" up -d --force-recreate --no-deps caddy

log "wait for api healthcheck"
for i in $(seq 1 60); do
  if docker inspect --format='{{.State.Health.Status}}' edgequake-api 2>/dev/null | grep -q healthy; then
    break
  fi
  if [[ "${i}" -eq 60 ]]; then
    log "api not healthy"
    "${COMPOSE[@]}" logs api | tail -80 | tee -a "${LOG}"
    exit 1
  fi
  sleep 3
done

log "wait for Caddy :80"
redir=""
http_args=(curl -sI --max-time 5)
https_args=(curl -sf --max-time 15)
https_tries=20
if [[ "${CADDY_MODE}" == "le" ]]; then
  http_args+=(-H "Host: ${EDGEQUAKE_HOSTNAME}")
  https_args=(curl -sf --max-time 15 --resolve "${EDGEQUAKE_HOSTNAME}:443:127.0.0.1" "https://${EDGEQUAKE_HOSTNAME}/health")
  https_tries=40
else
  https_args=(curl -skf --max-time 15 https://127.0.0.1/health)
fi
for i in $(seq 1 30); do
  redir="$("${http_args[@]}" http://127.0.0.1/health 2>/dev/null || true)"
  if echo "${redir}" | grep -qiE '^HTTP/.* (301|308)\b'; then
    break
  fi
  sleep 2
done
log "E2E-148-04 HTTP redirect"
echo "${redir}" | tee -a "${LOG}"
echo "${redir}" | grep -qiE '^HTTP/.* (301|308)\b' || { log "FAIL: HTTP was not 301/308"; exit 1; }
echo "${redir}" | grep -qiE '^Location:[[:space:]]*https://' || { log "FAIL: Location is not https"; exit 1; }

log "E2E-148-05 HTTPS /health"
https_ok=0
for i in $(seq 1 "${https_tries}"); do
  if "${https_args[@]}" | tee -a "${LOG}"; then
    echo | tee -a "${LOG}"
    https_ok=1
    break
  fi
  sleep 2
done
if [[ "${https_ok}" -ne 1 ]]; then
  log "FAIL: HTTPS /health"
  "${COMPOSE[@]}" logs caddy | tail -40 | tee -a "${LOG}"
  exit 1
fi

# Path-collision gate: /api* would steal WebUI /api-explorer (SPEC-035).
log "E2E-148-06 /api-explorer → frontend; /api/v1/* → api"
if [[ "${CADDY_MODE}" == "le" ]]; then
  explorer_hdr="$(curl -sI --max-time 15 --resolve "${EDGEQUAKE_HOSTNAME}:443:127.0.0.1" \
    "https://${EDGEQUAKE_HOSTNAME}/api-explorer" 2>/dev/null || true)"
  api_hdr="$(curl -sI --max-time 15 --resolve "${EDGEQUAKE_HOSTNAME}:443:127.0.0.1" \
    "https://${EDGEQUAKE_HOSTNAME}/api/v1/auth/me" 2>/dev/null || true)"
else
  explorer_hdr="$(curl -skI --max-time 15 https://127.0.0.1/api-explorer 2>/dev/null || true)"
  api_hdr="$(curl -skI --max-time 15 https://127.0.0.1/api/v1/auth/me 2>/dev/null || true)"
fi
echo "${explorer_hdr}" | tee -a "${LOG}"
echo "${api_hdr}" | tee -a "${LOG}"
# Frontend owns /api-explorer (SPEC-035): Next 200 HTML or auth 3xx to /login.
# A Caddy /api* steal would be Axum JSON 401 with no HTML.
echo "${explorer_hdr}" | grep -qiE '^HTTP/.* (200|302|307)\b' \
  || { log "FAIL: /api-explorer not served by frontend (got non-200/3xx)"; exit 1; }
if echo "${explorer_hdr}" | grep -qiE '^HTTP/.* (302|307)\b'; then
  echo "${explorer_hdr}" | grep -qiE '^[Ll]ocation:[[:space:]]*.*/login' \
    || { log "FAIL: /api-explorer Location is not /login (Caddy /api* collision?)"; exit 1; }
else
  echo "${explorer_hdr}" | grep -qiE '^[Cc]ontent-[Tt]ype:[[:space:]]*text/html' \
    || { log "FAIL: /api-explorer 200 is not HTML (Caddy /api* collision?)"; exit 1; }
fi
# API still owns /api/v1/*: expect auth challenge, not Next login redirect.
echo "${api_hdr}" | grep -qiE '^HTTP/.* (401|403)\b' \
  || { log "FAIL: /api/v1/auth/me not served by api (expected 401/403)"; exit 1; }
echo "${api_hdr}" | grep -qiE '^[Ll]ocation:[[:space:]]*.*/login' \
  && { log "FAIL: /api/v1/auth/me redirected to frontend login"; exit 1; } || true

log "E2E-148-03 extensions"
dx="$("${COMPOSE[@]}" exec -T postgres psql -U edgequake -d edgequake -c '\dx')"
echo "${dx}" | tee -a "${LOG}"
echo "${dx}" | grep -qw vector || { log "FAIL: vector extension missing"; exit 1; }
if echo "${dx}" | grep -qw age; then
  log "Apache AGE is installed"
else
  log "Apache AGE is not installed"
fi

log "DEPLOY_OK"
