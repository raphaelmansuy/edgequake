#!/usr/bin/env bash
# Render /opt/edgequake/.env from instance metadata + Secret Manager.
# LAW-148-6: secrets never logged.
set -euo pipefail

PREFIX="${STACK_PREFIX:-edgequake}"
PROJECT="${STACK_PROJECT:-}"
ORIGIN="${STACK_PUBLIC_ORIGIN:-}"
VERSION="${STACK_VERSION:-0.27.0}"
HOSTNAME="${STACK_HOSTNAME:-}"
ACME_EMAIL="${STACK_ACME_EMAIL:-}"
INSTALL_ROOT="${STACK_INSTALL_ROOT:-/opt/${PREFIX}}"
DATA_DISK="${STACK_DATA_DISK:-elitizon-db-data}"

if [[ -z "${PROJECT}" || -z "${ORIGIN}" ]]; then
  if [[ -f "${INSTALL_ROOT}/scripts/load-metadata.sh" ]]; then
    # shellcheck disable=SC1091
    source "${INSTALL_ROOT}/scripts/load-metadata.sh"
    PROJECT="${STACK_PROJECT:-${PROJECT}}"
    ORIGIN="${STACK_PUBLIC_ORIGIN:-${ORIGIN}}"
    VERSION="${STACK_VERSION:-${VERSION}}"
    HOSTNAME="${STACK_HOSTNAME:-${HOSTNAME}}"
    ACME_EMAIL="${STACK_ACME_EMAIL:-${ACME_EMAIL}}"
    PREFIX="${STACK_PREFIX:-${PREFIX}}"
    INSTALL_ROOT="${STACK_INSTALL_ROOT:-${INSTALL_ROOT}}"
    DATA_DISK="${STACK_DATA_DISK:-${DATA_DISK}}"
  fi
fi
if [[ -z "${PROJECT}" ]]; then
  PROJECT="$(curl -sf -H "Metadata-Flavor: Google" \
    http://metadata.google.internal/computeMetadata/v1/project/project-id)"
fi
if [[ -z "${ORIGIN}" ]]; then
  echo "render-env.sh: STACK_PUBLIC_ORIGIN is empty" >&2
  exit 1
fi

# File cert is for the public IP (empty SNI). The hostname uses Let's Encrypt.
EXTERNAL_IP="$(curl -sf -H "Metadata-Flavor: Google" \
  "http://metadata.google.internal/computeMetadata/v1/instance/network-interfaces/0/access-configs/0/external-ip" || true)"
TLS_HOST="${EXTERNAL_IP:-${ORIGIN#https://}}"

access_secret() {
  local name="$1"
  gcloud secrets versions access latest --secret="${name}" --project="${PROJECT}"
}

# Sets SECRETS_KEY. Creates the host file once. Never replaces a different key.
resolve_secrets_key() {
  local host_key=""
  SECRETS_KEY_FILE="${INSTALL_ROOT}/secrets/edgequake-secrets-key"
  local err rc
  err="$(mktemp)"
  set +e
  SECRETS_KEY="$(access_secret "${PREFIX}-secrets-key" 2>"${err}")"
  rc=$?
  set -e
  if [[ "${rc}" -ne 0 ]]; then
    if grep -Eq 'PERMISSION_DENIED|IAM_PERMISSION_DENIED' "${err}"; then
      echo "render-env.sh: the runtime service account cannot read ${PREFIX}-secrets-key" >&2
      rm -f "${err}"
      exit 1
    fi
    SECRETS_KEY=""
  fi
  rm -f "${err}"
  SECRETS_KEY="${SECRETS_KEY//$'\n'/}"
  SECRETS_KEY="${SECRETS_KEY//$'\r'/}"
  if [[ -z "${SECRETS_KEY}" || "${SECRETS_KEY}" == "UNSET" ]]; then
    SECRETS_KEY=""
  fi
  if [[ -s "${SECRETS_KEY_FILE}" ]]; then
    host_key="$(tr -d '\n\r' < "${SECRETS_KEY_FILE}")"
  fi
  if [[ -n "${SECRETS_KEY}" && -n "${host_key}" && "${SECRETS_KEY}" != "${host_key}" ]]; then
    echo "render-env.sh: ${PREFIX}-secrets-key does not match ${SECRETS_KEY_FILE}. Refusing to rotate." >&2
    exit 1
  fi
  if [[ -z "${SECRETS_KEY}" ]]; then
    SECRETS_KEY="${host_key}"
  fi
  if [[ -z "${SECRETS_KEY}" ]]; then
    install -d -m 0700 "${INSTALL_ROOT}/secrets"
    umask 077
    openssl rand -hex 32 | tr -d '\n' > "${SECRETS_KEY_FILE}.tmp"
    chmod 600 "${SECRETS_KEY_FILE}.tmp"
    mv "${SECRETS_KEY_FILE}.tmp" "${SECRETS_KEY_FILE}"
    SECRETS_KEY="$(tr -d '\n\r' < "${SECRETS_KEY_FILE}")"
  elif [[ -z "${host_key}" ]]; then
    install -d -m 0700 "${INSTALL_ROOT}/secrets"
    umask 077
    printf '%s' "${SECRETS_KEY}" > "${SECRETS_KEY_FILE}.tmp"
    chmod 600 "${SECRETS_KEY_FILE}.tmp"
    mv "${SECRETS_KEY_FILE}.tmp" "${SECRETS_KEY_FILE}"
  fi
  if ! printf '%s' "${SECRETS_KEY}" | python3 -c '
import base64, binascii, sys
raw = sys.stdin.read().strip()
ok = False
if len(raw) == 64:
    try:
        ok = len(binascii.unhexlify(raw)) == 32
    except binascii.Error:
        ok = False
elif len(raw) == 32:
    ok = True
else:
    try:
        ok = len(base64.b64decode(raw, validate=True)) == 32
    except Exception:
        ok = False
sys.exit(0 if ok else 1)
'; then
    echo "render-env.sh: EDGEQUAKE_SECRETS_KEY must be 32 bytes (64 hex, base64, or 32 raw bytes)" >&2
    exit 1
  fi
}

upsert_env() {
  python3 - "$1" "$2" "$3" <<'PY'
import sys
path, key, value = sys.argv[1:]
lines = open(path).read().splitlines()
found = False
out = []
for line in lines:
    if line.startswith(key + "="):
        out.append(f"{key}={value}")
        found = True
    else:
        out.append(line)
if not found:
    out.append(f"{key}={value}")
open(path, "w").write("\n".join(out) + "\n")
PY
}

env_value() {
  python3 - "$1" "$2" <<'PY'
import sys
path, key = sys.argv[1:]
for line in open(path):
    if line.startswith(key + "="):
        sys.stdout.write(line.split("=", 1)[1].strip())
        break
PY
}

if [[ "${1:-}" == "--ensure" ]]; then
  ENV_FILE="${STACK_ENV_FILE:-${INSTALL_ROOT}/.env}"
  if [[ ! -f "${ENV_FILE}" ]]; then
    echo "render-env.sh: ${ENV_FILE} is missing" >&2
    exit 1
  fi
  existing="$(env_value "${ENV_FILE}" EDGEQUAKE_SECRETS_KEY)"
  if [[ -n "${existing}" ]]; then
    host_key=""
    if [[ -s "${INSTALL_ROOT}/secrets/edgequake-secrets-key" ]]; then
      host_key="$(tr -d '\n\r' < "${INSTALL_ROOT}/secrets/edgequake-secrets-key")"
    fi
    if [[ -n "${host_key}" && "${host_key}" != "${existing}" ]]; then
      echo "render-env.sh: ${ENV_FILE} does not match the host secrets key. Refusing to rotate." >&2
      exit 1
    fi
    if [[ -z "${host_key}" ]]; then
      install -d -m 0700 "${INSTALL_ROOT}/secrets"
      umask 077
      printf '%s' "${existing}" > "${INSTALL_ROOT}/secrets/edgequake-secrets-key.tmp"
      chmod 600 "${INSTALL_ROOT}/secrets/edgequake-secrets-key.tmp"
      mv "${INSTALL_ROOT}/secrets/edgequake-secrets-key.tmp" "${INSTALL_ROOT}/secrets/edgequake-secrets-key"
    fi
  else
    resolve_secrets_key
    upsert_env "${ENV_FILE}" EDGEQUAKE_SECRETS_KEY "${SECRETS_KEY}"
  fi
  if [[ -z "$(env_value "${ENV_FILE}" EDGEQUAKE_SECRETS_KEY_ID)" ]]; then
    upsert_env "${ENV_FILE}" EDGEQUAKE_SECRETS_KEY_ID v1
  fi
  chmod 600 "${ENV_FILE}"
  exit 0
fi

POSTGRES_PASSWORD="$(access_secret "${PREFIX}-postgres-password")"
JWT_SECRET="$(access_secret "${PREFIX}-jwt")"
BOOTSTRAP_ADMIN_PASSWORD="$(access_secret "${PREFIX}-bootstrap-admin-password")"
MASTER_API_KEY="$(access_secret "${PREFIX}-master-api-key")"
OPENAI_API_KEY="$(access_secret "${PREFIX}-openai-api-key" || true)"
MISTRAL_API_KEY="$(access_secret "${PREFIX}-mistral-api-key" || true)"
resolve_secrets_key
# Placeholder version is "UNSET" (Terraform). v0.27.0 forbids mock as the server
# default unless EDGEQUAKE_ALLOW_MOCK_PROVIDER=1 (test hatch; LAW-148 smoke only).
LLM_PROVIDER="mock"
ALLOW_MOCK="1"
if [[ -z "${OPENAI_API_KEY}" || "${OPENAI_API_KEY}" == "UNSET" ]]; then
  OPENAI_API_KEY=""
fi
if [[ -z "${MISTRAL_API_KEY}" || "${MISTRAL_API_KEY}" == "UNSET" ]]; then
  MISTRAL_API_KEY=""
fi
if [[ -n "${OPENAI_API_KEY}" ]]; then
  LLM_PROVIDER="openai"
  ALLOW_MOCK=""
elif [[ -n "${MISTRAL_API_KEY}" ]]; then
  LLM_PROVIDER="mistral"
  ALLOW_MOCK=""
fi

ENV_FILE="${STACK_ENV_FILE:-${INSTALL_ROOT}/.env}"
umask 077
cat >"${ENV_FILE}" <<EOF
EDGEQUAKE_VERSION=${VERSION}
EDGEQUAKE_POSTGRES_TAG=${VERSION}
POSTGRES_PASSWORD=${POSTGRES_PASSWORD}
JWT_SECRET=${JWT_SECRET}
EDGEQUAKE_BOOTSTRAP_ADMIN_USERNAME=admin
EDGEQUAKE_BOOTSTRAP_ADMIN_PASSWORD=${BOOTSTRAP_ADMIN_PASSWORD}
EDGEQUAKE_BOOTSTRAP_ADMIN_EMAIL=admin@elitizon.local
EDGEQUAKE_MASTER_API_KEY=${MASTER_API_KEY}
EDGEQUAKE_SECRETS_KEY=${SECRETS_KEY}
EDGEQUAKE_SECRETS_KEY_ID=v1
EDGEQUAKE_API_URL=${ORIGIN}
EDGEQUAKE_PUBLIC_URL=${ORIGIN}
EDGEQUAKE_CORS_ORIGINS=${ORIGIN}
EDGEQUAKE_TLS_HOST=${TLS_HOST}
ELITIZON_DATA_MOUNT=/mnt/${DATA_DISK}
EDGEQUAKE_DEV_MODE=false
EDGEQUAKE_AUTH_ENABLED=true
EDGEQUAKE_LLM_PROVIDER=${LLM_PROVIDER}
EDGEQUAKE_EMBEDDING_PROVIDER=${LLM_PROVIDER}
EDGEQUAKE_VISION_PROVIDER=${LLM_PROVIDER}
EDGEQUAKE_ALLOW_MOCK_PROVIDER=${ALLOW_MOCK}
OPENAI_API_KEY=${OPENAI_API_KEY}
MISTRAL_API_KEY=${MISTRAL_API_KEY}
LLM_API_KEY=${MISTRAL_API_KEY}
EDGEQUAKE_HOSTNAME=${HOSTNAME}
CADDY_ACME_EMAIL=${ACME_EMAIL}
EOF
chmod 600 "${ENV_FILE}"
chown root:root "${ENV_FILE}"

STACK_ENV_FILE="${ENV_FILE}" STACK_PROJECT="${PROJECT}" STACK_INSTALL_ROOT="${INSTALL_ROOT}" \
  "$(dirname "$0")/render-originals.sh"
