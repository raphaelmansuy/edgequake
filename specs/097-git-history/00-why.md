# SPEC-097 — WHY (Five WHYs)

> **Cross-refs**: [README](README.md) · [Laws](00-first-principles.md) · [GH-351 study](issues/GH-351-benchmark-history-disk.md)  
> **Issue**: https://github.com/raphaelmansuy/edgequake/issues/351  
> **Product pin**: EdgeQuake v0.22.0+

---

## Symptom (reporter)

The repository is over 5 GB on disk. Cloning for source builds (e.g. containers) is very slow. The vast majority (~4 GB) sits under `specs/001-benchmark/e2e/artifacts/history/` — 160+ benchmark run folders with large `medical-mid-*` (~100 MB+) and `smoke-*` (~10–30 MB) trees.

---

## Five WHYs

### WHY 1 — Why is the working tree / checkout so large?

Because each `archive_run` copies full `predictions_*.json`, `eval_*.json`, `eval_*.raw.json`, and often `logs/progress.jsonl` into `history/<stage>-<timestamp>/`, and those files were committed to git.

### WHY 2 — Why were they committed?

Because Acc forensics and ladder progression treated the archive directory as the durable record. Thin scorecards were not distinguished from regenerable bulk JSON when publishing commits. Only `medical-full-*/predictions|eval` were later gitignored (GitHub 100 MB limit) — smoke/mid fat remained tracked.

### WHY 3 — Why does deleting them in a new commit not fix clone time?

Because git packs retain every historical blob. Tip `git rm` leaves ~4 GB of uncompressed history-path blobs (and ~700 MB compressed pack) until history is rewritten with [git-filter-repo](https://github.com/newren/git-filter-repo).

### WHY 4 — Why not Git LFS?

Because these files are **experiment scratch**, not distribution assets. LFS still costs bandwidth on clone/checkout for contributors who never run benches. Regenerable via `make bench001-*`; Acc claims already live in thin `publish/` SSOT.

### WHY 5 — Why does this matter for open source?

GitHub recommends repos ideally &lt; 1 GB and strongly &lt; 5 GB ([large files docs](https://docs.github.com/en/repositories/working-with-files/managing-large-files/about-large-files-on-github)). Slow clones tax every contributor, CI checkout, and container build — a structural barrier unrelated to product code quality.

**Root cause:** Regenerable bulk benchmark outputs were versioned as if they were source. Policy and ignore rules never split **thin SSOT** from **fat forensics**, and packs were never rewritten after the bulk landed.

---

## Causal ASCII

```
  make bench001-* / archive_run
           |
           v
  history/<run>/{scorecard,SUMMARY,...}   ← thin (~KB–MB)  →  SHOULD be in git
  history/<run>/{predictions,eval,*.raw}  ← fat (~10–100MB) →  MUST be local-only
           |
           v
  git commit (fat tracked) ──► pack grows ──► clone / CI / docker slow
           |
           +── tip gitignore alone ──► NEW commits OK, OLD blobs remain
           |
           v
  git filter-repo --invert-paths ──► packs shrink ──► clone healthy
```
