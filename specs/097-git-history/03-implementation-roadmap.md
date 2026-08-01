# SPEC-097 — Implementation Roadmap

> **DoD**: Gates G1–G5 green; #351 closable with before/after pack metrics.

## Wave 0 — Spec + inventory

- [x] SPEC-097 pack under `specs/097-git-history/`
- [x] Measured findings F-351-01…11

## Wave 1 — Stop the bleeding (tip)

- [x] Expand `.gitignore` fat globs (all stages, not only medical-full)
- [x] `git rm --cached` tracked fat paths (leave on disk)
- [x] `archive_run` writes `LOCAL_ONLY.md`; fat still copied locally
- [x] Update SPEC-001 e2e README artifact layout
- [x] `tools/git-hygiene/check_no_fat_artifacts.sh` + `make git-hygiene` + CI

## Wave 2 — History rewrite

- [ ] Fresh mirror clone
- [ ] `git filter-repo --invert-paths` for fat globs + `sdks/swift/.build/**` + `zz_test_docs/**`
- [ ] Verify history-path blob sum &lt; ~20 MB; pack shrink
- [ ] Force-push rewritten refs; announce re-clone / PR rebase

## Wave 3 — Closeout

- [x] CONTRIBUTING note
- [ ] Close #351 with metrics
- [ ] Task log under `logs/`

## Wave 4 — Gates

See [04-e2e-test-matrix.md](04-e2e-test-matrix.md).
