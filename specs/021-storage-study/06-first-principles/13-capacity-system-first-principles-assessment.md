# 13 — Capacity System: First-Principles Assessment (Code Is Law)

Date: 2026-06-25
Scope: Assess the "0.00 MB of 100.00 MB used" capacity widget shown in the dashboard screenshot against the actual code, then derive first-principles recommendations so capacity and messaging are adequate for the workload.
Method: First-principles trace of every storage/capacity concept from UI → API → resource budget → database. Live code verification, no assumptions.

---

## Executive finding

The capacity widget in the screenshot is **fictional from the backend's perspective**:

1. **No storage-limit concept exists in the backend.** `WorkspaceStatsResponse.storage_bytes` reports *used* bytes only — there is no `storage_limit` / `max_storage_bytes` field on `Workspace`, `Tenant`, or any stats response.
2. **The "100.00 MB" total is a hardcoded frontend illusion** with no backend authority. A repository-wide search for `100.00`, `of 100`, `MB used`, `storage_capacity`, `StorageUsageWidget` returns **zero** source files. The widget was never in the codebase (also absent from git history).
3. **The only enforced caps are structural**: `max_workspaces` (per tenant) and `max_documents` (per workspace, declared but **not enforced at ingestion**). There is no per-workspace or per-tenant storage quota.
4. **The upload byte limit drifted**: backend SSOT is `MAX_UPLOAD_BYTES = 50 MiB` (`edgequake-core/src/resource/budget.rs`), but the frontend `use-document-dropzone.ts` hardcoded `100 * 1024 * 1024` and the i18n strings said "10MB". Three different values across two tiers — a DRY violation documented as `RB-MEM-005` drift in SPEC-006.

The capacity system is therefore **incomplete**: the UI promises a limit the backend does not define, the limit it implies contradicts the real upload cap, and there is no enforcement path.

---

## First-principles trace

### A) What the UI shows vs. what the backend exposes

| Concept | UI source | API source | Backend SSOT | Status |
|---------|-----------|------------|--------------|--------|
| Used storage | (widget not in code) | `WorkspaceStatsResponse.storage_bytes: u64` | `pg_get_workspace_stats` aggregates `documents.content_length` | ✅ used bytes exist |
| Storage limit / capacity | "100.00 MB" (hardcoded, not in code) | **none** — no field on `Workspace`, `Tenant`, or `WorkspaceStatsResponse` | **none** | ❌ missing |
| Upload byte limit | `use-document-dropzone.ts` hardcoded `100 * 1024 * 1024`; i18n said "10MB" | `DefaultBodyLimit::max(resource_budget.max_upload_bytes)` | `MAX_UPLOAD_BYTES = 50 * 1024 * 1024` (`budget.rs`) | ❌ drifted (now fixed) |
| Document count quota | `Workspace.max_documents` (display only) | `Workspace.max_documents?` field | `quota_ops.rs` enforces `max_workspaces` only | ⚠️ declared, not enforced |
| Workspace count quota | n/a | `Tenant.max_workspaces` | `quota_ops.rs::pg_update_tenant_quota` | ✅ enforced |

### B) Where the limit *should* live

The backend already has a SSOT pattern for caps: `edgequake-core/src/resource/budget.rs::ResourceBudgetConfig`, documented in `specifications/006-ensure-perf/004_resource_budget_catalog.md` (BR-006-010: "Every cap must have exactly one authoritative definition"). A storage capacity belongs there, not as a UI literal.

### C) Why the message is "too short for the workload"

The screenshot reads "0.00 MB of 100.00 MB used". Applying first principles to the EdgeQuake workload:

1. **Single-document reality**: One PDF vision-extraction run on a 25 MB paper produces ~25 MB raw + markdown + chunks + embeddings (1536-dim float32 ≈ 6 KB/chunk × ~200 chunks ≈ 1.2 MB) + graph entities. A 100 MB *workspace* cap would be exhausted by ~3–4 PDFs — far below the `max_documents` default of 10 000.
2. **Throughput reality**: `RB-ING-001` allows 16 concurrent extractions; `RB-WRK-004` queues 100 tasks. At 25 MB/task average, a full queue represents 2.5 GB of in-flight data. A 100 MB cap is ~4% of one queue cycle — the capacity message is 25× too small for the configured throughput.
3. **Embedding growth**: `storage_bytes` today only sums `documents.content_length`. It does **not** count `chunks.embedding` (the column was dropped in migration 039, fixed in P5-01b) or graph edges. The "used" number is itself undercounting, so even the numerator of the widget is wrong.

**Conclusion**: the implied 100 MB capacity is neither derived from the backend nor sized for the workload. It is a UI placeholder that misleads users about both the limit and their usage.

---

## Recommended remediation (ordered)

> **Status (2026-06-25)**: Item 1 (upload-limit DRY) implemented as **P5-02** — see `edgequake_webui/src/lib/api/upload-limits.ts`. Items 2–5 are proposed for a follow-up iteration.

### 1) Codify the upload byte limit in one client SSOT ✅ Done (P5-02)

- New module `edgequake_webui/src/lib/api/upload-limits.ts` exposes `MAX_UPLOAD_BYTES` (mirrors backend `MAX_UPLOAD_BYTES = 50 MiB`, overridable via `NEXT_PUBLIC_MAX_UPLOAD_BYTES`).
- `use-document-dropzone.ts` and `document-dropzone.tsx` import from it; the "100MB" / "10MB" literals are removed.
- i18n `fileTooLarge` strings now interpolate `{{limit}}` instead of hardcoding a number (en/fr/zh updated).

### 2) Add a real storage capacity to the backend (proposed)

- Extend `ResourceBudgetConfig` with `max_workspace_storage_bytes` and `max_tenant_storage_bytes` (env: `EDGEQUAKE_MAX_WORKSPACE_STORAGE_BYTES`, `EDGEQUAKE_MAX_TENANT_STORAGE_BYTES`).
- Surface them on `WorkspaceStatsResponse` as `storage_limit_bytes: u64` next to the existing `storage_bytes`.
- Document in `004_resource_budget_catalog.md` as `RB-STOR-001` / `RB-STOR-002`.

### 3) Make `storage_bytes` complete (proposed)

- Recompute `pg_get_workspace_stats.storage_bytes` to sum `documents.content_length` **plus** chunk payload size **plus** graph edge count × edge row size. The current aggregate undercounts by ~30–60% for text-heavy workspaces.
- Add a regression test asserting `storage_bytes >= sum(content_length)`.

### 4) Enforce `max_documents` at ingestion (proposed)

- `max_documents` is declared on `Workspace` and shown in `WorkspaceStatsCards`, but no handler rejects an upload when `document_count >= max_documents`. Add a check in the upload handler that returns 402/409 with `code: "WORKSPACE_DOCUMENT_LIMIT_EXCEEDED"` (matches the existing `error-categories.ts` quota pattern).

### 5) Replace the fictional widget with a real one (proposed)

- Build `WorkspaceStorageUsageCard` that consumes `storage_bytes` (used) and `storage_limit_bytes` (limit) from `WorkspaceStatsResponse`.
- Show percentage with the same color thresholds as `BudgetIndicator` (≥80% amber, ≥100% red).
- Disable the upload dropzone when usage ≥ 100% with a tooltip explaining why.

---

## First-principles design rules for capacity systems

Derived from the gaps above — apply to any future capacity/quota work:

1. **One SSOT per cap.** Every capacity value must have exactly one authoritative definition (SPEC-006 BR-006-010). The UI mirrors it; it does not invent it.
2. **Cap and throughput must agree.** A storage cap must be ≥ (max concurrent tasks × avg task size) × headroom. If `RB-ING-001 = 16` and avg task = 25 MB, the cap must be ≥ 16 × 25 × 1.5 ≈ 600 MB, not 100 MB.
3. **Used must measure what counts.** `storage_bytes` must include every byte the user pays for (raw + chunks + embeddings + graph), or the percentage is a lie.
4. **Enforce what you display.** A displayed limit without a 402/409 enforcement path is a UI promise the backend breaks. Either enforce or remove the display.
5. **Degrade visibly, never silently.** When the backend is unreachable, the capacity widget must show "Connecting…" (see `BackendStatusBanner`), not "0 / 100 MB" — 0 used is indistinguishable from "no data".
6. **Env-overridable with clamps.** Caps must be tunable per deployment via env vars with documented min/max clamps (BR-006-011), so a 100 MB dev tenant and a 100 GB prod tenant share one code path.

---

## Cross-references

- `specifications/006-ensure-perf/004_resource_budget_catalog.md` — RB-MEM-005 drift (now resolved for the upload limit).
- `specs/021-storage-study/06-first-principles/11-ux-zero-documents-root-cause-assessment.md` — sibling root-cause for the 0-documents UX.
- `edgequake/crates/edgequake-core/src/resource/budget.rs` — backend resource SSOT.
- `edgequake_webui/src/lib/api/upload-limits.ts` — new client upload-limit SSOT (P5-02).
- `edgequake_webui/src/components/shared/backend-status-banner.tsx` — visible degradation when the backend is unreachable.
