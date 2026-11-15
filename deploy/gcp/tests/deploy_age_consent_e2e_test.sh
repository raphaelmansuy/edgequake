#!/usr/bin/env bash
# Issue 537 wired through the real deploy.sh, plus the dry-run gate from issue 541.
# A version pin must not execute `migrate --drop-age`. A failed dry-run must not apply.
# Runs deploy.sh against a stub docker. Needs passwordless sudo (root check).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
DEPLOY="${ROOT}/deploy/gcp/scripts/deploy.sh"
fail=0

if grep -n 'migrate dry-run || true' "${DEPLOY}"; then
  echo "FAIL: deploy.sh still swallows migrate dry-run"
  exit 1
fi
if ! grep -q 'age_drop_policy.sh' "${DEPLOY}"; then
  echo "FAIL: deploy.sh does not source the consent policy"
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
write_env() {
  local version="$1"
  cat >"${WORK}/stack/.env" <<EOF
EDGEQUAKE_VERSION=${version}
EDGEQUAKE_POSTGRES_TAG=${version}
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
}

cat >"${STUB_BIN}/gcloud" <<'EOF'
#!/usr/bin/env bash
exit 1
EOF

cat >"${STUB_BIN}/curl" <<'EOF'
#!/usr/bin/env bash
exit 1
EOF

cat >"${STUB_BIN}/ctr" <<'EOF'
#!/usr/bin/env bash
exit 0
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
  printf '%s\n' "${STUB_AGE:-f}"
  exit 0
fi
if [[ "${joined}" == *" migrate dry-run "* ]]; then
  if [[ "${STUB_DRY_RUN:-ok}" == "ok" ]]; then
    echo "dry-run complete: no migrations applied (preview only)."
    exit 0
  fi
  echo "[stub] migrate dry-run simulated failure" >&2
  exit "${STUB_DRY_RUN_CODE:-1}"
fi
if [[ "${joined}" == *" migrate --drop-age "* ]]; then
  echo "[stub] migrate --drop-age"
  exit 0
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
chmod 0755 "${STUB_BIN}/docker" "${STUB_BIN}/gcloud" "${STUB_BIN}/curl" "${STUB_BIN}/ctr"

run_deploy() {
  local name="$1"
  local version="$2"
  local age="$3"
  local dry="$4"
  local token="${5:-}"
  local trace="${WORK}/trace-${name}"
  local log="${WORK}/deploy-${name}.log"
  local out="${WORK}/out-${name}.txt"
  write_env "${version}"
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
    STUB_AGE="${age}" \
    STUB_DRY_RUN="${dry}" \
    EDGEQUAKE_DEPLOY_DROP_AGE="${token}" \
    "${DEPLOY}" >"${out}" 2>&1 </dev/null
  local rc=$?
  set -e
  echo "${rc}" >"${WORK}/rc-${name}"
  cp "${out}" "${WORK}/last-out.txt"
  cp "${trace}" "${WORK}/last-trace.txt"
}

assert_absent() {
  local file="$1" needle="$2" label="$3"
  if grep -qF -- "${needle}" "${file}"; then
    echo "FAIL: ${label}: found unexpected '${needle}'"
    fail=1
  fi
}

assert_present() {
  local file="$1" needle="$2" label="$3"
  if ! grep -qF -- "${needle}" "${file}"; then
    echo "FAIL: ${label}: missing '${needle}'"
    fail=1
  fi
}

drop_count() {
  grep -cF -- '--drop-age' "$1" || true
}

schema_apply_count() {
  { grep -E 'api migrate( |$)' "$1" || true; } | { grep -v 'dry-run' || true; } | { grep -v -- '--drop-age' || true; } | wc -l | tr -d ' '
}

echo "== installed AGE, closed stdin, no token: no drop, schema apply runs =="
run_deploy keep 0.40.7 t ok
rc="$(cat "${WORK}/rc-keep")"
if [[ "${rc}" -ne 86 ]]; then
  echo "FAIL: keep expected exit 86, got ${rc}"
  fail=1
fi
assert_present "${WORK}/out-keep.txt" "Leaving it installed" "keep stdout"
assert_present "${WORK}/out-keep.txt" "LD-15 migrate apply" "keep stdout"
assert_absent "${WORK}/out-keep.txt" "running migrate --drop-age" "keep stdout"
assert_absent "${WORK}/out-keep.txt" "DEPLOY_OK" "keep stdout"
if [[ "$(drop_count "${WORK}/trace-keep")" -ne 0 ]]; then
  echo "FAIL: keep trace ran migrate --drop-age"
  fail=1
fi
if [[ "$(schema_apply_count "${WORK}/trace-keep")" -ne 1 ]]; then
  echo "FAIL: keep trace schema apply count $(schema_apply_count "${WORK}/trace-keep")"
  fail=1
fi

echo "== token yes and AGE installed: drop once, then schema apply =="
run_deploy drop 0.40.7 t ok yes
rc="$(cat "${WORK}/rc-drop")"
if [[ "${rc}" -ne 86 ]]; then
  echo "FAIL: drop expected exit 86, got ${rc}"
  fail=1
fi
assert_present "${WORK}/out-drop.txt" "running migrate --drop-age" "drop stdout"
assert_present "${WORK}/out-drop.txt" "LD-15 migrate apply" "drop stdout"
assert_absent "${WORK}/out-drop.txt" "FAIL: migration dry-run failed" "drop stdout"
if [[ "$(drop_count "${WORK}/trace-drop")" -ne 1 ]]; then
  echo "FAIL: drop trace --drop-age count $(drop_count "${WORK}/trace-drop")"
  fail=1
fi
drop_line="$(grep -nF -- '--drop-age' "${WORK}/trace-drop" | head -1 | cut -d: -f1 || true)"
apply_line="$( { grep -nE 'api migrate( |$)' "${WORK}/trace-drop" || true; } | { grep -v 'dry-run' || true; } | { grep -v -- '--drop-age' || true; } | head -1 | cut -d: -f1 || true)"
if [[ -z "${drop_line}" || -z "${apply_line}" || "${drop_line}" -ge "${apply_line}" ]]; then
  echo "FAIL: drop order drop=${drop_line:-none} apply=${apply_line:-none}"
  fail=1
fi

echo "== token yes but AGE absent: do not drop =="
run_deploy absent 0.40.7 f ok yes
rc="$(cat "${WORK}/rc-absent")"
if [[ "${rc}" -ne 86 ]]; then
  echo "FAIL: absent expected exit 86, got ${rc}"
  fail=1
fi
assert_present "${WORK}/out-absent.txt" "not running migrate --drop-age" "absent stdout"
if [[ "$(drop_count "${WORK}/trace-absent")" -ne 0 ]]; then
  echo "FAIL: absent trace ran migrate --drop-age"
  fail=1
fi

echo "== v0.35.4 still has age.so: token does not drop =="
run_deploy old 0.35.4 t ok yes
rc="$(cat "${WORK}/rc-old")"
if [[ "${rc}" -ne 86 ]]; then
  echo "FAIL: old expected exit 86, got ${rc}"
  fail=1
fi
if [[ "$(drop_count "${WORK}/trace-old")" -ne 0 ]]; then
  echo "FAIL: old-image trace ran migrate --drop-age"
  fail=1
fi
assert_present "${WORK}/out-old.txt" "LD-15 migrate apply" "old stdout"

echo "== closed stdin and a failed dry-run abort before schema apply =="
run_deploy dryfail 0.40.7 t fail
rc="$(cat "${WORK}/rc-dryfail")"
if [[ "${rc}" -ne 1 ]]; then
  echo "FAIL: dryfail expected exit 1, got ${rc}"
  fail=1
fi
assert_present "${WORK}/out-dryfail.txt" "FAIL: migration dry-run failed" "dryfail stdout"
assert_absent "${WORK}/out-dryfail.txt" "LD-15 migrate apply" "dryfail stdout"
assert_absent "${WORK}/out-dryfail.txt" "DEPLOY_OK" "dryfail stdout"
if [[ "$(drop_count "${WORK}/trace-dryfail")" -ne 0 ]]; then
  echo "FAIL: dryfail dropped AGE without consent"
  fail=1
fi
if [[ "$(schema_apply_count "${WORK}/trace-dryfail")" -ne 0 ]]; then
  echo "FAIL: dryfail reached schema apply"
  fail=1
fi

echo "== consented drop still refuses schema apply when dry-run fails =="
run_deploy dropdry 0.40.7 t fail yes
rc="$(cat "${WORK}/rc-dropdry")"
if [[ "${rc}" -ne 1 ]]; then
  echo "FAIL: dropdry expected exit 1, got ${rc}"
  fail=1
fi
assert_present "${WORK}/out-dropdry.txt" "running migrate --drop-age" "dropdry stdout"
assert_present "${WORK}/out-dropdry.txt" "FAIL: migration dry-run failed" "dropdry stdout"
assert_absent "${WORK}/out-dropdry.txt" "LD-15 migrate apply" "dropdry stdout"
if [[ "$(drop_count "${WORK}/trace-dropdry")" -ne 1 ]]; then
  echo "FAIL: dropdry --drop-age count $(drop_count "${WORK}/trace-dropdry")"
  fail=1
fi
if [[ "$(schema_apply_count "${WORK}/trace-dropdry")" -ne 0 ]]; then
  echo "FAIL: dropdry reached schema apply after a failed dry-run"
  fail=1
fi

if [[ "${fail}" -ne 0 ]]; then
  echo "---- last stdout ----"
  sed -n '1,240p' "${WORK}/last-out.txt" || true
  echo "---- last trace ----"
  sed -n '1,200p' "${WORK}/last-trace.txt" || true
  exit 1
fi

echo "PASS: deploy.sh drops Apache AGE only with consent, and a failed dry-run does not apply"
