#!/usr/bin/env bash
# SPEC-041 — Simulate upgrade paths for M078/M079 repair (all edge cases).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../../.." && pwd)"
EVIDENCE="${SCRIPT_DIR}/evidence/upgrade_paths.log"
M078_BROKEN="d22cc6d8416c6a8ccf28542c583724f139fcf70926edef4faec490b9daac665a7e526b9c46bdc758f4c05d0d0c1d62d2"
M078_FIXED="a043177271c82c65a7509855f1d64c02c46235343126a9bbb96c359f4c25aa35427c79bb50051d499b431d869eb8e930"

: "${DB_HOST:=localhost}"
: "${DB_PORT:=5437}"
: "${DB_USER:=edgequake}"
: "${DB_PASS:=edgequake_secret}"

mkdir -p "${SCRIPT_DIR}/evidence"
exec > >(tee "${EVIDENCE}") 2>&1

echo "== SPEC-041 upgrade path simulation $(date -u +%Y-%m-%dT%H:%M:%SZ) =="
echo "Host: ${DB_HOST}:${DB_PORT}"

export PGPASSWORD="${DB_PASS}"

psql_base() {
  psql -h "${DB_HOST}" -p "${DB_PORT}" -U "${DB_USER}" "$@"
}

db_url() {
  echo "postgresql://${DB_USER}:${DB_PASS}@${DB_HOST}:${DB_PORT}/$1"
}

drop_db() {
  local db="$1"
  psql_base -d postgres -v ON_ERROR_STOP=1 -c "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '${db}' AND pid <> pg_backend_pid();" >/dev/null 2>&1 || true
  psql_base -d postgres -v ON_ERROR_STOP=1 -c "DROP DATABASE IF EXISTS ${db};"
  psql_base -d postgres -v ON_ERROR_STOP=1 -c "CREATE DATABASE ${db};"
  psql_base -d "${db}" -v ON_ERROR_STOP=1 -c "CREATE EXTENSION IF NOT EXISTS vector;"
}

run_migrations() {
  sqlx migrate run --source "${REPO_ROOT}/edgequake/migrations" --database-url "$1"
}

PASS=0
FAIL=0
check() {
  local name="$1" cond="$2"
  if [[ "${cond}" == "ok" ]]; then echo "  PASS: ${name}"; ((PASS++)) || true
  else echo "  FAIL: ${name} — ${cond}"; ((FAIL++)) || true; fi
}

# PATH A: Fresh install applies M078 + M079
echo ""
echo "--- PATH A: Fresh DB (any version ≤77 → v0.13.3) ---"
DB_A="spec041_path_a"
drop_db "${DB_A}"
URL_A=$(db_url "${DB_A}")
run_migrations "${URL_A}" >/dev/null
check "PATH A v78" "$([[ $(psql_base -d "${DB_A}" -t -A -c "SELECT COUNT(*) FROM _sqlx_migrations WHERE version=78 AND success") == "1" ]] && echo ok || echo fail)"
check "PATH A v79" "$([[ $(psql_base -d "${DB_A}" -t -A -c "SELECT COUNT(*) FROM _sqlx_migrations WHERE version=79 AND success") == "1" ]] && echo ok || echo fail)"

# PATH B: v0.13.2 skip-path — v78 old checksum, repair, M079 pending
echo ""
echo "--- PATH B: v0.13.2 skip-path checksum repair ---"
DB_B="spec041_path_b"
drop_db "${DB_B}"
URL_B=$(db_url "${DB_B}")
run_migrations "${URL_B}" >/dev/null
psql_base -d "${DB_B}" -c "DELETE FROM _sqlx_migrations WHERE version >= 79;"
psql_base -d "${DB_B}" -c "UPDATE _sqlx_migrations SET checksum = decode('${M078_BROKEN}', 'hex') WHERE version = 78;"
psql_base -d "${DB_B}" -c "UPDATE _sqlx_migrations SET checksum = decode('${M078_FIXED}', 'hex') WHERE version = 78 AND encode(checksum,'hex') = '${M078_BROKEN}';"
CS=$(psql_base -d "${DB_B}" -t -A -c "SELECT encode(checksum,'hex') FROM _sqlx_migrations WHERE version=78;")
check "PATH B checksum L1 repair" "$([[ "${CS}" == "${M078_FIXED}" ]] && echo ok || echo "cs=${CS}")"
run_migrations "${URL_B}" >/dev/null
check "PATH B v79 after repair" "$([[ $(psql_base -d "${DB_B}" -t -A -c "SELECT COUNT(*) FROM _sqlx_migrations WHERE version=79 AND success") == "1" ]] && echo ok || echo fail)"

# PATH C: Blocked at v78 — simulate DB at v77, pending 78+79
echo ""
echo "--- PATH C: Blocked at v78 (pending migrations) ---"
DB_C="spec041_path_c"
drop_db "${DB_C}"
URL_C=$(db_url "${DB_C}")
run_migrations "${URL_C}" >/dev/null
psql_base -d "${DB_C}" -c "DELETE FROM _sqlx_migrations WHERE version >= 78;"
check "PATH C v78 pending" "$([[ $(psql_base -d "${DB_C}" -t -A -c "SELECT COUNT(*) FROM _sqlx_migrations WHERE version=78") == "0" ]] && echo ok || echo fail)"
run_migrations "${URL_C}" >/dev/null
check "PATH C v78 applied" "$([[ $(psql_base -d "${DB_C}" -t -A -c "SELECT COUNT(*) FROM _sqlx_migrations WHERE version=78 AND success") == "1" ]] && echo ok || echo fail)"
check "PATH C v79 applied" "$([[ $(psql_base -d "${DB_C}" -t -A -c "SELECT COUNT(*) FROM _sqlx_migrations WHERE version=79 AND success") == "1" ]] && echo ok || echo fail)"

# PATH D: No invalid operator in sqlx migration SQL bodies
echo ""
echo "--- PATH D: Static operator gate ---"
if grep -rFe '->>>' "${REPO_ROOT}/edgequake/migrations/"*.sql 2>/dev/null | grep -v '^[^:]*:--'; then
  check "PATH D no ->>> in migration SQL" "found"
else
  check "PATH D no ->>> in migration SQL" "ok"
fi

echo ""
echo "=== Upgrade paths: PASS=${PASS} FAIL=${FAIL} ==="
[[ "${FAIL}" -eq 0 ]]
