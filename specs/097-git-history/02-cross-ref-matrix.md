# SPEC-097 — Cross-Ref Matrix

| Surface | Path / symbol | Law | Finding | Gate |
|---------|---------------|-----|---------|------|
| Issue | [#351](https://github.com/raphaelmansuy/edgequake/issues/351) | G1–G6 | F-351-01 | — |
| Ignore | [`.gitignore`](../../.gitignore) fat globs | G3, G5 | F-351-07 | G1 |
| Archive | [`archive_run`](../../tools/bench001/bench001/progress.py) | G3 | F-351-08 | G2 |
| Guard | [`tools/git-hygiene/check_no_fat_artifacts.sh`](../../tools/git-hygiene/check_no_fat_artifacts.sh) | G5 | F-351-03 | G4 |
| Make | `make git-hygiene` | G5 | — | G4 |
| CI | `.github/workflows/ci.yml` job `git-hygiene` | G5 | — | G4 |
| Thin history | `specs/001-benchmark/e2e/artifacts/history/*/{scorecard,SUMMARY,…}` | G2 | F-351-04 | G3 |
| Publish SSOT | `specs/001-benchmark/e2e/artifacts/publish/` | G2 | F-351-05 | — |
| Rewrite | [`runbook/history-rewrite.md`](runbook/history-rewrite.md) | G4 | F-351-02,09,10 | G3, G5 |
| Docs | [`specs/001-benchmark/e2e/README.md`](../001-benchmark/e2e/README.md) | G1 | — | — |
| Contrib | [`CONTRIBUTING.md`](../../CONTRIBUTING.md) | G1 | — | — |
