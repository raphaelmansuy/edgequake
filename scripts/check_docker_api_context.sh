#!/usr/bin/env bash
# Fail closed if the API Docker build context would break `cargo` manifest parse.
# Root cause of v0.16.0 CD flake: .dockerignore excluded benches/ while Cargo.toml
# declares [[bench]] (and [[example]]) — cargo refuses to parse without those files.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
EQ="$ROOT/edgequake"
DOCKERIGNORE="$EQ/.dockerignore"
CARGO_TOML="$EQ/Cargo.toml"

fail() {
  echo "ERROR: $*" >&2
  exit 1
}

[[ -f "$CARGO_TOML" ]] || fail "missing $CARGO_TOML"
[[ -f "$DOCKERIGNORE" ]] || fail "missing $DOCKERIGNORE"

# Paths that must exist on disk for cargo to parse the workspace root manifest.
REQUIRED_PATHS=(
  benches/chunking_bench.rs
  benches/storage_bench.rs
  benches/graph_performance.rs
  benches/graphrag_bench.rs
  examples/basic_rag.rs
  examples/streaming_query.rs
  examples/graph_exploration.rs
  examples/multi_tenant.rs
)

for rel in "${REQUIRED_PATHS[@]}"; do
  [[ -f "$EQ/$rel" ]] || fail "missing required path for Cargo.toml: $rel"
done

# .dockerignore must NOT exclude benches/ or examples/ (Dockerfile COPYs them).
if grep -E '^(benches/|examples/)$' "$DOCKERIGNORE" >/dev/null; then
  fail ".dockerignore excludes benches/ or examples/ — Docker cargo build will fail to parse Cargo.toml"
fi

# Dockerfile must COPY both directories.
# WHY: Docker build context is repo root (.) so paths are prefixed with edgequake/
DOCKERFILE="$EQ/docker/Dockerfile"
[[ -f "$DOCKERFILE" ]] || fail "missing $DOCKERFILE"
# Check for either "COPY benches/" or "COPY edgequake/benches/" (accept both patterns)
grep -q 'COPY.*benches/ benches/' "$DOCKERFILE" || fail "Dockerfile missing: COPY benches/ (with or without edgequake/ prefix)"
grep -q 'COPY.*examples/ examples/' "$DOCKERFILE" || fail "Dockerfile missing: COPY examples/ (with or without edgequake/ prefix)"

# Runtime must stay distroless: no curl/wget healthcheck client (Trivy HIGH).
if grep -E 'apt-get install.*\b(curl|wget)\b' "$DOCKERFILE" >/dev/null; then
  fail "Dockerfile must not apt-get install curl or wget (use edgequake healthcheck)"
fi
if grep -E 'HEALTHCHECK.*\b(curl|wget)\b' "$DOCKERFILE" >/dev/null; then
  fail "Dockerfile HEALTHCHECK must not use curl or wget (use edgequake healthcheck)"
fi
grep -q 'edgequake", "healthcheck"' "$DOCKERFILE" \
  || grep -q "edgequake', 'healthcheck'" "$DOCKERFILE" \
  || fail "Dockerfile HEALTHCHECK must invoke edgequake healthcheck"

# edgeparse-ocr: Tesseract CLI + tessdata must be packaged into the distroless image.
grep -q 'tesseract-ocr-eng' "$DOCKERFILE" \
  || fail "Dockerfile must install tesseract-ocr-eng (edgeparse-ocr)"
grep -q 'TESSDATA_PREFIX=' "$DOCKERFILE" \
  || fail "Dockerfile must set TESSDATA_PREFIX for tessdata models"
grep -q '/usr/local/bin/tesseract' "$DOCKERFILE" \
  || fail "Dockerfile must COPY tesseract into /usr/local/bin/tesseract"

# Parse-only check (no compile) — same failure mode as the Docker builder stage.
(cd "$EQ" && cargo metadata --format-version 1 --no-deps >/dev/null)

# Every `include!` a build script performs must resolve inside the image context.
#
# WHY: `crates/edgequake-api/build.rs` does `include!("../../build_meta.rs")`. The
# Dockerfile copies crates/, src/, benches/, examples/ and models.toml — but not that
# file — so both arches failed at ~250s with "couldn't find file `build_meta.rs`"
# while every local `cargo build` passed, because the file is on disk here. v0.38.4
# never hit it: the include landed afterwards, in the same commit that created it.
#
# `cargo metadata` cannot see this, so check the include targets directly.
python3 - "$ROOT" "$DOCKERFILE" <<'PY'
import os, pathlib, re, sys

root, dockerfile = sys.argv[1], sys.argv[2]
eq = os.path.join(root, "edgequake")
copy_srcs = set()
for line in pathlib.Path(dockerfile).read_text().splitlines():
    m = re.match(r"^\s*COPY\s+(?:--\S+\s+)*(\S+)\s+(\S+)\s*$", line, re.IGNORECASE)
    if m:
        copy_srcs.add(m.group(1))

problems = []
pat = re.compile(r'include!\(\s*"([^"]+\.rs)"\s*\)')
for script in pathlib.Path(eq).rglob("build.rs"):
    text = script.read_text(errors="ignore")
    for target in pat.findall(text):
        resolved = os.path.normpath(os.path.join(script.parent, target))
        rel = os.path.relpath(resolved, root)
        if not os.path.isfile(resolved):
            problems.append(f'{script} includes "{target}" but {rel} does not exist')
            continue
        # The Docker context is the repo root; a COPY of this repo-relative path (or of
        # any directory containing it) puts the file in the image.
        covered = any(
            rel == src or rel.startswith(src.rstrip("/") + "/")
            for src in copy_srcs
        )
        if not covered:
            problems.append(
                f'{script} includes "{target}" ({rel}) but the Dockerfile never COPYs it '
                f'— the image build fails with "couldn\'t find file"'
            )

if problems:
    for p in problems:
        print(f"ERROR: {p}", file=sys.stderr)
    sys.exit(1)
PY

echo "Docker API context OK (benches/examples + build-script includes present, dockerignore + Dockerfile aligned)"
