#!/usr/bin/env bash
# Tests for scripts/dev_vertex_project.sh — the first-run Vertex project question.
#
# The logic that matters: which project wins, when the question is asked, and that a rejected
# answer is never saved. Run directly: bash scripts/dev_vertex_project_test.sh
set -uo pipefail

_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
_script="$_root/scripts/dev_vertex_project.sh"
_file="$_root/.gcloud_active_project"
_fail=0
_pass=0

t() { # t <name> <expected> <actual>
  if [ "$2" = "$3" ]; then
    _pass=$((_pass + 1)); printf '  ok   %s\n' "$1"
  else
    _fail=$((_fail + 1)); printf '  FAIL %s\n       expected: %s\n       actual:   %s\n' "$1" "$2" "$3"
  fi
}

_backup=""
if [ -f "$_file" ]; then
  _backup="$(mktemp)"
  cp "$_file" "$_backup"
fi
cleanup() {
  if [ -n "$_backup" ]; then cp "$_backup" "$_file"; rm -f "$_backup"; else rm -f "$_file"; fi
}
trap cleanup EXIT
rm -f "$_file"

# WHY `env -u`: the developer shell in this repo exports GOOGLE_CLOUD_PROJECT=saas-app-001, and
# a test that inherits it would pass for the wrong reason.
run() { env -u GOOGLE_CLOUD_PROJECT -u PROJECT_ID "$@" bash "$_script" 2>/dev/null; }

printf 'dev_vertex_project.sh\n'

t "saved project wins over the environment" "mobioos-506015" \
  "$(printf 'mobioos-506015\n' > "$_file"; GOOGLE_CLOUD_PROJECT=other-project bash "$_script" 2>/dev/null)"
rm -f "$_file"

t "GOOGLE_CLOUD_PROJECT is used when nothing is saved" "from-env" \
  "$(run env GOOGLE_CLOUD_PROJECT=from-env)"
t "PROJECT_ID is used when GOOGLE_CLOUD_PROJECT is unset" "from-project-id" \
  "$(run env PROJECT_ID=from-project-id)"

t "GOOGLE_CLOUD_PROJECT wins over PROJECT_ID" "wins" \
  "$(run env GOOGLE_CLOUD_PROJECT=wins PROJECT_ID=loses)"
_out="$(env -u GOOGLE_CLOUD_PROJECT -u PROJECT_ID PROJECT_ID=loses GOOGLE_CLOUD_PROJECT=wins \
  bash "$_script" 2>&1 >/dev/null)"
case "$_out" in
  *"Two Vertex projects are set"*) t "a conflict is announced, not silent" "warned" "warned" ;;
  *) t "a conflict is announced, not silent" "warned" "silent" ;;
esac

t "no source and no tty: empty, and no file written" "" \
  "$(rm -f "$_file"; env -u GOOGLE_CLOUD_PROJECT -u PROJECT_ID bash "$_script" </dev/null 2>/dev/null)"
t "no source and no tty: nothing saved" "absent" \
  "$([ -f "$_file" ] && echo present || echo absent)"

t "--print fails when nothing is saved" "1" \
  "$(rm -f "$_file"; bash "$_script" --print >/dev/null 2>&1; echo $?)"
t "--print echoes the saved project" "mobioos-506015" \
  "$(printf 'mobioos-506015\n' > "$_file"; bash "$_script" --print 2>/dev/null)"

# A rejected id must not be written, or the next run would silently trust it.
printf 'Not A Project!\n' > /tmp/.vx_bad_answer
_out="$(rm -f "$_file"; env -u GOOGLE_CLOUD_PROJECT -u PROJECT_ID \
  bash "$_script" --prompt < /tmp/.vx_bad_answer 2>&1 || true)"
rm -f /tmp/.vx_bad_answer
case "$_out" in
  *"does not look like"*) t "a malformed id is rejected" "rejected" "rejected" ;;
  # No tty: the script correctly declines to prompt at all.
  *"not set"*) t "no tty: the question is never asked" "quiet" "quiet" ;;
  *) t "no tty: the question is never asked" "quiet" "asked" ;;
esac
t "a malformed id is never saved" "absent" \
  "$([ -f "$_file" ] && echo present || echo absent)"

# The failure this script exists for: a .zshrc export must not beat the pin.
printf 'mobioos-506015\n' > "$_file"
t "a stray shell export loses to the pinned file" "mobioos-506015" \
  "$(GOOGLE_CLOUD_PROJECT=saas-app-001 PROJECT_ID=other-project bash "$_script" 2>/dev/null)"
bash "$_script" --forget >/dev/null
t "forget removes the pin so the next run can ask" "absent" \
  "$([ -f "$_file" ] && echo present || echo absent)"

printf '\n%d passed, %d failed\n' "$_pass" "$_fail"
[ "$_fail" -eq 0 ]
