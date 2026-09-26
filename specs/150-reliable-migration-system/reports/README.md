# Epoch matrix reports

Committed artifacts (lean):

- `pg{16,17,18}/*.json` — per-epoch status / duration / ledger_max
- `pg{16,17,18}/SUMMARY.md` — table view

Regenerate schema dumps + full logs locally:

```bash
make spec150-matrix-quick   # key epochs
make spec150-matrix PG=all  # full 56-case matrix (FORCE_REPLAY=1 recommended)
```

See parent [README](../README.md) honesty table for what “ok” means.
