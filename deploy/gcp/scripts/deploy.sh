#!/usr/bin/env bash
# SPEC-148 deploy: pull images, LD-15 migrate, compose up, HTTPS + \dx gates.
set -euo pipefail

ROOT="${STACK_ROOT:-/opt/edgequake}"
COMPOSE_DIR="${ROOT}/compose"
ENV_FILE="${ROOT}/.env"
LOG="${STACK_LOG:-/var/log/edgequake-deploy.log}"

log() { echo "[$(date -Is)] $*" | tee -a "${LOG}"; }

# Consent for the irreversible Apache AGE drop is read from the invoking shell
# before `${ENV_FILE}` is sourced, so a value inside a shared env file can never
# authorize it.
DEPLOY_AGE_DROP="${EDGEQUAKE_DEPLOY_AGE_DROP:-}"

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

# An existing .env is kept across deploys. Fill a missing envelope key in
# place so a later image roll still lets Connections save a credential.
log "ensure provider secrets key"
STACK_ENV_FILE="${ENV_FILE}" "${ROOT}/scripts/render-env.sh" --ensure

# A version bump must still attach the originals key. A missing secret
# leaves the server on database mode.
log "render originals key"
STACK_ENV_FILE="${ENV_FILE}" "${ROOT}/scripts/render-originals.sh"

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

# Dropping Apache AGE (migration 171) is irreversible: it removes the
# extension and every AGE graph, and no down-migration exists. Consent for it
# is a deliberate operator act, never an inference:
#
#   sudo EDGEQUAKE_DEPLOY_AGE_DROP=drop /opt/edgequake/scripts/deploy.sh
#
# Every other value — including unset — keeps Apache AGE and stops the deploy
# with that instruction instead of destroying data. A value in the shared
# `.env` (sourced below for the compose stack) does not count: on this host
# `.env` is rewritten by render-env.sh on every deploy, so only the invoking
# shell can carry the operator's decision.
age_drop_consented() {
  local raw
  raw="$(printf '%s' "${DEPLOY_AGE_DROP:-}" | tr '[:upper:]' '[:lower:]' | tr -d '[:space:]')"
  case "${raw}" in
    drop | yes | 1 | true | on) return 0 ;;
    *) return 1 ;;
  esac
}

# Apache AGE must be settled before the 0.35.5+ postgres image opens this
# volume: a data directory that loaded `age` cannot open on an image without
# age.so (docs/operations/upgrade-to-0.35.5.md). Only the 0.35.4 image can read
# it, so probe there. Reading the fact is safe; destroying the extension is
# irreversible, so migration 171 runs only when the operator says so on the
# command line.
AGE_CUTOVER_IMAGE="0.35.4"
PGDATA_DIR="${ELITIZON_DATA_MOUNT:-/mnt/elitizon-db-data}/pgdata"

age_installed_on_image() {
  local tag="$1" out
  out="$(EDGEQUAKE_POSTGRES_TAG="${tag}" EDGEQUAKE_VERSION="${tag}" \
    "${COMPOSE[@]}" exec -T postgres psql -U edgequake -d edgequake -tAc \
    "SELECT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age')" 2>/dev/null)" || out=""
  printf '%s' "${out}" | tr -d '[:space:]'
}

age_must_be_dropped=""
# An initialized data directory is the only case that can still carry the
# extension; a fresh one cannot, so greenfield deploys skip the probe.
if postgres_has_no_age_library && [[ -f "${PGDATA_DIR}/PG_VERSION" ]]; then
  log "probe Apache AGE on the ${AGE_CUTOVER_IMAGE} image (the last one with age.so)"
  docker pull "ghcr.io/raphaelmansuy/edgequake-postgres:${AGE_CUTOVER_IMAGE}"
  EDGEQUAKE_POSTGRES_TAG="${AGE_CUTOVER_IMAGE}" EDGEQUAKE_VERSION="${AGE_CUTOVER_IMAGE}" \
    "${COMPOSE[@]}" up -d postgres
  wait_postgres
  if [[ "$(age_installed_on_image "${AGE_CUTOVER_IMAGE}")" == "t" ]]; then
    if age_drop_consented; then
      age_must_be_dropped="t"
    else
      log "REFUSING to drop Apache AGE without operator consent"
      log "  this volume still has the age extension"
      log "  ${EDGEQUAKE_VERSION} has no age library, so it cannot open this volume"
      log "  migration 171 would drop the extension and every AGE graph. There is no undo SQL."
      log "  nothing was dropped; the stack is still on the ${AGE_CUTOVER_IMAGE} postgres image"
      log "  take a backup, then re-run and say so on the command line:"
      log "    sudo EDGEQUAKE_DEPLOY_AGE_DROP=drop ${ROOT}/scripts/deploy.sh"
      log "  to stay on Apache AGE, keep the ${AGE_CUTOVER_IMAGE} pin instead of ${EDGEQUAKE_VERSION}"
      exit 2
    fi
  fi
fi

# Drop before the requested image starts: while the extension is installed, only
# the cutover image can open this volume, so the new one must not try first.
if [[ "${age_must_be_dropped}" == "t" ]]; then
  log "operator consented (EDGEQUAKE_DEPLOY_AGE_DROP); dropping Apache AGE"
  docker pull "ghcr.io/raphaelmansuy/edgequake:${AGE_CUTOVER_IMAGE}"
  EDGEQUAKE_POSTGRES_TAG="${AGE_CUTOVER_IMAGE}" EDGEQUAKE_VERSION="${AGE_CUTOVER_IMAGE}" \
    "${COMPOSE[@]}" run --rm --no-deps -T api migrate --drop-age
  log "Apache AGE dropped; starting ${EDGEQUAKE_VERSION} postgres"
fi

log "start postgres"
"${COMPOSE[@]}" up -d postgres

log "wait for postgres healthy"
wait_postgres

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
