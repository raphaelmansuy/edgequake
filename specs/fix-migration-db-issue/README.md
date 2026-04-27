# Fix: Migration 019 Checksum Mismatch (v0.10.1 → v0.10.12 Upgrade)

> **Status:** FIXED in commit after `6f3d0204`  
> **Severity:** CRITICAL — prevents server startup  
> **Affected versions:** Any instance that ran v0.10.1 and upgrades to v0.10.2–v0.10.12  

---

## Documents in this spec

| File | Purpose |
|------|---------|
| `README.md` (this file) | Index, executive summary, fix instructions |
| `root_cause_analysis.md` | First-principles analysis, timeline, checksums |
| `reproduction_guide.md` | Step-by-step reproduction + proof |
| `prevention_playbook.md` | How to prevent this class of bug forever |
| `sqlx_checksum_proof.py` | Runnable proof script (5 checks, all pass) |
| `reproduce_and_verify.sh` | Bash script for Docker-based end-to-end test |

---

## Executive Summary

```
  v0.10.1 install             v0.10.12 deploy
  +--------------+            +--------------------+
  | Fresh DB     |            | Same DB            |
  | Run mig 019  |            | mig 019 FILE       |
  | Store hash A |  ------->  | has been CHANGED   |
  |              |  upgrade   | Hash B != Hash A   |
  |              |            | STARTUP FAILS      |
  +--------------+            +--------------------+

  Error: "migration 19 was previously applied but has been modified"
```

**Root cause:** Commit `6f3d0204` ("fix: harden 0.10.6 release") modified
`019_add_tenant_workspace_to_tasks.sql` after it had already been deployed
in v0.10.1. SQLx detects the SHA-384 mismatch at startup and aborts.

**Fix:** Restored migration 019 to its byte-for-byte v0.10.1 content.
The `ALTER TABLE … SET DEFAULT` lines that were added belong in
migration 035 (where they already exist correctly).

---

## Quick Fix Instructions

### For users already running v0.10.12 on a db that was migrated from v0.10.1:

The fix is included in the next release. Upgrading will resolve the issue.

### Emergency workaround (before patch release):

```sql
-- Option 1: Update the stored checksum to match the CURRENT file
-- Run this on your PostgreSQL database:
UPDATE public._sqlx_migrations
SET checksum = decode(
  '7b544306c5da16b05ec0607aa81b356055e82f4247705f98b5b036c26ab2cf1c5910d2a062309c6fedbd44e0eb54437a',
  'hex'
)
WHERE version = 19;
```

> **Note:** This workaround accepts the broken checksum. The proper fix
> (restoring the file) is preferred. See [root_cause_analysis.md](root_cause_analysis.md).

---

## Cross-references

- Root cause analysis: [root_cause_analysis.md](root_cause_analysis.md)
- Exact reproduction steps: [reproduction_guide.md](reproduction_guide.md)  
- Prevention policies: [prevention_playbook.md](prevention_playbook.md)
- Runnable proof: `python3 sqlx_checksum_proof.py` (from repo root)
