# 07 — Upgrade path matrix

Parent: [README](README.md) · Epochs: [03](03-release-schema-evolution.md) · Proof: [09](09-test-proof-protocol.md)

Goal: from **each published schema epoch**, `edgequake migrate` (HEAD binary) reaches HEAD ledger **without** undocumented env vars.

Duration class (empty-ish DB, PG16): **S** <30s, **M** 30s–3min, **L** 3–15min, **XL** depends on rowcount (Phase D). Classes are hypotheses until WP-10 measures; SPEC-93 v0.22→M141 was 85s (PG16) / 397s (PG17) on 600 docs.

`repair?` = HEAD today needs `EDGEQUAKE_ALLOW_CHECKSUM_REPAIR` or is **broken**. After WP-2, all listed fossils auto-accept.

| From epoch (max mig) | Tags | Fossils in ledger | Drops pending on train | Duration | Today | After WP-2/3/5 |
|----------------------|------|-------------------|------------------------|----------|-------|----------------|
| 024 | v0.2.0–v0.4.1 | — | 125/126/131 later | XL | Untested | E then D (091) then C gated |
| 025 | v0.5.1 | — | same | XL | Untested | same |
| 026 | v0.5.5–v0.6.0 | — | same | XL | Untested | same |
| 029 | v0.7.0 | — | same | XL | Untested | same |
| 030 | v0.8.0–v0.9.4 | — | same | XL | Untested | same |
| 031 | v0.9.5–v0.9.6 | — | same | XL | search_path era; pin public | same |
| 032 | v0.9.7–v0.9.19 | — | same | XL | — | same |
| 034 | v0.10.0–v0.10.5 | — | same | XL | — | same |
| 035 | v0.10.6–v0.10.12 | **019=`7b544306…`** | same | XL | **BROKEN** no repair | WP-2 accepts 019 fossil |
| 035 + 001 fossil | **v0.11.0 only** | **001=`9e44513e…`** | same | XL | **BROKEN** #195 class | WP-2 accepts 001 fossil |
| 035 restored | v0.11.1–v0.11.2 | — | same | XL | lockfile era starts v0.11.2 | OK |
| 036 | v0.11.3–v0.12.5 | — | same | XL | — | OK |
| 037 | v0.12.6 | — | same | XL | — | OK |
| 038 | v0.12.7–v0.12.11 | — | same | L+ | GIN via support/038 | Phase D/index job |
| 077 | v0.13.0–v0.13.1 | **071=`fa6cce9c…`** | same | L+ | needs repair 71 | auto |
| 078 | **v0.13.2** | **078=`d22cc6d8…`** (+071 if applied old) | same | L+ | #273; repair 78 | auto |
| 079 | v0.13.3 | 071 until 0.14 | same | L+ | — | auto 71 |
| 081 | v0.14.0–v0.15.1 | 071 current | same | L+ | #275 halfvec | OK if 71 repaired |
| 083 | v0.16.0 | — | same | L | — | OK |
| 086 | v0.17.0–v0.18.0 | — | same | L | — | OK |
| 089 | v0.19.0 | — | same | L | lease view | OK |
| 094 | v0.20.0 | — | same | L | — | OK |
| 095 | v0.20.1–v0.20.2 | — | same | L | X-03 eq_source_id | support/092 → migrate-only |
| 097 | v0.21.0 | — | same | L | — | OK |
| 098 | v0.21.1–v0.21.3 | — | same | L | — | OK |
| 105 | **v0.22.0** | — | 125+ after 106 | M–L | SPEC-93 GREEN synthetic | WP-8 realistic seed |
| 141 | **v0.23.0** | **118,121,125,131 old hashes** | 125/126/131 if not dropped | M | SPEC-110/111 | auto fossils |
| 142 | v0.24.0–v0.24.1 | same fossils if not repaired | 142 deferred | M | #362–364 | auto |
| 144 | v0.24.2–v0.24.3 | current 118+ | mid-cutover | M | Card 143/144 | OK |
| 147 | v0.24.4 | — | mid-cutover | M | #374 | OK |
| 148 | **v0.25.0** | — | 125/126/131 legal pending | M | SPEC-137 consent | OK (0.26.1 CLI) |
| 149 | **v0.26.0–v0.26.10** | — | leftover drops possible | S–M | SPEC-139, #396 | data engine honesty |
| 158 | HEAD unreleased | — | — | S fresh | not in GHCR | release train |

## Paths that are broken **today** (must WP-2)

```text
  v0.10.6 -- v0.10.12  ledger checksum(19) = 7b544306...
       HEAD embed(19) = 1f538faa...
       sqlx VersionMismatch  -- no module --

  v0.11.0              ledger checksum(1) = 9e44513e...
       HEAD embed(1)  = bb40c61f...
       same, #195     -- no module --
```

## Mid-cutover (v0.23+)

Serving on pending **only** 125/126/131 (+ deferred 142) is legal (`pending_ok_to_serve`). Operators:

1. `edgequake migrate` — Phase E (and D copy).
2. When `guard` GREEN: `edgequake migrate --confirm-drop` — Phase C.
3. Next migrate applies 142 if residue gone.

Do not weaken DROP SQL (SPEC-137 LAW-137-3). Image of **migrator** must be ≥ API (D-14).

## Fresh install

Empty `_sqlx_migrations` → runner applies 001..HEAD (or squash after WP-10). Extensions still from initdb (`init-extensions.sql`) **or** 001 (`CREATE EXTENSION` guarded). Dual create must stay idempotent.

## PostgreSQL majors

SPEC-93: PG16/17/18 GREEN on v0.22 synthetic. AGE + pgvector **required**. #280 was volume layout, not SQL. Matrix CI: three majors, AGE enabled (WP-8). Non-goal: `pg_upgrade` of PG itself ([12](12-risks-honest-assessment.md)).
