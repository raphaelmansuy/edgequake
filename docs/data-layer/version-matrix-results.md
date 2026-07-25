# Version matrix live results

Captured on local PG18.4 (vector 0.8.5, age 1.8.0) — 2026-07-25.

| Suite | PG18 | PG17 | PG16 |
|---|---|---|---|
| data_layer_ops_matrix (235 Ref IDs) | **pass** | pending CI battle | pending CI battle |
| data_layer_scaling | **pass** | pending CI battle | pending CI battle |
| data_layer_limits | **pass** | pending CI battle | pending CI battle |
| data_layer_registry | **pass** (no DB) | **pass** | **pass** |
| lint_dataop_xref | **pass** | **pass** | **pass** |

PG16/17 filled by `.github/workflows/data-layer-matrix.yml` schedule/`battle=true`.
