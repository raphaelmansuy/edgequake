#!/usr/bin/env bash
# Issue #537 regression: deploy.sh must never run `migrate --drop-age` on its
# own. A version check is not consent, and neither is a closed stdin.
#
# Drives the real deploy.sh against a fake `docker`/`python3` PATH, so the
# script's own control flow is what gets measured.
#
# Run: ./deploy/gcp/scripts/tests/age_drop_consent.sh
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
DEPLOY="${SCRIPT_DIR}/deploy.sh"
WORK="$(mktemp -d)"
trap 'rm -rf "${WORK}"' EXIT

fail=0
pass() { echo "PASS: $*"; }
die() { echo "FAIL: $*"; fail=1; }

# The script demands root. Re-exec the same code path under a user namespace
# when the caller is not root, so the test is portable.
if [[ "${EUID}" -ne 0 ]]; then
  if unshare --map-root-user --user true >/dev/null 2>&1; then
    exec unshare --map-root-user --user "${BASH:-bash}" "$0" "$@"
  fi
  echo "SKIP: needs root (or user namespaces) to run deploy.sh"
  exit 0
fi

# --- fake host layout -------------------------------------------------------
make_host() {
  local root="$1" age_installed="$2" version="$3"
  rm -rf "${root}"
  mkdir -p "${root}/compose" "${root}/scripts" "${root}/bin" "${root}/log"

  # deploy.sh copies one of these onto Caddyfile.runtime before the AGE gate.
  printf 'runtime\n' >"${root}/compose/Caddyfile"
  printf 'runtime\n' >"${root}/compose/Caddyfile.hostname"

  cat >"${root}/.env" <<EOF
EDGEQUAKE_VERSION=${version}
EDGEQUAKE_POSTGRES_TAG=${version}
POSTGRES_PASSWORD=fake
EDGEQUAKE_TLS_HOST=127.0.0.1
EDGEQUAKE_ORIGIN=https://example.test
ELITIZON_DATA_MOUNT=${root}/data
EOF

  # An initialized data directory: the only shape that can still carry AGE.
  mkdir -p "${root}/data/pgdata"
  printf '16\n' >"${root}/data/pgdata/PG_VERSION"

  # Scripts deploy.sh sources. No-ops keep the fake focused on the AGE gate.
  for helper in render-env.sh render-originals.sh load-metadata.sh generate-tls.sh; do
    printf '#!/usr/bin/env bash\nexit 0\n' >"${root}/scripts/${helper}"
    chmod +x "${root}/scripts/${helper}"
  done

  : >"${root}/docker.log"
  cat >"${root}/bin/docker" <<EOF
#!/usr/bin/env bash
printf 'TAG=%s %s\n' "\${EDGEQUAKE_POSTGRES_TAG:-}" "\$*" >>"${root}/docker.log"
case "\$*" in
  *"psql"*"pg_extension"*) printf '%s\n' "${age_installed}" ;;
esac
exit 0
EOF
  chmod +x "${root}/bin/docker"

  # The deploy's own health/HTTPS gates are out of scope here and wait on
  # real sockets. Stub them so the run is bounded and stops at the AGE gate.
  printf '#!/usr/bin/env bash\nexit 1\n' >"${root}/bin/curl"
  chmod +x "${root}/bin/curl"
  printf '#!/usr/bin/env bash\nexit 0\n' >"${root}/bin/sleep"
  chmod +x "${root}/bin/sleep"
}

run_deploy() {
  local root="$1" consent="$2" stdin_closed="$3"
  (
    export PATH="${root}/bin:${PATH}"
    export STACK_ROOT="${root}"
    export STACK_LOG="${root}/log/deploy.log"
    if [[ -n "${consent}" ]]; then
      export EDGEQUAKE_DEPLOY_AGE_DROP="${consent}"
    fi
    if [[ "${stdin_closed}" == "closed" ]]; then
      "${DEPLOY}" </dev/null >"${root}/stdout.txt" 2>&1
    else
      "${DEPLOY}" >"${root}/stdout.txt" 2>&1
    fi
  )
  echo $?
}

dropped_age() {
  grep -q -- "--drop-age" "${1}/docker.log" 2>/dev/null
}

# Reaching the LD-15 migrate means the AGE gate let the deploy through.
reached_migrate() {
  grep -q "LD-15 migrate apply" "${1}/stdout.txt" 2>/dev/null
}

refused_age() {
  grep -q "REFUSING to drop Apache AGE" "${1}/stdout.txt" 2>/dev/null
}

# The 0.35.4 probe must happen before the requested image opens the volume.
probed_on_cutover_image() {
  grep -q "0\.35\.4" "${1}/docker.log" 2>/dev/null
}

# --- 1. version above the AGE cut, volume still has AGE, no consent ---------
root="${WORK}/nocs"
make_host "${root}" t 0.35.5
rc="$(run_deploy "${root}" "" closed)"
refused_age "${root}" || die "no consent must say why it stopped"
[[ "${rc}" -ne 0 ]] || die "no consent must stop the deploy (rc=${rc})"
dropped_age "${root}" && die "no consent must never run migrate --drop-age"
reached_migrate "${root}" && die "no consent must stop before any migrate"
probed_on_cutover_image "${root}" || die "the probe must run on the 0.35.4 image"
grep -q "EDGEQUAKE_DEPLOY_AGE_DROP=drop" "${root}/stdout.txt" \
  || die "no consent must print the consent command"
pass "no consent, AGE installed: refuses, does not drop"

# --- 2. version below the AGE cut: the question does not apply --------------
root="${WORK}/old"
make_host "${root}" t 0.35.4
run_deploy "${root}" "" closed >/dev/null
refused_age "${root}" && die "0.35.4 must not ask the AGE question"
reached_migrate "${root}" || die "0.35.4 deploy must reach migrate"
dropped_age "${root}" && die "0.35.4 must not run migrate --drop-age"
pass "below the cut: proceeds, no drop"

# --- 3. version above the cut, fresh volume (no AGE extension) --------------
root="${WORK}/noage"
make_host "${root}" f 0.35.5
run_deploy "${root}" "" closed >/dev/null
refused_age "${root}" && die "a volume without AGE must not be refused"
reached_migrate "${root}" || die "fresh volume deploy must reach migrate"
dropped_age "${root}" && die "fresh volume must not run migrate --drop-age"
pass "fresh volume: proceeds, no drop"

# --- 3b. greenfield (no data directory): no probe, no pull ------------------
root="${WORK}/greenfield"
make_host "${root}" t 0.35.5
rm -rf "${root}/data"
run_deploy "${root}" "" closed >/dev/null
refused_age "${root}" && die "greenfield must not ask the AGE question"
reached_migrate "${root}" || die "greenfield deploy must reach migrate"
grep -q "0\.35\.4" "${root}/docker.log" \
  && die "greenfield must not pull the 0.35.4 probe image"
pass "greenfield: no probe, no drop"

# --- 4. explicit consent on the command line --------------------------------
root="${WORK}/consent"
TARGET_VERSION="0.35.5"
make_host "${root}" t "${TARGET_VERSION}"
run_deploy "${root}" drop closed >/dev/null
dropped_age "${root}" || die "consent must run migrate --drop-age"
reached_migrate "${root}" || die "consented deploy must continue to migrate"
# The drop must run on the cutover image, and must happen before the requested
# image touches the volume (which cannot open an AGE data directory).
grep -- "--drop-age" "${root}/docker.log" | grep -q "TAG=0.35.4" \
  || die "the drop must run on the 0.35.4 image"
drop_line="$(grep -n -- "--drop-age" "${root}/docker.log" | cut -d: -f1)"
# First command that would actually start a container with the requested pin.
# A pull is inert; `up`/`run` are what open the data directory.
target_line="$(grep -n "TAG=${TARGET_VERSION}.*\(up -d postgres\|run --rm\)" \
  "${root}/docker.log" | head -1 | cut -d: -f1)"
if [[ -n "${target_line}" ]]; then
  [[ "${drop_line}" -lt "${target_line}" ]] \
    || die "the drop must run before ${TARGET_VERSION} touches the volume"
fi
pass "explicit consent: drops AGE on 0.35.4 before the new image"

# --- 5. a value inside .env is not consent ----------------------------------
root="${WORK}/envfile"
make_host "${root}" t 0.35.5
printf '%s\n' "EDGEQUAKE_DEPLOY_AGE_DROP=drop" >>"${root}/.env"
run_deploy "${root}" "" closed >/dev/null
dropped_age "${root}" && die ".env must never run migrate --drop-age"
refused_age "${root}" || die ".env value must still be refused"
pass ".env value is not consent"

# --- 6. consent words that are not an answer --------------------------------
for bad in no keep "" "   "; do
  root="${WORK}/bad"
  make_host "${root}" t 0.35.5
  run_deploy "${root}" "${bad}" closed >/dev/null
  dropped_age "${root}" && die "consent='${bad}' must never run migrate --drop-age"
  refused_age "${root}" || die "consent='${bad}' must be refused"
done
pass "no, keep, and blank are not consent"

# --- 7. source-level guard: the flag is never unconditional -----------------
count="$(grep -c -- "--drop-age" "${DEPLOY}")"
[[ "${count}" -eq 1 ]] \
  || die "deploy.sh must contain exactly one --drop-age invocation, found ${count}"
line="$(grep -n -- "--drop-age" "${DEPLOY}" | cut -d: -f1)"
start=$((line > 25 ? line - 25 : 1))
sed -n "${start},${line}p" "${DEPLOY}" | grep -q 'consented\|age_must_be_dropped' \
  || die "the --drop-age call must sit behind the consent gate"
pass "single --drop-age call, behind the gate"

if [[ "${fail}" -eq 0 ]]; then
  echo "PASS: issue #537 deploy consent gate"
fi
exit "${fail}"
