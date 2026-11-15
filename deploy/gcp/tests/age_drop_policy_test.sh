#!/usr/bin/env bash
# Consent policy for deploy.sh migration 171 (issue 537).
# A version pin must not run `migrate --drop-age`. Closed stdin must not either.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
POLICY="${ROOT}/scripts/age_drop_policy.sh"
DEPLOY="${ROOT}/scripts/deploy.sh"

# shellcheck source=deploy/gcp/scripts/age_drop_policy.sh
source "${POLICY}"

fail=0
check() {
  local name="$1" got="$2" want="$3"
  if [[ "${got}" == "${want}" ]]; then
    printf 'OK   %s\n' "${name}"
  else
    printf 'FAIL %s: got %q want %q\n' "${name}" "${got}" "${want}" >&2
    fail=1
  fi
}

run_step() {
  local version="$1"
  age_deploy_step "${version}"
}

run_decision() {
  local version="$1" age="$2"
  # Command substitution is how deploy.sh reads the decision. Stdout is a
  # pipe; stdin is whatever the caller attached.
  age_drop_decision "${version}" "${age}"
}

unset EDGEQUAKE_DEPLOY_DROP_AGE EDGEQUAKE_MIGRATE_DROP_AGE || true

# Issue 537: version >= 0.35.5 and AGE installed, stdin closed. No drop.
check "above step (0.35.5, no token)" "$(run_step 0.35.5)" "decide"
check "above decision (0.35.5, age=t, stdin closed)" \
  "$(run_decision 0.35.5 t </dev/null)" "keep"
check "above pipe of yes is not consent" \
  "$(printf 'yes\n' | run_decision 0.35.5 t)" "keep"

# Below the library cutoff: the old script did not drop. Still must not.
check "below step (0.35.4)" "$(run_step 0.35.4)" "none"
check "below decision (0.35.4, age=t)" "$(run_decision 0.35.4 t </dev/null)" "keep"
EDGEQUAKE_DEPLOY_DROP_AGE=yes
check "below + token still does not pre-drop (image has age.so)" \
  "$(run_step 0.35.4)" "none"
check "below + token does not decide drop" \
  "$(run_decision 0.35.4 t </dev/null)" "keep"
unset EDGEQUAKE_DEPLOY_DROP_AGE

# Extension absent: nothing to destroy.
check "noage step" "$(run_step 0.35.5)" "decide"
check "noage decision" "$(run_decision 0.35.5 f </dev/null)" "keep"
EDGEQUAKE_DEPLOY_DROP_AGE=yes
check "noage + token does not decide drop" "$(run_decision 0.35.5 f </dev/null)" "keep"
check "token + no library image probes before start" "$(run_step 0.35.5)" "drop-first"
unset EDGEQUAKE_DEPLOY_DROP_AGE

# Explicit token. Closed stdin is fine: the variable is the consent.
for token in yes YES Yes 1 true TRUE on ' yes '; do
  EDGEQUAKE_DEPLOY_DROP_AGE="${token}"
  check "token ${token@Q} step" "$(run_step 0.40.7)" "drop-first"
  check "token ${token@Q} decision" "$(run_decision 0.40.7 t </dev/null)" "drop"
done
unset EDGEQUAKE_DEPLOY_DROP_AGE

for token in no false 0 y '' 'yes please' n; do
  EDGEQUAKE_DEPLOY_DROP_AGE="${token}"
  check "rejected token ${token@Q}" "$(run_decision 0.35.5 t </dev/null)" "keep"
  check "rejected token ${token@Q} step" "$(run_step 0.35.5)" "decide"
done
unset EDGEQUAKE_DEPLOY_DROP_AGE

# The migrate CLI's variable must not consent for the deploy script.
# shellcheck disable=SC2034  # tripwire: the policy must not read this
EDGEQUAKE_MIGRATE_DROP_AGE=yes
check "migrate env is not deploy consent (step)" "$(run_step 0.35.5)" "decide"
check "migrate env is not deploy consent (decision)" \
  "$(run_decision 0.35.5 t </dev/null)" "keep"
unset EDGEQUAKE_MIGRATE_DROP_AGE

# Later product pins still lack the library. The pin alone is not consent.
for version in 0.35.5 0.35.6 0.36.0 0.40.7; do
  check "lacks library ${version} without token" "$(run_step "${version}")" "decide"
  check "keep ${version} without token" "$(run_decision "${version}" t </dev/null)" "keep"
done

# Unparsable tags fail closed: do not treat them as a no-library image.
for version in '' latest v0.35.5 0.35.5-pg16; do
  EDGEQUAKE_DEPLOY_DROP_AGE=yes
  check "unparsable ${version@Q} + token" "$(run_step "${version}")" "none"
done
unset EDGEQUAKE_DEPLOY_DROP_AGE

# Shell version must not override the argument. 0.35.4 has the library.
# shellcheck disable=SC2034  # tripwire: the policy must not read this
EDGEQUAKE_VERSION=0.40.7
EDGEQUAKE_DEPLOY_DROP_AGE=yes
check "argument wins over EDGEQUAKE_VERSION" "$(run_step 0.35.4)" "none"
unset EDGEQUAKE_VERSION EDGEQUAKE_DEPLOY_DROP_AGE

python3 - "${POLICY}" "${DEPLOY}" <<'PY'
import os, pty, select, subprocess, sys, time

policy, deploy = sys.argv[1], sys.argv[2]
text = open(deploy, encoding="utf-8").read()
fn = text.split("drop_age_using_library_image()", 1)[1].split("\nlog_age_left_installed()", 1)[0]
if "api migrate --drop-age" not in fn:
    raise SystemExit("drop command is not inside drop_age_using_library_image")
if text.replace(fn, "", 1).count("api migrate --drop-age") != 0:
    raise SystemExit("api migrate --drop-age appears outside the consent function")
if text.count("api migrate --drop-age") != 1:
    raise SystemExit("expected exactly one api migrate --drop-age invocation")
if "postgres_has_no_age_library" in text:
    raise SystemExit("version check must not be its own drop gate")
if "age_deploy_step" not in text or "age_drop_decision" not in text:
    raise SystemExit("deploy.sh does not call the consent policy")
if "drop_age_using_library_image env" not in text:
    raise SystemExit("missing env-consent call")
if "drop_age_using_library_image terminal" not in text:
    raise SystemExit("missing terminal-consent call")
# The schema migrate that always runs must not carry the flag.
apply = [ln for ln in text.splitlines() if "LD-15 migrate apply" in ln]
if len(apply) != 1 or "--drop-age" not in apply[0] or "does not pass --drop-age" not in apply[0]:
    raise SystemExit(f"schema migrate log drifted: {apply}")
schema = [ln for ln in text.splitlines() if ln.strip().endswith("api migrate")]
if schema != ['"${COMPOSE[@]}" run --rm --no-deps -T api migrate']:
    raise SystemExit(f"schema migrate command drifted: {schema}")
print("OK   deploy.sh wires --drop-age only after consent")

def decide(answer: bytes, *, env_yes: bool, wrapper: bool) -> tuple[str, bytes]:
    master, slave = pty.openpty()
    env = os.environ.copy()
    env.pop("EDGEQUAKE_MIGRATE_DROP_AGE", None)
    if env_yes:
        env["EDGEQUAKE_DEPLOY_DROP_AGE"] = "yes"
    else:
        env.pop("EDGEQUAKE_DEPLOY_DROP_AGE", None)
    script = r'''
set -euo pipefail
source "$1"
if [[ "$4" == wrapper ]]; then
  decision="$(age_drop_decision "$2" "$3")"
  printf '%s\n' "$decision"
else
  age_drop_decision "$2" "$3"
fi
'''
    proc = subprocess.Popen(
        ["bash", "-c", script, "bash", policy, "0.35.5", "t", "wrapper" if wrapper else "direct"],
        stdin=slave,
        stdout=slave,
        stderr=slave,
        env=env,
        close_fds=True,
    )
    os.close(slave)
    buf = b""
    sent = False
    deadline = time.time() + 5
    while time.time() < deadline:
        if proc.poll() is not None:
            while select.select([master], [], [], 0)[0]:
                try:
                    chunk = os.read(master, 4096)
                except OSError:
                    chunk = b""
                if not chunk:
                    break
                buf += chunk
            break
        readable, _, _ = select.select([master], [], [], 0.2)
        if not readable:
            continue
        try:
            chunk = os.read(master, 4096)
        except OSError:
            break
        if not chunk:
            break
        buf += chunk
        if answer and (not sent) and b"Drop Apache AGE?" in buf:
            os.write(master, answer)
            sent = True
    else:
        proc.kill()
        os.close(master)
        raise SystemExit(f"timed out waiting for decision: {buf!r}")
    rc = proc.wait(timeout=2)
    os.close(master)
    if rc != 0:
        raise SystemExit(f"decision helper exit {rc}: {buf!r}")
    lines = [ln.strip() for ln in buf.decode("utf-8", "replace").splitlines()]
    choices = [ln for ln in lines if ln in ("drop", "keep")]
    if not choices:
        raise SystemExit(f"no decision in {buf!r}")
    return choices[-1], buf

# deploy.sh captures the decision. A yes on that path must still drop,
# and a pipe-like missing answer must keep. Env consent must not prompt.
got, buf = decide(b"y\n", env_yes=False, wrapper=True)
if got != "drop" or b"Drop Apache AGE?" not in buf:
    raise SystemExit(f"wrapper y should prompt and drop: {got} {buf!r}")
print("OK   terminal y inside command substitution drops")

got, buf = decide(b"yes\n", env_yes=False, wrapper=True)
if got != "drop":
    raise SystemExit(f"wrapper yes should drop: {got} {buf!r}")
print("OK   terminal yes drops")

got, _ = decide(b"n\n", env_yes=False, wrapper=True)
if got != "keep":
    raise SystemExit(f"terminal n should keep: {got}")
print("OK   terminal n keeps")

got, _ = decide(b"\n", env_yes=False, wrapper=True)
if got != "keep":
    raise SystemExit("terminal Enter should keep")
print("OK   terminal Enter keeps")

got, buf = decide(b"", env_yes=True, wrapper=True)
if got != "drop" or b"Drop Apache AGE?" in buf:
    raise SystemExit(f"env token must drop without a prompt: {got} {buf!r}")
print("OK   env token skips the prompt inside command substitution")
PY

if [[ "${fail}" -ne 0 ]]; then
  exit 1
fi
echo "age_drop_policy_test: all checks passed"
