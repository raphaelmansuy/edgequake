# SPEC-042 — Product Owner Lens

**Audience:** Release manager, operators, customer success  
**Question:** What does Issue #161 deliver — and do we force PG18?

---

## Position: triple-track (no forced migration)

| Tier | Who | Stack |
| ---- | --- | ----- |
| **PG16 legacy** | Existing prod | AGE 1.6.0 — stay without migration |
| **PG17 modern** | RDS PG17, incremental migrate | AGE 1.7.0 — **full #161** |
| **PG18 recommended** | New installs | AGE 1.7.0 — longest runway |

**PO message:** “We upgraded extensions and added PG18 as the recommended path. PG16 remains supported until ~2028.”

---

## User-visible outcome

| Before | After (PG16 path) | After (PG17/PG18 path) |
| ------ | ----------------- | ---------------------- |
| pgvector 0.7.x on old volumes | Auto 0.8.3 + reindex | Same |
| AGE catalog drift | M043 auto sync | Same + AGE 1.7.0 |
| No extension visibility | `/health` ext versions | Same |
| Single PG16 pin | PG16 + PG17 + PG18 images | AGE 1.7.0 on modern tiers |

---

## Issue #161 framing (honest)

| Audience on… | #161 status |
| ------------ | ----------- |
| PG16 | **Partial** — latest for PG16 (AGE 1.6.0) |
| PG17 | **Full** — AGE 1.7.0 |
| PG18 | **Full** — AGE 1.7.0 |

Do not claim “AGE 1.7.0 everywhere” if PG16 supported tier remains.

---

## What we are **not** shipping

1. **Forced PG18 migration** — opt-in via `migrate_postgres_major.sh`
2. **AGE 1.7.0 on PG16** — physically impossible (upstream constraint)
3. **Two application versions** — one binary, three Docker DB images

---

## Release notes snippet

> **PostgreSQL:** EdgeQuake now ships **triple-track** database images — **PG16** (legacy, AGE 1.6.0), **PG17** (modern, AGE 1.7.0), and **PG18** (recommended, AGE 1.7.0). All include pgvector **0.8.3**. Existing PG16 deployments continue without a major migration. New installs should use PG18; managed PG17 hosts can use the PG17 image. Optional migration: `scripts/migrate_postgres_major.sh`. Related: [#161](https://github.com/raphaelmansuy/edgequake/issues/161).

---

## Acceptance criteria

**PG16 tier (must not regress):**

- [ ] `make postgres-image-build` green
- [ ] `run_extension_upgrade_proof.sh` green
- [ ] `/health` age ≥ 1.6.0 on PG16

**PG17 tier:**

- [x] `make postgres-image-build-pg17` green
- [x] `check_extension_pins.sh pg17` green

**PG18 tier:**

- [ ] `make postgres-image-build-pg18` green
- [ ] `run_pg18_migration_procedure.sh` green
- [ ] `/health` age ≥ 1.7.0 on PG18

**Triple-track policy:**

- [ ] [012-dual-pg-major-compatibility.md](./012-dual-pg-major-compatibility.md) published
- [ ] No release requires PG17/18 exclusively

---

## Roadmap (Phase E)

| Milestone | Value | PG16 impact |
| --------- | ----- | ----------- |
| PG18 default in quickstart | Faster #161 adoption | PG16 image still published |
| **E-01 halfvec** | ~50% vector disk | Opt-in all tiers |
| **E-03 uuidv7** | Time-ordered doc IDs | PG18 only |
| **E-04 AGE COPY** | Bulk graph ingest | PG17+ only |
| **E-02 AGE RLS** | DB-enforced tenant isolation | PG17+ only; PG16 app-level |
| PG16 EOL (~2028-11) | End support tier | Migration comms 6 mo prior |
