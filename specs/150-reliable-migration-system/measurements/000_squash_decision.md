# SPEC-150 WP-10 measurements

**Squash decision:** `no_squash`

T_fresh < 180s on all majors (measured twice if needed). No 000_baseline squash.

## T_fresh (HEAD migrate --confirm-drop on empty DB)

| PG | T_fresh_s | ledger_max |
|---:|---:|---:|
| 16 | 2.31 | 159 |
| 17 | 1.2 | 159 |
| 18 | 1.13 | 159 |

## T_upgrade proxy (epoch matrix wall time per case)

| PG | n | min_s | max_s | avg_s |
|---:|---:|---:|---:|---:|
| 16 | 32 | 8 | 19 | 10.81 |
| 17 | 12 | 6 | 10 | 7.75 |
| 18 | 12 | 6 | 9 | 7.75 |

No converge migrations 160+ required — schema diffs empty after restrict-token filter.
