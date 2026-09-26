# 01 — First principles (LAW-150)

Parent: [README](README.md) · Why: [00](00-why.md) · Target: [06](06-target-architecture.md)

These laws are derived from Postgres, sqlx, and orchestrators — not from EdgeQuake taste. Code that violates a law is a defect ([05](05-root-cause-analysis.md)).

## LAW-150-1 — The ledger is the truth

sqlx 0.8.6 `Migrator::run` *"validate[s] previously applied migrations against the current migration source to detect accidental changes"* ([docs.rs sqlx 0.8.6 Migrator](https://docs.rs/sqlx/0.8.6/sqlx/migrate/struct.Migrator.html)). Applied rows live in `public._sqlx_migrations` (version, checksum SHA-384, success).

```text
  disk NNN_*.sql  --SHA-384-->  binary embed (migrate!)
                                    |
                                    v
  _sqlx_migrations.checksum  ==?  embedded checksum
         |                            |
         +-- mismatch --> VersionMismatch (hard fail)
         +-- missing file + ignore_missing=false --> VersionMissing
         +-- success=false --> Dirty(version)
```

**Implication:** never delete or silently rewrite a shipped body. If a historical body must be accepted, record **every known SHA-384** as a first-class variant of that version. The live file on disk stays the current bytes; the registry lists the fossils.

## LAW-150-2 — Migration and serving are separate lifecycles

Two processes, two failure domains:

```text
  MIGRATE LIFECYCLE                         SERVE LIFECYCLE
  -----------------                         ---------------
  acquire run lock (deadline)               bind HTTP
  apply expand / data / contract            GET /live  = process up
  write ledger + run telemetry              GET /ready = schema in window
  exit 0 | non-zero                         never INSERT/DDL schema
```

Serving may **read** `_sqlx_migrations` and a run-status table. Serving may **not** `UPDATE` checksums, `CREATE INDEX`, `ALTER TABLE`, or unbounded `INSERT..SELECT`.

GitLab: schema migrations run **before** new application code; they must not require taking the installation offline ([style guide](https://docs.gitlab.com/development/migration_style_guide/)).

## LAW-150-3 — Expand, then data, then contract (N-1 compatible)

```text
  Phase E  EXPAND     add nullable columns, new tables, new indexes that
                      old binaries ignore. Old pods keep serving.

  Phase D  DATA       batched, committed per batch, resumable jobs.
                      Dual-write / backfill. Never DROP.

  Phase C  CONTRACT   NOT VALID -> VALIDATE, DROP TABLE, DROP COLUMN.
                      Human-gated. New binaries may require it;
                      old binaries are already gone.
```

Postgres lock facts that make this mandatory ([explicit locking](https://www.postgresql.org/docs/current/explicit-locking.html)):

- `CREATE INDEX` (no `CONCURRENTLY`) takes **SHARE** — blocks `INSERT`/`UPDATE`.
- Many `ALTER TABLE` / `DROP TABLE` take **ACCESS EXCLUSIVE** — blocks even `SELECT`.
- *"Once acquired, a lock is normally held until the end of the transaction."*
- *"a transaction seeking either a table-level or row-level lock will wait indefinitely"* unless `lock_timeout` is set.

`CREATE INDEX CONCURRENTLY` *"cannot be executed within a transaction block"* ([CREATE INDEX](https://www.postgresql.org/docs/current/sql-createindex.html)). sqlx 0.8 wraps each file in a transaction unless the file starts with `-- no-transaction` (`sqlx-core-0.8.6/src/migrate/source.rs:127`). Therefore CONCURRENTLY **cannot** live in a default sqlx file.

Constraints: add `NOT VALID`, then `VALIDATE CONSTRAINT` in a later step so the ACCESS EXCLUSIVE window is the add, not the scan ([ALTER TABLE](https://www.postgresql.org/docs/current/sql-altertable.html)). Doing both in **one** sqlx transaction (M141) holds the exclusive lock through the validate — that is a LAW-150-3 violation.

## LAW-150-4 — Shipped files are immutable; known variants are data

`checksums.lock` is a CI pin of **current** bytes. It is not a history of deployed bytes. SPEC-111's allowlist env var is an operator ritual, not a registry.

**Allowed:** a versioned map `{ version -> [sha384, ...] }` compiled into the migrator, including 001 `9e44513e…` and 019 `7b544306…`.

**Forbidden:** rewriting `_sqlx_migrations.checksum` for an **unknown** hash; editing a numbered file to "just make CI green" without adding a **new** version.

sqlx 0.9 adds `sqlx.toml` "characters to ignore when hashing" ([sqlx 0.9.0 changelog](https://github.com/launchbadge/sqlx/blob/main/CHANGELOG.md)). Whitespace-ignore is **not** a substitute for a variant registry: semantic edits (DISTINCT ON, `->>` vs `->>>`) must remain exact-hash matches.

## LAW-150-5 — Every step is idempotent, resumable, and lock-bounded

```text
  session start
    SET lock_timeout = '<class>'
    SET statement_timeout = '<class>'
    SELECT pg_try_advisory_lock($run_id)  -- fail or retry, never wait forever
    ...
    pg_advisory_unlock / session end
```

sqlx 0.8 uses `pg_advisory_lock` (blocking, no deadline) and only around `MIGRATOR.run` — not around checksum repair or reconcile. PostgreSQL: session-level advisory locks *"are held until explicitly released or the session ends"* and *"do not honor transaction semantics"* ([§13.3.5](https://www.postgresql.org/docs/current/explicit-locking.html)). A crashed client releases them; a **stuck waiter** does not time out.

Idempotency: `IF NOT EXISTS`, `ON CONFLICT` that cannot hit the same row twice in one statement (error `21000`), keyset batches that do not advance past failed rows.

`SET` vs `SET LOCAL`: session `SET` survives a committed transaction; `SET LOCAL` dies at COMMIT ([SET](https://www.postgresql.org/docs/current/sql-set.html)). Migrator timeouts must be **session** (or re-applied per statement), not LOCAL inside a migration that commits early (inner `BEGIN`/`COMMIT` in M128 etc.).

## LAW-150-6 — Pending schema means wait, not crash

Kubernetes: a failed **liveness** probe **kills** the container; a failed **readiness** probe only removes it from Service endpoints; a **startup** probe covers slow init without touching liveness ([Configure probes](https://kubernetes.io/docs/tasks/configure-pod-container/configure-liveness-readiness-startup-probes/)).

```text
  gate=fail (today)          gate=wait (target default)
  -----------------          --------------------------
  schema behind              schema behind
    -> exit 78                 -> bind
    -> nothing listens         -> /live 200
    -> kubelet restarts        -> /ready 503 schema_pending
                               -> poll ledger; flip 200 when in window
```

`fail` remains for operators who want crash-loop as a pager. Default for Helm/compose is `wait` so migrate Jobs and API pods can coexist.

Helm **pre-install / pre-upgrade** run after render and **before** resources are created/updated. **post-install** with `--wait` waits for readiness first ([hooks](https://helm.sh/docs/topics/charts_hooks/)). Migrate belongs on **pre-***.

Compose: `depends_on: migrate: condition: service_completed_successfully` ([startup order](https://docs.docker.com/compose/how-tos/startup-order/)).

## LAW-150-7 — Prove with realistic data; measure before optimizing

SPEC-93 was GREEN on PG16/17/18 in **85–397 s** with **600 synthetic docs** and empty-ish graphs ([matrix-summary](../93-migration-assessment/reports/matrix-summary.md)). Three days later M118 died on multi-workspace `21000`. Proof that is not shaped like production is not proof.

A squash baseline (single snapshot schema for **fresh** installs) is allowed **only after** [10](10-performance-budget.md) measurements show fresh-apply of 001..HEAD misses the budget. Squash never replaces the upgrade train for existing ledgers (LAW-150-1).

## Collision with existing laws

| Existing | Keep? | Note |
|----------|-------|------|
| SPEC-091 LD-15 (boot does not apply sqlx) | **Keep, extend** | Extend to "boot does not write" (repair + spawned SQL). |
| SPEC-111 LAW-MIG (no silent checksum rewrite) | **Keep, replace ritual** | Env allowlist becomes override for *unknown* hashes; known fossils auto-repair. |
| SPEC-091 expand-then-drop | **Keep** | Becomes LAW-150-3 phases E/D/C. |
| `EDGEQUAKE_ALLOW_BOOT_MIGRATE` | **Stay dead** | Warn-only shim at `mod.rs:957-976`. Do not revive. |

## ASCII: lock classes vs phase

```text
  statement                    lock taken              phase allowed
  ----------------------------- ---------------------- --------------
  CREATE TABLE / ADD nullable   brief ACCESS EXCL.     E
  CREATE INDEX (txn)            SHARE (blocks writes)  E only if small
  CREATE INDEX CONCURRENTLY     SHARE UPDATE EXCL.     E as no_tx job
  INSERT..SELECT batch COMMIT   ROW EXCL.              D
  VALIDATE CONSTRAINT           SHARE UPDATE EXCL.*    D or C
  DROP TABLE / ALTER TYPE       ACCESS EXCL.           C gated
  HNSW CREATE INDEX             SHARE + CPU/IO         D/C, one at a time

  * VALIDATE still scans; do not hold an earlier ACCESS EXCL. in the same txn.
```
