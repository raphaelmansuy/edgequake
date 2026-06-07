# CI Publish Idempotency

> **Problem**: `cargo publish` exits 101 when the crate version already exists on crates.io.
> If a developer publishes manually before the tag-triggered CI runs (or if CI retries),
> the workflow turns red despite the crate being correctly published.

---

## Root cause

`cargo publish` is **not idempotent** — it always fails if the version exists:

```
error: crate edgequake-llm@0.6.15 already exists on crates.io index
Process completed with exit code 101.
```

Crates.io is an **append-only** registry; you cannot overwrite or delete a published
crate version. There is no `--skip-if-exists` flag in stable Cargo.

---

## Decision: check-before-publish

The CI workflow was updated to query crates.io before attempting to publish:

```bash
VERSION=$(grep '^version' Cargo.toml | head -n1 | cut -d'"' -f2)
PUBLISHED=$(curl -sf "https://crates.io/api/v1/crates/edgequake-llm/$VERSION" \
  -H "User-Agent: edgequake-llm-ci/1.0" | python3 -c "import json,sys; \
  d=json.load(sys.stdin); print('yes')" 2>/dev/null || echo "no")

if [ "$PUBLISHED" = "yes" ]; then
  echo "✓ edgequake-llm@$VERSION is already published on crates.io — skipping."
  exit 0
fi

cargo publish --locked
```

### Why crates.io API (not `cargo search`)

`cargo search` uses a full-text index that can be **up to 15 minutes stale** after
a publish. The JSON API at `/api/v1/crates/{name}/{version}` reflects the live
index and returns 404 for unpublished versions.

### Edge cases handled

| Scenario | Behaviour |
|---|---|
| Version already published (manually or prior run) | Exits 0, logs a ✓ message |
| Version not yet published | Runs `cargo publish --locked` normally |
| crates.io API unreachable (network outage) | Falls through to `cargo publish`; if version exists, cargo returns 101 → step fails (desired: surface the network issue) |
| Token missing / invalid | `cargo publish` fails with auth error (correct behaviour) |
| Cargo.toml version ≠ tag | Already caught in `preflight` job (`Verify tag matches Cargo.toml version` step) |

---

## Alternatives considered

| Alternative | Rejected because |
|---|---|
| `cargo publish || true` | Silently swallows ALL errors, including auth failures, network errors, and packaging bugs |
| `cargo publish 2>&1 \| grep -v "already exists" \|\| ...` | Fragile string matching; breaks if Cargo changes the error message |
| Remove the workflow trigger on manual-publish | Manual publishing bypasses quality gates; defeats the purpose of CI |
| Use `cargo publish --no-verify` | Skips packaging checks; makes the CI less trustworthy |

---

## Workflow diff (publish.yml)

The change replaces the single-step `Publish` with a two-step sequence:

```yaml
- name: Check if already published
  shell: bash
  run: |
    VERSION=$(grep '^version' Cargo.toml | head -n1 | cut -d'"' -f2)
    STATUS=$(curl -sf -o /dev/null -w "%{http_code}" \
      "https://crates.io/api/v1/crates/edgequake-llm/$VERSION" \
      -H "User-Agent: edgequake-llm-ci/1.0" || echo "000")
    if [ "$STATUS" = "200" ]; then
      echo "ALREADY_PUBLISHED=true" >> "$GITHUB_ENV"
      echo "✓ edgequake-llm@$VERSION already on crates.io — publish step will be skipped."
    else
      echo "ALREADY_PUBLISHED=false" >> "$GITHUB_ENV"
    fi

- name: Publish to crates.io
  if: env.ALREADY_PUBLISHED == 'false'
  env:
    CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}
  run: cargo publish --locked
```

This approach:
- Sets an env var to pass state between steps (the idiomatic GitHub Actions pattern).
- Skips the publish step entirely (not just exits 0) so the step shows "skipped" (grey)
  rather than "passed" (green), making it visually clear what happened.
- Keeps `cargo publish --locked` for the actual publish to enforce lockfile consistency.
