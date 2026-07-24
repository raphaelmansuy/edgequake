# SPEC-086 — Cross-Reference Matrix

> Finding ↔ lenses ↔ laws ↔ wave ↔ verification IDs

| Finding ID | Study | Primary lenses | Laws | Wave | Verify IDs |
|------------|-------|----------------|------|------|------------|
| `ux086_dual_progress_ui` | [F-dual-progress-ui](findings/F-dual-progress-ui.md) | FE Designer, UI, UX | 22,25 | 2 | `ux086_v_one_presenter`, `ux086_e_md_live_stage`, `ux086_e_pdf_parity`, `ux086_e_skip_converting`, `ux086_e_reprocess_md`, `ux086_e_batch_mixed` |
| `ux086_store_beats_poll` | [F-store-beats-poll](findings/F-store-beats-poll.md) | Full Stack, FE Designer | 23 | 2 | `ux086_v_merge_rule`, `ux086_e_ws_gap`, `ux086_e_admit_404` |
| `ux086_staging_list` | [F-staging-list-visibility](findings/F-staging-list-visibility.md) | Full Stack, Product Owner | 24 | 1 | `ux086_v_staging_list`, `ux086_e_refresh_mid`, `ux086_e_staging_promote`, `ux086_e_fairness_queue` |
| `ux086_sparse_md_events` | [F-sparse-md-events](findings/F-sparse-md-events.md) | Full Stack, O(n) | 26 | 1 | `ux086_v_stage_ws`, `ux086_e_small_md` |
| `ux086_source_type` | [F-source-type-taxonomy](findings/F-source-type-taxonomy.md) | Full Stack, AI Engineer | 25 | 3 | `ux086_v_source_markdown`, `ux086_e_skip_converting` |
| `ux086_extract_quality` | [F-extraction-quality-parity](findings/F-extraction-quality-parity.md) | AI Engineer, Product Owner | 27 | 3 | `ux086_v_density_gate`, `ux086_e_batch_mixed` |

### Cross-cutting e2e (LAW-28 / inherit)

| Verify ID | Inherits | Notes |
|-----------|----------|-------|
| `ux086_e_cancel_md` | SPEC-057 | Cancel → Stopping → Cancelled; no Completed flash |
| `ux086_e_cancel_stopping_md` | SPEC-057 / ops | Explicit Stopping → Cancelled on ActiveRuns |
| `ux086_e_fairness_queue` | SPEC-084 / 057 | Also linked on `ux086_staging_list` (visibility of queued staging) |
| `ux086_e_reprocess_md` | SPEC-050 | Also linked on `ux086_dual_progress_ui` |

### Ops reliability (F-ingest-ops-reliability)

| Finding ID | Verify IDs |
|------------|------------|
| `ux086_false_orphan_pending` | `ux086_e_queued_behind_busy`, `ux086_v_orphan_queue_coverage` |
| `ux086_replace_delete_race` | `ux086_e_replace_waits_delete` |
| `ux086_reingest_fail_closed` | `ux086_v_reingest_fail_closed` |
| `ux086_md_converting_label` | `ux086_e_md_no_converting_pdf`, `ux086_e_skip_converting`, `ux086_v_md_hide_converting` |

---

## Lens coverage

| Lens | File | Owns / challenges |
|------|------|-------------------|
| Product Owner | [LENS-product-owner.md](lenses/LENS-product-owner.md) | JTBD, metrics, anti-goals |
| Full Stack | [LENS-fullstack.md](lenses/LENS-fullstack.md) | End-to-end identity + loaders |
| AI Engineer | [LENS-ai-engineer.md](lenses/LENS-ai-engineer.md) | Chunk/extract quality |
| O(n) Expert | [LENS-on-expert.md](lenses/LENS-on-expert.md) | Poll/WS/scan cost |
| Front End Designer | [LENS-frontend-designer.md](lenses/LENS-frontend-designer.md) | Component composition |
| UX | [LENS-ux.md](lenses/LENS-ux.md) | State machine copy |
| UI Designer | [LENS-ui-designer.md](lenses/LENS-ui-designer.md) | Visual conflict fixes |

---

## Explicit non-dependencies

| Claim | Reality |
|-------|---------|
| MD needs PDF page SSE for parity | **False** — LAW-26 stage WS + poll suffice |
| Staging must be removed for list visibility | **False** — staging-aware load (or provisional shell) is enough |
| Entity count MD == PDF means quality | **False** — LAW-27 density/structure |
| Cancel semantics need redesign | **False** — LAW-28 inherit 057 |
| 068 already closed format-agnostic UX | **False** — identity FIXED; presenter + list visibility OPEN |

---

## Dependency graph

```
  admit text/MD --> staging metadata --+
                                       +--> progress loader (068 FIXED)
                                       +--> list/track/activity (OPEN: ux086_staging_list)
  admit PDF -----> final queued shell --> list visible
  Insert worker --> status writes --> progress facade --> poll/WS
  FE PdfUploadProgress <--- PDF channels (rich)
  FE IngestionProgressPanel <--- store seed (sticky)  --> ux086_store_beats_poll
  FE dual fork ---------------------------------------> ux086_dual_progress_ui
```
