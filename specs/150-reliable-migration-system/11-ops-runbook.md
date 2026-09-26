# 11 — Operator runbook (SPEC-150)

Parent: [README](README.md) · Short doc: [`edgequake/docs/migrations.md`](../../edgequake/docs/migrations.md)

## Golden path

```text
  1. Backup (pg_dump -Fc or volume snapshot)
  2. edgequake migrate dry-run          # preview
  3. edgequake migrate                  # SAFE SCHEMA (drops still gated)
  4. edgequake migrate drain            # data engine / cutover jobs (optional timeout)
  5. If guard GREEN and you intend to drop legacy stores:
       edgequake migrate --confirm-drop
  6. Roll / start API (SCHEMA_GATE=wait recommended under orchestration)
  7. curl -sf "$HOST/ready"
```

Do **not** pass unknown flags (SPEC-137). Consent token: `--confirm-drop` (alias `--drop-confirm`). Env equivalent: `EDGEQUAKE_MIGRATION_CONFIRM_DROP=1`.

Migrator binary **≥** API binary.

## Compose

Migrate is a one-shot service; API waits on `service_completed_successfully` and sets `EDGEQUAKE_SCHEMA_GATE=wait` (quickstart, `edgequake/docker/*`, GCP overlay).

```bash
docker compose up -d   # migrate runs first; API binds /live while waiting if needed
```

## Helm

- **External DB** (`postgres.enabled=false`): Job hooks `pre-install,pre-upgrade` + hook-scoped env Secret.
- **Bundled postgres**: non-hook Job `edgequake-migrate-r{{ .Release.Revision }}` (DB does not exist at pre-install); API wait-mode + startupProbe on `/live`.

## ECS / systemd

- Task/oneshot: `edgequake migrate` before serve.
- systemd: `Type=oneshot` migrate unit `Before=edgequake.service`.

## make dev

`VISIBLE_MIGRATE_STEP` runs `edgequake migrate` before backend start. Known fossils are auto-accepted from `migrations/manifest.toml` (no env). Unknown hashes still need `EDGEQUAKE_ALLOW_CHECKSUM_REPAIR=<versions>` or a new migration.

## Reading status / probes

| Signal | Meaning |
|--------|---------|
| `edgequake migrate status` | Pending versions + last `migration_run` (after 159) |
| `/live` 200 | Process up (wait-mode lite router or full server) |
| `/ready` 503 | Schema / index gate not ready — run migrate or wait |
| `/ready` 200 | Safe for traffic |
| Exit 78 | Boot gate refuse (`BOOT_GATE_REFUSAL:`) |
| Exit 75 | Migrate lock busy (`MIGRATE_LOCK_BUSY`) |
| Exit 65 | Unknown checksum (not a listed fossil) |

`/ready` re-evaluates live (TTL cache). Large-graph 038: `scripts/migrations/apply_038.sh --concurrent` then wait for `/ready`.

## Recovery

### VersionMismatch (checksum)

1. Confirm body was not edited after ship (LAW: new file, never patch).
2. If hash matches a **non-`dev_only`** fossil in `manifest.toml`, re-run `edgequake migrate` (auto-accept).
3. Else scoped override: `EDGEQUAKE_ALLOW_CHECKSUM_REPAIR=NNN` for one shot, then remove the env.
4. `EDGEQUAKE_DEV_MODE` does **not** bypass unknown hashes.

### Dirty version

Message includes version, description, and pointer here. Fix the partial apply (manual SQL / re-run) before continuing. Do not delete `_sqlx_migrations` rows casually.

### Concurrent migrate / stuck lock

Wait for the other Job, or after crash ensure no session holds `hashtext('edgequake.migrate.run')`. Deadline: `EDGEQUAKE_MIGRATE_LOCK_DEADLINE` (default 60s) → exit 75.

### Serve still writing DDL

Default serve path is verify-only. Escape hatch (one release): `EDGEQUAKE_SERVE_RECONCILE=1`. Prefer fixing the migrate Job instead.

## Related

- Manifest SSOT: `edgequake/migrations/manifest.toml`
- Epoch proof: `make spec150-matrix` / `make spec150-matrix-quick`
- Risks: [12-risks-honest-assessment.md](12-risks-honest-assessment.md)
