# SPEC-097 — Finding Register

> Measured on local clone 2026-08-01 (pre-rewrite). Uncompressed blob sizes via `git cat-file`.

| ID | Finding | Evidence | Law | Disposition |
|----|---------|----------|-----|-------------|
| **F-351-01** | `history/` working tree ~4.4 GB / 164 runs | `du -sh specs/001-benchmark/e2e/artifacts/history` | G1 | Strip fat; keep thin |
| **F-351-02** | History-path blobs ~4.17 GB (2073) in object DB | `git rev-list --objects --all -- …/history` | G4 | filter-repo invert |
| **F-351-03** | Fat dominated by predictions/eval/raw (~4.2 GB WT) | basename size rollup | G3 | gitignore + rm --cached |
| **F-351-04** | Thin keepables ~4 MB total | scorecard/SUMMARY/BUSINESS_REPORT rollup | G2 | Keep tracked |
| **F-351-05** | `publish/` ~0.4 MB — Acc SSOT | blob sum under `artifacts/publish` | G2 | Unchanged |
| **F-351-06** | GitHub repo size ~717 MB; pack ~700 MB | `gh api …/.size`, `git count-objects -vH` | G4 | Expect shrink post-rewrite |
| **F-351-07** | Partial ignore only for `medical-full-*` fat | `.gitignore` lines 148–153 | G5 | Broaden globs |
| **F-351-08** | `archive_run` always copies fat JSON | `tools/bench001/bench001/progress.py` | G3 | Keep local copy; emit LOCAL_ONLY |
| **F-351-09** | `sdks/swift/.build` ~626 MB still in history | rev-list blob sum | G4 | Strip in rewrite |
| **F-351-10** | `zz_test_docs` ~116 MB still in history | rev-list blob sum | G4 | Strip in rewrite |
| **F-351-11** | Follow-up: large tracked PDFs / CSV outside history | e.g. `queryedgeQuake.csv` 45 MB, SPEC-049 PDFs | G1 | Out of scope #351; track as F-351-FUP |

## Follow-ups (not this issue)

| ID | Item | Notes |
|----|------|-------|
| **F-351-FUP-01** | `specs/012-performance/data/queryedgeQuake.csv` | Optional later strip / externalize |
| **F-351-FUP-02** | SPEC-049 / API fixture PDFs | Keep if needed for tests; else LFS/releases |
