# SPEC-097 — Edge Cases

| ID | Case | Handling |
|----|------|----------|
| **EC-01** | Open PRs after force-push | Rebase onto rewritten `edgequake-main`; do not merge pre-rewrite tip |
| **EC-02** | Forks / old clones | Must **re-clone**; `git pull` resurrects deleted blobs |
| **EC-03** | GitHub UI size lag | Server GC may delay; trust local `git count-objects -vH` |
| **EC-04** | Accidental `git add -f` fat JSON | `make git-hygiene` / CI fails; remove from index |
| **EC-05** | Local forensics still needed | Fat files remain on disk under `history/`; only untracked |
| **EC-06** | Doc links to `predictions_*.json` | Rare; retarget to `scorecard.json` or `publish/peers` |
| **EC-07** | `medical-full` already ignored | Broader globs supersede; keep behavior |
| **EC-08** | Allowlisted large fixtures | Guard allowlist for intentional test fixtures if any remain &gt; 50 MiB |
