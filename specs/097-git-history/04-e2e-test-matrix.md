# SPEC-097 — Verification Gates

| Gate | Command / check | Pass |
|------|-----------------|------|
| **G1** | `git ls-files` for fat globs | Zero paths |
| **G2** | `archive_run` still produces thin + local fat + `LOCAL_ONLY.md` | Unit/manual OK |
| **G3** | After rewrite: history-path uncompressed blob sum | &lt; ~20 MB |
| **G4** | `make git-hygiene` | Exit 0 on clean tree; fails if fat staged or blob &gt; 50 MiB (non-allowlist) |
| **G5** | Fresh clone / `git count-objects -vH` | Pack materially smaller than pre-rewrite ~700 MB |

## Quick commands

```bash
make git-hygiene

# G1
git ls-files 'specs/001-benchmark/e2e/artifacts/**/predictions_*.json' | wc -l
git ls-files 'specs/001-benchmark/e2e/artifacts/**/eval_*.json' | wc -l

# G3 (post-rewrite)
git rev-list --objects --all -- 'specs/001-benchmark/e2e/artifacts/history' \
  | git cat-file --batch-check='%(objecttype) %(objectname) %(objectsize) %(rest)' \
  | awk '/^blob/ {s+=$3; n++} END {printf "blobs=%d size=%.1f MB\n", n, s/1024/1024}'
```
