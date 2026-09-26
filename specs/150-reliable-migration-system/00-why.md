# 00 — WHY

Parent: [README](README.md) · Next: [01-first-principles](01-first-principles.md)

## The job to be done

An operator who deployed **any published EdgeQuake tag** must be able to:

1. Point a **current** migrator at that database.
2. Reach the **current** schema (HEAD numbered migrations, today M158 unreleased / M149 in v0.26.10).
3. Do it **reliably** (idempotent, resumable, fail-closed on unknown drift, honest on data copy).
4. Do it **fast enough** that a deploy is not an overnight outage.
5. Keep the **API process able to start** (liveness) even while schema is pending.

That is the whole spec. Everything else is a means.

## Why the current design fails the job

EdgeQuake already learned the expensive lesson that **the API must not apply irreversible schema at boot** (SPEC-091 LD-15, v0.23.0). The binary now exits **78** (`EX_CONFIG`) when the ledger is behind, with prefix `BOOT_GATE_REFUSAL:` ([`mod.rs:832-835`](../../edgequake/crates/edgequake-api/src/state/migration_bootstrap/mod.rs), [`main.rs:1144-1150`](../../edgequake/src/main.rs)).

That fix moved the write path to `edgequake migrate`. It did **not** finish the job:

| Operator experience | Physical cause |
|---------------------|----------------|
| New image crash-loops on Helm upgrade | Migrate Job is `post-install,post-upgrade` ([`migrate-job.yaml:10`](../../deploy/kubernetes/helm/edgequake/templates/migrate-job.yaml)). Helm runs post-hooks **after** resources exist; with `--wait`, post-hooks wait until those resources are Ready ([Helm chart hooks](https://helm.sh/docs/topics/charts_hooks/)). Pods exit 78 before they can become Ready. |
| Compose `up` never reaches a serving API | No migrate service. Fresh / behind DB => exit 78 => `restart: unless-stopped` loops. |
| `migration N was previously applied but has been modified` | Eight shipped files were edited after a release tag. Repair exists for 071/078/118/121/125/131 **only if** `EDGEQUAKE_ALLOW_CHECKSUM_REPAIR` or `EDGEQUAKE_DEV_MODE` is set ([`checksum_repair.rs:24,52-59`](../../edgequake/crates/edgequake-api/src/state/migration_bootstrap/checksum_repair.rs)). **001 (v0.11.0) and 019 (v0.10.6–v0.10.12) have no repair.** |
| API is "verify-only" but still runs heavy SQL | `tokio::spawn` of m040/m139/m140/m141 calls `sqlx::raw_sql` on every serving boot until a progress key exists ([`mod.rs:1487-1515`](../../edgequake/crates/edgequake-api/src/state/migration_bootstrap/mod.rs), [`m139.rs:35`](../../edgequake/crates/edgequake-api/src/state/migration_bootstrap/reconcile/m139.rs)). |
| Nothing listens during the gate | HTTP bind is after `AppState::new_postgres` succeeds. Liveness cannot pass. Kubernetes then kills / restarts the pod. |
| Upgrade "succeeds" with missing rows | Repeated data-copy incidents (#363 silent 99.7% drop, SPEC-139 W3 `expected 44580 actual 18503`, open #396). The ledger advanced; the data did not. |

## Why "migrations inside the program" is the load-bearing problem

sqlx's `migrate!()` macro embeds SQL in the **same artifact** that serves HTTP. That is convenient for a laptop. It is hostile to production:

```text
  desired:   [migrate Job] ----schema ready----> [API replicas]
  actual:    [API replicas] --exit 78--> crashloop
             [migrate Job]  --runs later, or never (Helm --wait deadlock)--
```

GitLab's public rule is the contrast class: *"Migrations are not allowed to require GitLab installations to be taken offline ever"* and schema migrations run **before** new application code ([Migration Style Guide](https://docs.gitlab.com/development/migration_style_guide/)). EdgeQuake inverted that in Helm: new code first, migrate after.

The binary **can** stay one artifact (subcommand `edgequake migrate`) **if and only if** orchestrators invoke that subcommand as a **separate lifecycle** (Job / compose one-shot / ECS task) and the serving entrypoint **never writes**. A second binary (`edgequake-migrate`) is optional packaging, not a first principle — see [06](06-target-architecture.md).

## Cost of the last two years

Twenty-nine incidents are catalogued in [02](02-incident-catalogue.md). Recurring pattern: **a patch that unblocks one fleet becomes the next fleet's checksum or cardinality failure.** Editing 001 to pin `search_path` caused #195. Repairing 078 checksums broke fresh PG16 (table missing). Unique index 143 made a race loud (#374 → #377 → #383).

CI did not catch them because fixtures were empty graphs, one workspace per document id, and `migration-guard` runs `pgvector/pgvector:pg16` **without AGE**.

## What "reliable and fast" means here

- **Reliable:** every epoch in [07](07-upgrade-path-matrix.md) has a proven path; unknown checksum drift fails closed with an actionable message; data-copy jobs do not advance a cursor past a 21000/23505 skip; kill-9 mid-run resumes.
- **Fast:** expand-phase DDL uses short lock windows (`lock_timeout` + retry, `NOT VALID` / `CONCURRENTLY` where required). Data phase is batched and does not hold a single sqlx transaction across a full AGE graph (M156/M158 today set `statement_timeout = 0` inside one transaction). Fresh-install budget in [10](10-performance-budget.md). Squash is a **measured** optimization, not a rewrite of history.

## Why start with assessment

Without the epoch table, a "squash baseline" would strand v0.11.0 and v0.10.6-0.10.12 databases. Without the incident catalogue, we would rebuild checksum-repair-as-env-var (SPEC-111) and call it done. The implementation plan in [08](08-implementation-plan.md) is sequenced so WP-2 (variant registry) unblocks ancient fleets **before** WP-10 (squash) is even allowed to be discussed.
