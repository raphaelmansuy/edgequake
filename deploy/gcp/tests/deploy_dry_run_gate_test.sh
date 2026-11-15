#!/usr/bin/env bash
# Issue 541: a failed LD-15 migrate dry-run must abort deploy.sh before apply.
# Runs the real script against a stub docker. Needs passwordless sudo (root check).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
DEPLOY="${ROOT}/deploy/gcp/scripts/deploy.sh"
fail=0

if grep -n 'migrate dry-run || true' "${DEPLOY}"; then
  echo "FAIL: deploy.sh still swallows migrate dry-run"
  exit 1
fi

if [[ "${EUID}" -ne 0 ]]; then
  exec sudo --preserve-env=PATH bash "$0" "$@"
fi

WORK="$(mktemp -d)"
trap 'rm -rf "${WORK}"' EXIT

STUB_BIN="${WORK}/bin"
mkdir -p "${STUB_BIN}" "${WORK}/stack/scripts" "${WORK}/stack/compose" "${WORK}/stack/secrets"

cp "${ROOT}/deploy/gcp/scripts/render-env.sh" \
  "${ROOT}/deploy/gcp/scripts/render-originals.sh" \
  "${ROOT}/deploy/gcp/scripts/generate-tls.sh" \
  "${WORK}/stack/scripts/"
cp "${ROOT}/deploy/gcp/compose/Caddyfile" "${WORK}/stack/compose/Caddyfile"
cp "${ROOT}/deploy/gcp/compose/docker-compose.yml" "${WORK}/stack/compose/docker-compose.yml"
chmod 0755 "${WORK}/stack/scripts/"*.sh

SECRETS_KEY="$(openssl rand -hex 32)"
cat >"${WORK}/stack/.env" <<EOF
EDGEQUAKE_VERSION=0.40.7
EDGEQUAKE_POSTGRES_TAG=0.40.7
POSTGRES_PASSWORD=change-me
JWT_SECRET=change-me-in-production-256-bit-secret-key
EDGEQUAKE_BOOTSTRAP_ADMIN_PASSWORD=change-me
EDGEQUAKE_MASTER_API_KEY=change-me
EDGEQUAKE_SECRETS_KEY=${SECRETS_KEY}
EDGEQUAKE_SECRETS_KEY_ID=v1
EDGEQUAKE_API_URL=https://203.0.113.10
EDGEQUAKE_CORS_ORIGINS=https://203.0.113.10
EDGEQUAKE_TLS_HOST=203.0.113.10
ELITIZON_DATA_MOUNT=${WORK}/pg
EDGEQUAKE_LLM_PROVIDER=mock
EDGEQUAKE_ALLOW_MOCK_PROVIDER=1
OPENAI_API_KEY=
MISTRAL_API_KEY=
EDGEQUAKE_DECISION_ENABLED=0
EOF
chmod 600 "${WORK}/stack/.env"

cat >"${STUB_BIN}/gcloud" <<'EOF'
#!/usr/bin/env bash
exit 1
EOF

cat >"${STUB_BIN}/curl" <<'EOF'
#!/usr/bin/env bash
# Metadata and health probes must not leave this machine.
exit 1
EOF

cat >"${STUB_BIN}/docker" <<'EOF'
#!/usr/bin/env bash
set -u
trace="${DOCKER_TRACE:?}"
printf '%s\n' "$*" >>"${trace}"

if [[ "${1:-}" != "compose" ]]; then
  exit 0
fi

joined=" $* "
if [[ "${joined}" == *" pg_isready "* ]]; then
  exit 0
fi
if [[ "${joined}" == *" pg_extension "* ]]; then
  printf 'f\n'
  exit 0
fi
if [[ "${joined}" == *" migrate dry-run "* ]]; then
  if [[ "${STUB_DRY_RUN:-fail}" == "ok" ]]; then
    echo "dry-run complete: no migrations applied (preview only)."
    echo "drop-readiness RED (informational)"
    exit 0
  fi
  echo "[stub] migrate dry-run simulated failure" >&2
  exit "${STUB_DRY_RUN_CODE:-1}"
fi
if [[ "${joined}" == *" migrate "* ]]; then
  echo "[stub] LD-15 migrate apply"
  exit 0
fi
if [[ "${joined}" == *" up -d postgres "* ]]; then
  exit 0
fi
if [[ "${joined}" == *" pull "* || "${joined}" == *" pull" ]]; then
  exit 0
fi
if [[ "${joined}" == *" up -d "* || "${joined}" == *" up -d" ]]; then
  echo "[stub] compose up" >&2
  exit 86
fi
echo "[stub] unexpected docker compose: $*" >&2
exit 97
EOF
chmod 0755 "${STUB_BIN}/docker" "${STUB_BIN}/gcloud" "${STUB_BIN}/curl"

run_deploy() {
  local mode="$1"
  local code="${2:-1}"
  local trace="${WORK}/trace-${mode}-${code}"
  local log="${WORK}/deploy-${mode}-${code}.log"
  local out="${WORK}/out-${mode}-${code}.txt"
  : >"${trace}"
  chmod 666 "${trace}"
  set +e
  PATH="${STUB_BIN}:${PATH}" \
    STACK_ROOT="${WORK}/stack" \
    STACK_INSTALL_ROOT="${WORK}/stack" \
    STACK_PROJECT="test-project" \
    STACK_PUBLIC_ORIGIN="https://203.0.113.10" \
    STACK_LOG="${log}" \
    DOCKER_TRACE="${trace}" \
    STUB_DRY_RUN="${mode}" \
    STUB_DRY_RUN_CODE="${code}" \
    "${DEPLOY}" >"${out}" 2>&1
  local rc=$?
  set -e
  echo "${rc}" >"${WORK}/rc-${mode}-${code}"
  cp "${out}" "${WORK}/last-out.txt"
  cp "${trace}" "${WORK}/last-trace.txt"
}

assert_absent() {
  local file="$1"
  local needle="$2"
  local label="$3"
  if grep -qF "${needle}" "${file}"; then
    echo "FAIL: ${label}: found unexpected '${needle}'"
    fail=1
  fi
}

assert_present() {
  local file="$1"
  local needle="$2"
  local label="$3"
  if ! grep -qF "${needle}" "${file}"; then
    echo "FAIL: ${label}: missing '${needle}'"
    fail=1
  fi
}

echo "== dry-run exit 1 aborts before apply =="
run_deploy fail 1
rc="$(cat "${WORK}/rc-fail-1")"
out="${WORK}/out-fail-1.txt"
trace="${WORK}/trace-fail-1"
if [[ "${rc}" -ne 1 ]]; then
  echo "FAIL: expected exit 1, got ${rc}"
  fail=1
fi
assert_present "${out}" "[stub] migrate dry-run simulated failure" "fail-1 stdout"
assert_present "${out}" "FAIL: migration dry-run failed" "fail-1 stdout"
assert_present "${out}" "LD-15 migrate dry-run" "fail-1 stdout"
assert_absent "${out}" "LD-15 migrate apply" "fail-1 stdout"
assert_absent "${out}" "DEPLOY_OK" "fail-1 stdout"
assert_absent "${out}" "[stub] compose up" "fail-1 stdout"
if grep -E 'api migrate$' "${trace}" | grep -vq 'dry-run'; then
  echo "FAIL: fail-1 trace reached migrate apply"
  sed -n '1,200p' "${trace}"
  fail=1
fi
if ! grep -q 'migrate dry-run' "${trace}"; then
  echo "FAIL: fail-1 trace never ran dry-run"
  fail=1
fi

echo "== dry-run exit 2 also aborts before apply =="
run_deploy fail 2
rc="$(cat "${WORK}/rc-fail-2")"
out="${WORK}/out-fail-2.txt"
trace="${WORK}/trace-fail-2"
if [[ "${rc}" -ne 1 ]]; then
  echo "FAIL: expected exit 1 when dry-run exits 2, got ${rc}"
  fail=1
fi
assert_present "${out}" "FAIL: migration dry-run failed" "fail-2 stdout"
assert_absent "${out}" "LD-15 migrate apply" "fail-2 stdout"
assert_absent "${out}" "DEPLOY_OK" "fail-2 stdout"
if grep -E 'api migrate$' "${trace}" | grep -vq 'dry-run'; then
  echo "FAIL: fail-2 trace reached migrate apply"
  fail=1
fi

echo "== dry-run exit 0 (RED readiness) still applies =="
run_deploy ok 0
rc="$(cat "${WORK}/rc-ok-0")"
out="${WORK}/out-ok-0.txt"
trace="${WORK}/trace-ok-0"
if [[ "${rc}" -ne 86 ]]; then
  echo "FAIL: expected exit 86 from stubbed compose up, got ${rc}"
  fail=1
fi
assert_present "${out}" "drop-readiness RED (informational)" "ok stdout"
assert_present "${out}" "LD-15 migrate apply" "ok stdout"
assert_present "${out}" "[stub] LD-15 migrate apply" "ok stdout"
assert_present "${out}" "[stub] compose up" "ok stdout"
assert_absent "${out}" "FAIL: migration dry-run failed" "ok stdout"
assert_absent "${out}" "DEPLOY_OK" "ok stdout"
dry_line="$(grep -n 'migrate dry-run' "${trace}" | head -1 | cut -d: -f1 || true)"
apply_line="$(grep -nE 'api migrate$' "${trace}" | head -1 | cut -d: -f1 || true)"
up_line="$(grep -n ' up -d$' "${trace}" | head -1 | cut -d: -f1 || true)"
if [[ -z "${dry_line}" || -z "${apply_line}" || -z "${up_line}" ]]; then
  echo "FAIL: ok trace missing dry-run, apply, or compose up"
  sed -n '1,200p' "${trace}"
  fail=1
elif [[ "${dry_line}" -ge "${apply_line}" || "${apply_line}" -ge "${up_line}" ]]; then
  echo "FAIL: ok order dry=${dry_line} apply=${apply_line} up=${up_line}"
  sed -n '1,200p' "${trace}"
  fail=1
fi

if [[ "${fail}" -ne 0 ]]; then
  echo "---- last stdout ----"
  sed -n '1,240p' "${WORK}/last-out.txt" || true
  echo "---- last trace ----"
  sed -n '1,200p' "${WORK}/last-trace.txt" || true
  exit 1
fi

echo "PASS: migrate dry-run failure aborts before apply"
