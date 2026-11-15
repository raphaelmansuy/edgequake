#!/usr/bin/env bash
# Consent for dropping Apache AGE during GCP deploy.
#
# Migration 171 drops the extension and its graphs. That cannot be undone.
# A version pin is a property of the image, not permission to destroy data.
# This file only decides. It does not run docker or migrate.
#
# shellcheck shell=bash

# True (exit 0) when this image tag does not ship age.so.
# v0.35.4 is the last published tag that includes the library.
# Unparsable tags are not treated as a no-library image.
version_lacks_age_library() {
  python3 - "$@" <<'PY'
import sys
raw = sys.argv[1].strip() if len(sys.argv) > 1 else ""
parts = []
for piece in raw.split("."):
    if piece == "" or not piece.isdigit():
        raise SystemExit(1)
    parts.append(int(piece))
if not parts:
    raise SystemExit(1)
raise SystemExit(0 if tuple(parts) >= (0, 35, 5) else 1)
PY
}

# Canonical token is yes. 1, true, and on match the migrate CLI's consent set.
# "y" alone is not a token: a one-letter env value is too easy to set by mistake.
age_drop_env_consented() {
  local raw
  raw="$(printf '%s' "${EDGEQUAKE_DEPLOY_DROP_AGE:-}" | tr '[:upper:]' '[:lower:]' | tr -d '[:space:]')"
  case "${raw}" in
    yes|1|true|on) return 0 ;;
    *) return 1 ;;
  esac
}

# A real terminal may answer the question. A closed stdin, a pipe, or a
# one-sided tty is not an operator sitting at the prompt. Default is no.
#
# deploy.sh reads the decision through a command substitution, so this
# function's stdout is a pipe. Stdin and stderr are still the operator's
# terminal, and the prompt is written to stderr. Check those, not stdout.
age_drop_terminal_consented() {
  local answer
  if [[ ! -t 0 || ! -t 2 ]]; then
    return 1
  fi
  echo "Apache AGE is installed. Images from v0.35.5 do not ship age.so." >&2
  echo "Dropping it runs migration 171 on the v0.35.4 image and cannot be undone." >&2
  echo "Take a backup first. Press Enter to leave Apache AGE installed." >&2
  answer=""
  if ! read -r -p "Drop Apache AGE? [y/N] " answer; then
    echo "Apache AGE left installed (no answer)." >&2
    return 1
  fi
  case "$(printf '%s' "${answer}" | tr '[:upper:]' '[:lower:]' | tr -d '[:space:]')" in
    y|yes) return 0 ;;
    *) return 1 ;;
  esac
}

# What to do before Postgres is started.
# drop-first: consent is already in the environment, and this image has no age.so.
#   Open the volume on v0.35.4 (the last image with the library) and drop only
#   when the extension is actually installed.
# decide: this image has no age.so and nobody has consented yet. Start the
#   target image, then ask age_drop_decision once pg_extension is readable.
# none: do not drop. The image still has the library, or the tag is unparsable.
age_deploy_step() {
  local version="$1"
  if ! version_lacks_age_library "${version}"; then
    printf 'none\n'
    return 0
  fi
  if age_drop_env_consented; then
    printf 'drop-first\n'
    return 0
  fi
  printf 'decide\n'
}

# drop or keep, after the caller has read pg_extension.
# age_installed is the psql boolean: t or f.
# EDGEQUAKE_MIGRATE_DROP_AGE is the migrate CLI's own switch. It is not read
# here. Deploy consent is only EDGEQUAKE_DEPLOY_DROP_AGE or a terminal yes.
age_drop_decision() {
  local version="$1"
  local age_installed="$2"
  if ! version_lacks_age_library "${version}"; then
    printf 'keep\n'
    return 0
  fi
  if [[ "${age_installed}" != "t" ]]; then
    printf 'keep\n'
    return 0
  fi
  if age_drop_env_consented || age_drop_terminal_consented; then
    printf 'drop\n'
    return 0
  fi
  printf 'keep\n'
}

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
  echo "source age_drop_policy.sh; it does not deploy" >&2
  exit 1
fi
