# SPEC-097 — First Principles

> **Cross-refs**: [WHY](00-why.md) · [Roadmap](03-implementation-roadmap.md) · [SPEC-001 e2e](../001-benchmark/e2e/README.md)  
> **External**: [git-filter-repo](https://github.com/newren/git-filter-repo) · [GitHub large files](https://docs.github.com/en/repositories/working-with-files/managing-large-files/about-large-files-on-github)

---

## Laws

| Law | Statement |
|-----|-----------|
| **LAW-G1** | Git stores **source + small SSOT claims**, never regenerable bulk run outputs. |
| **LAW-G2** | Acc / latency **claims** live in thin artifacts: `publish/latest`, `publish/peers/*`, and per-run `scorecard.json` + human reports. |
| **LAW-G3** | Fat run outputs (`predictions_*.json`, `eval_*.json`, `eval_*.raw.json`, `logs/progress.jsonl`) are **local-only**; regenerable via `make bench001-*`. |
| **LAW-G4** | Stopping future commits is insufficient; **history must be rewritten** so clones stop downloading dead blobs. |
| **LAW-G5** | Prevention > cleanup: expand `.gitignore` + add a size/path guard so CI/`make` blocks reintroduction. |
| **LAW-G6** | Do **not** use Git LFS for these artifacts — they are not distribution assets; they are experiment scratch. |

---

## First principles (decomposition)

### 1. What belongs in git?

| Keep (thin) | Drop (fat) |
|-------------|------------|
| `scorecard.json` | `predictions_eq.json` / `predictions_lr.json` |
| `SUMMARY.md` / `BUSINESS_REPORT.md` / `EXEC_SUMMARY.txt` | `eval_*.json` / `eval_*.raw.json` |
| `meta.json` / `eq_workspace.json` / `progress.json` (small) | `logs/progress.jsonl` |
| `ABLATION_NOTE.md` / `LIVE.md` / `LOCAL_ONLY.md` | Build ghosts: `sdks/swift/.build/**`, `zz_test_docs/**` |
| `publish/latest` + `publish/peers/*` | — |

### 2. Why keep thin `history/<run>/`?

SPEC-001 improvement notes deep-link run folders for Acc numbers. Thin scorecards preserve those links without multi-GB packs. Publish peers remain the business SSOT.

### 3. Why rewrite history?

Per GitHub and git-filter-repo docs: removing a file from the tip leaves blobs in the object database. Clone downloads packs built from the full commit graph. Only path invert / blob strip reclaims space.

### 4. SOLID / DRY

| Principle | Application |
|-----------|-------------|
| **S** | `.gitignore` owns ignore policy; `archive_run` owns local layout; `check_no_fat_artifacts.sh` owns enforcement; runbook owns rewrite. |
| **O** | New fat patterns = extend ignore globs + guard allowlist once. |
| **D** | Contributors depend on thin SSOT paths, not on local fat JSON. |
| **DRY** | One glob family in `.gitignore`; guard script mirrors the same patterns. |

---

## Normative ignore family (tip)

```gitignore
specs/001-benchmark/e2e/artifacts/**/predictions_*.json
specs/001-benchmark/e2e/artifacts/**/eval_*.json
specs/001-benchmark/e2e/artifacts/**/eval_*.raw.json
specs/001-benchmark/e2e/artifacts/**/logs/progress.jsonl
specs/001-benchmark/e2e/artifacts/history/**/logs/
```
