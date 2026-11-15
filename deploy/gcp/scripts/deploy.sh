#!/usr/bin/env bash
# SPEC-148 deploy: pull images, LD-15 migrate, compose up, HTTPS + \dx gates.
# Migration 171 (drop Apache AGE) runs only with EDGEQUAKE_DEPLOY_DROP_AGE
# or a yes on a real terminal. A version pin is not consent.
set -euo pipefail

_DEPLOY_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=deploy/gcp/scripts/age_drop_policy.sh
source "${_DEPLOY_SCRIPT_DIR}/age_drop_policy.sh"

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
  local i
  for i in $(seq 1 60); do
    if "${COMPOSE[@]}" exec -T postgres pg_isready -U edgequake -d edgequake >/dev/null 2>&1; then
      return 0
    fi
    if [[ "${i}" -eq 60 ]]; then
      log "postgres not ready"
      "${COMPOSE[@]}" logs postgres | tail -50 | tee -a "${LOG}"
      return 1
    fi
    sleep 2
  done
}

# v0.35.4 is the last published postgres image that ships age.so.
# DROP EXTENSION on a later image fails with "could not access file age".
AGE_LIBRARY_IMAGE_TAG="0.35.4"

query_age_installed() {
  local raw
  raw="$("${COMPOSE[@]}" exec -T postgres psql -U edgequake -d edgequake -tAc \
    "SELECT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age')")"
  printf '%s' "${raw}" | tr -d '[:space:]'
}

# consent is "env" (re-checks EDGEQUAKE_DEPLOY_DROP_AGE) or "terminal"
# (age_drop_decision already returned drop). Any other argument refuses.
drop_age_using_library_image() {
  local consent="${1:?consent required}"
  local target_version="${EDGEQUAKE_VERSION}"
  local target_pg_tag="${EDGEQUAKE_POSTGRES_TAG:-${target_version}}"
  local age_installed
  case "${consent}" in
    env)
      if ! age_drop_env_consented; then
        log "refusing migrate --drop-age: EDGEQUAKE_DEPLOY_DROP_AGE is not set"
        return 1
      fi
      ;;
    terminal) ;;
    *)
      log "refusing migrate --drop-age without operator consent"
      return 1
      ;;
  esac
  log "dropping Apache AGE with the ${AGE_LIBRARY_IMAGE_TAG} image before ${target_version} opens this volume (consent: ${consent})"
  docker pull "ghcr.io/raphaelmansuy/edgequake-postgres:${AGE_LIBRARY_IMAGE_TAG}"
  docker pull "ghcr.io/raphaelmansuy/edgequake:${AGE_LIBRARY_IMAGE_TAG}"
  EDGEQUAKE_POSTGRES_TAG="${AGE_LIBRARY_IMAGE_TAG}" EDGEQUAKE_VERSION="${AGE_LIBRARY_IMAGE_TAG}" \
    "${COMPOSE[@]}" up -d postgres
  wait_postgres
  age_installed="$(query_age_installed)"
  if [[ "${age_installed}" != "t" ]]; then
    log "Apache AGE is not installed; not running migrate --drop-age"
  else
    log "running migrate --drop-age (migration 171, irreversible)"
    EDGEQUAKE_POSTGRES_TAG="${AGE_LIBRARY_IMAGE_TAG}" EDGEQUAKE_VERSION="${AGE_LIBRARY_IMAGE_TAG}" \
      "${COMPOSE[@]}" run --rm --no-deps -T api migrate --drop-age
  fi
  log "starting ${target_version} postgres"
  EDGEQUAKE_POSTGRES_TAG="${target_pg_tag}" EDGEQUAKE_VERSION="${target_version}" \
    "${COMPOSE[@]}" up -d postgres
  wait_postgres
}

log_age_left_installed() {
  log "Apache AGE is installed. ${EDGEQUAKE_VERSION} images do not ship age.so."
  log "Leaving it installed. This deploy will not run migrate --drop-age."
  log "Safe schema migration still runs. Migration 171 stays pending."
  log "To drop it (irreversible — take a backup first), re-run with EDGEQUAKE_DEPLOY_DROP_AGE=yes"
  log "Or on the ${AGE_LIBRARY_IMAGE_TAG} image: edgequake migrate --drop-age"
}

log_age_volume_wont_open() {
  log "Images from v0.35.5 have no age.so. A data directory that already loaded Apache AGE cannot open on this image."
  log "This deploy does not infer consent to drop it from the version pin."
  log "Back up, then re-run with EDGEQUAKE_DEPLOY_DROP_AGE=yes, or drop it on the ${AGE_LIBRARY_IMAGE_TAG} image: edgequake migrate --drop-age"
}

log "start postgres"
age_step="$(age_deploy_step "${EDGEQUAKE_VERSION}")"
case "${age_step}" in
  drop-first)
    log "consent: EDGEQUAKE_DEPLOY_DROP_AGE (migration 171 may run)"
    drop_age_using_library_image env
    ;;
  *)
    if [[ "${age_step}" != "decide" && "${age_step}" != "none" ]]; then
      log "refusing migrate --drop-age (unrecognized age step '${age_step}')"
    fi
    "${COMPOSE[@]}" up -d postgres
    log "wait for postgres healthy"
    if ! wait_postgres; then
      if [[ "${age_step}" == "decide" ]]; then
        log_age_volume_wont_open
      fi
      exit 1
    fi
    if [[ "${age_step}" == "decide" ]]; then
      age_installed="$(query_age_installed)"
      age_decision="$(age_drop_decision "${EDGEQUAKE_VERSION}" "${age_installed}")"
      case "${age_decision}" in
        drop)
          log "consent: terminal yes (migration 171 may run)"
          drop_age_using_library_image terminal
          ;;
        keep)
          if [[ "${age_installed}" == "t" ]]; then
            log_age_left_installed
          fi
          ;;
        *)
          log "refusing migrate --drop-age (unrecognized decision '${age_decision}')"
          if [[ "${age_installed}" == "t" ]]; then
            log_age_left_installed
          fi
          ;;
      esac
    fi
    ;;
esac

# Non-zero means the preview did not finish (connect, ledger, or image).
# Drop-readiness RED is informational and still exits 0 in the CLI.
# Do not swallow a failure with `|| true`: apply must not run on an
# unverified schema.
log "LD-15 migrate dry-run"
if ! "${COMPOSE[@]}" run --rm --no-deps -T api migrate dry-run; then
  log "FAIL: migration dry-run failed"
  exit 1
fi

log "LD-15 migrate apply (schema only; does not pass --drop-age)"
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
