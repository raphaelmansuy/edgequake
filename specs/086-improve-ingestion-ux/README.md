# SPEC-086 — Improve Ingestion UX (Format-Agnostic Feedback)

> **Product pin**: EdgeQuake v0.21.1  
> **Docs status**: Spec pack authored 2026-07-24  
> **Implementation**: Waves 1–4 landed 2026-07-24  
> **Symptom**: Markdown upload shows “Queued for processing…” / Done-ish chrome while PDF gets live stage + page feedback  
> **Inherits**: [SPEC-048](../048-improve-ux/) · [SPEC-050](../050-pipeline-and-delete/) · [SPEC-054/056](../054-fix-bugs-17/) · [SPEC-057](../057-pipeline-reliability/) · [068 text progress](../001-benchmark/001-edgquake-improvements/068-text-ingest-progress-parity.md) · [SPEC-084](../084-reliability-fix/) · [SPEC-085 laws](../085-fix-security/00-first-principles.md)

## Verification status (SSOT)

See [01-finding-register.md](01-finding-register.md): **19 FIXED / 0 PARTIAL / 2 OPEN**.

| Wave | Goal | Status |
|------|------|--------|
| **0** | Spec pack + contract pins + lenses | **done** |
| **1** | Backend visibility SSOT (staging in list/track/activity) + stage-transition WS | **done** |
| **2** | FE one presenter + poll/store merge | **done** |
| **3** | Source-type taxonomy + density golden-pair gate | **done** |
| **4** | Playwright e2e edge matrix green | **done** |
| **ops** | Upload/cancel/delete/replace reliability | **done** |

---

## Start here

1. [00-first-principles.md](00-first-principles.md) — LAW-22…LAW-28 + five whys  
2. [01-finding-register.md](01-finding-register.md) — every finding with status  
3. [02-cross-ref-matrix.md](02-cross-ref-matrix.md) — `ux086_*` IDs  
4. [03-implementation-roadmap.md](03-implementation-roadmap.md) — DRY/SOLID waves  
5. [04-verification-matrix.md](04-verification-matrix.md) — gates  
6. [05-surface-playbooks.md](05-surface-playbooks.md) — commands  
7. [06-contract-pins.md](06-contract-pins.md) — track_id / stages / visibility floors  
8. Lenses → [`lenses/`](lenses/README.md)  
9. Findings → [`findings/`](findings/README.md)  
10. E2E → [`e2e/README.md`](e2e/README.md)

---

## Locked decisions

1. **One progress presenter** for all formats: ActiveRuns-style `ServerStageStepper` + stage/overall bars. PDF page detail is an optional sub-panel under `converting`, not a second product.  
2. **One progress merge rule**: `max(store, poll)` by stage rank + terminal override; poll **writes** into Zustand (immutable updates). Never leave seed as SSOT.  
3. **In-flight list visibility**: documents list / track / pipeline activity use the same staging-aware metadata load as progress (068). Prefer extending `load_scoped_document_metadata_for_progress` (or shared helper) — do not invent a third loader.  
4. **Stage vocabulary**: keep `UnifiedStage`; non-PDF **skips** `converting` (shown as skipped, not grey “stuck”). Labels are format-aware.  
5. **SSE**: do not add PDF-style page SSE for MD; emit **stage-transition WS events** for all Insert tracks (OCP on existing bridge). Poll remains fallback.  
6. **Quality gate**: golden pair MD↔PDF compares density + recall proxies, not raw entity count equality.  
7. **Non-goals**: Temporal migration; changing cancel/fairness SSOT (057); rewriting SPEC-048 from scratch.

---

## Ops / docs SSOT (do not fork)

| Concern | Path |
|---------|------|
| Progress surfaces | [`docs/deep-dives/pipeline-progress.md`](../../docs/deep-dives/pipeline-progress.md) |
| Cancel / fairness | [`docs/ingestion-cancel-and-fairness.md`](../../docs/ingestion-cancel-and-fairness.md) |
| Observability | [`docs/OBSERVABILITY.md`](../../docs/OBSERVABILITY.md) |

---

## Surfaces (blast radius)

| Surface | Role |
|---------|------|
| `edgequake_webui` | Presenter, store merge, upload routing |
| `edgequake-api` | Admit, staging load, list/track, WS bridge |
| `edgequake-pipeline` / tasks | UnifiedStage, Insert status writes |
| Playwright e2e | Format-parity + edge matrix |

---

## E2E proof (2026-07-24)

| Check | Result |
|-------|--------|
| `contract_068_text_ingest_progress` | pass |
| `contract_086_ingestion_visibility` | pass |
| Vitest merge / 086 / upload taxonomy | pass |
| `e2e/spec068-text-ingest-progress.spec.ts` | pass |
| `e2e/spec086-ingestion-ux.spec.ts` (6 cases) | pass |
| `scripts/ingestion_density_gate.py` sample | PASS ratio≥0.25 |

---

## Success criteria

- Same stepper + stage/overall bars for PDF, MD, TXT, images (format-specific sub-detail only where real work exists).  
- No “green Done + Queued for processing…” conflict.  
- In-flight MD visible on documents list / ActiveRuns before promote.  
- Poll can advance UI without WS.  
- Spec cross-refs 100%: every finding ↔ law ↔ wave ↔ verify ID.  
- E2E matrix in [`e2e/README.md`](e2e/README.md) green after Waves 1–4.
