# SPEC-097 — History rewrite runbook

> **LAW-G4**. Use a **fresh mirror clone**. Tool: [git-filter-repo](https://github.com/newren/git-filter-repo).  
> Destructive: rewrites all SHAs. Coordinate force-push; contributors re-clone.

## Preconditions

```bash
# Backup
git clone --mirror git@github.com:raphaelmansuy/edgequake.git edgequake-mirror-backup.git

# Work clone
git clone --mirror git@github.com:raphaelmansuy/edgequake.git edgequake-filter.git
cd edgequake-filter.git
```

Record before metrics:

```bash
git count-objects -vH
gh api repos/raphaelmansuy/edgequake --jq '{size_kb:.size}'
```

## Filter

```bash
git filter-repo --force --invert-paths \
  --path-glob 'specs/001-benchmark/e2e/artifacts/**/predictions_*.json' \
  --path-glob 'specs/001-benchmark/e2e/artifacts/**/eval_*.json' \
  --path-glob 'specs/001-benchmark/e2e/artifacts/**/eval_*.raw.json' \
  --path-glob 'specs/001-benchmark/e2e/artifacts/**/logs/**' \
  --path-glob 'sdks/swift/.build/**' \
  --path-glob 'zz_test_docs/**'
```

Note: `git filter-repo` removes `origin`. Re-add before push:

```bash
git remote add origin git@github.com:raphaelmansuy/edgequake.git
```

## Verify

```bash
git count-objects -vH

git rev-list --objects --all -- 'specs/001-benchmark/e2e/artifacts/history' \
  | git cat-file --batch-check='%(objecttype) %(objectname) %(objectsize) %(rest)' \
  | awk '/^blob/ {s+=$3; n++} END {printf "blobs=%d size=%.1f MB\n", n, s/1024/1024}'
# Target: << 20 MB uncompressed for history path
```

## Push

```bash
git push --force --mirror origin
```

## Announce (issue / Discussions)

```text
SPEC-097 / GH-351: git history rewritten to remove regenerable bench001 fat JSON
and build ghosts (swift .build, zz_test_docs).

All commit SHAs changed. Please:
1. Re-clone (do not pull old clones).
2. Rebase open PRs onto the new edgequake-main tip.

Thin history/<run>/ scorecards and publish/ peers are preserved.
```

## Local working copy after rewrite

```bash
# Preferred: fresh clone
git clone git@github.com:raphaelmansuy/edgequake.git
```
