# 02 — Cross-Ref Matrix (SPEC-146)

Maps laws ↔ findings ↔ code ↔ tests ↔ lenses. Updated after honest assessment (2026-09-05).

## Law → Finding → Wave

| Law | Findings | Wave |
|-----|----------|------|
| LAW-146-1 Workspace walls | F-146-23,24 | M1a |
| LAW-146-2 Cedar (classified) | F-146-28,37 | M0–M1a |
| LAW-146-3 RBAC vs ABAC | F-146-15,18 | M0–M5 |
| LAW-146-4 One AuthzContext | F-146-13,21 | M1a–M4 |
| LAW-146-5 document_filter ∩ | F-146-03 | M2 |
| LAW-146-6 Existence-hiding | F-146-19,20 | M1a/M4 |
| LAW-146-7 Empty honesty | F-146-22,30,34 | M1a–M4 |
| LAW-146-8 Upload labels | F-146-01,02,26,36 | M1a/M1b |
| LAW-146-9 ANN pre-filter | F-146-04,05,27 | M2 |
| LAW-146-10 Provenance hops | F-146-06,07 | M3 |
| LAW-146-11 Community/popular | F-146-08,09 | M3 |
| LAW-146-12 Caches | F-146-10,11,12 | M4 |
| LAW-146-13 Key principals | F-146-17,18,25 | M1a–M5 |
| LAW-146-14 Legacy backfill | F-146-01 | M1a |
| LAW-146-15 Role unify | F-146-16 | M0 |
| LAW-146-16 Feature flag | — | M0 |
| LAW-146-17 KV list PEP | F-146-19,33,34 | M1a |
| LAW-146-18 Single AllowSet path | F-146-37 | M1a |
| LAW-146-19 Tagged PrincipalId | F-146-35,18,25 | M1a |
| LAW-146-20 ABAC requires auth | F-146-24; EC-146-35 | M0 |
| LAW-146-21 ANN over-fetch + allow-set cap | F-146-38; EC-146-36,37 | M2 |
| LAW-146-22 Monotonic policy_generation | F-146-39; EC-146-38 | M1a |
| LAW-146-23 Deny observability | F-146-40; EC-146-39 | M1a |
| LAW-146-24 Allow-set then ∩ filter | F-146-03; EC-146-03 | M2 |
| LAW-146-25 Break-glass TTL | F-146-18; EC-146-40 | M5 |

## Finding → Gate → Lens

| Finding | Gate | Primary lens |
|---------|------|--------------|
| F-146-01 Schema gap | G-146-12 | Database |
| F-146-02 Upload UI | Playwright upload | Document Manager / Front |
| F-146-03 document_filter | G-146-21 | API / Fullstack |
| F-146-04/05 Typed ANN | G-146-20 | Embedding/Graph |
| F-146-06/07 Hops/hubs | G-146-30,31 | Embedding/Graph / AI |
| F-146-08/09 Community/popular | G-146-32,33 | Embedding/Graph |
| F-146-10..12 Caches/MCP | G-146-40,41 | Security / API |
| F-146-14 No members API | G-146-13 | API / UX |
| F-146-15 Permission unused | G-146-14 | Fullstack / Security |
| F-146-16 Role drift | G-146-01 | PO / Front |
| F-146-18 Master key | G-146-50 | Security |
| F-146-19/33 KV list | G-146-11 | Fullstack / Database |
| F-146-20 Parse IDOR | G-146-44 | API / Security |
| F-146-21 Graph REST | G-146-33 | API / Graph |
| F-146-22 Citations | G-146-42 | AI / UX |
| F-146-25 Worker principal | G-146-53 | Security |
| F-146-26 Quarantine | G-146-12 | Doc Manager / UX |
| F-146-28 Policy store | G-146-00 | Security / DB |
| F-146-32 Acc UNCONFIRMED | measure M5 | PO / AI |
| F-146-34 status_counts | G-146-16 | UX / API |
| F-146-35 PrincipalId | M1a schema | Database / Security |
| F-146-36 Dual-write labels | G-146-12 | Fullstack / Doc Manager |
| F-146-37 Dual evaluator forbid | LAW-146-18 | Security / Fullstack |
| F-146-38 Allow-set / ANN scale | G-146-17,18 | Embedding/Graph / Database |
| F-146-39 policy_generation | G-146-55 | Security / Database |
| F-146-40 Deny observability | G-146-19 | Security |
| F-146-41 Cedar list cost | allow-set cache | Security / Fullstack |

## Code anchors → Law

| Code surface | Laws |
|--------------|------|
| `edgequake-auth/src/rbac.rs` | 3,15 |
| `edgequake-api/src/middleware.rs` | 1,4,20 |
| `document_filter_resolver.rs` | 5 |
| `documents/query/list.rs` (KV scan) | 6,7,17 |
| `document_admission.rs` / dropzone | 8 |
| `typed_read.rs` / `fleet_embedding_index.rs` | 9 |
| `graph_expand.rs` / `graph_hops.rs` | 10 |
| `community_global.rs` / `popular.rs` | 11 |
| SPEC-103 caches / `retrieval_id_cache.rs` | 12 |
| `auth_validation.rs` (master key string) | 13,19 |
| migrations 009/096 | 1 (backstop only) |
| `user-management-card.tsx` | 15 |
| New `edgequake-authz` | 2,4,18,21,22,24 |
| `edgequake-audit` | 13,23,25 (break-glass + deny) |

## EC → Gate (summary)

See full table in [09-edge-cases.md](09-edge-cases.md). Critical path:

```ascii
  EC-01,02,12,15,32,33,34,39 ──► M1a
  EC-22,23,30             ──► M1b
  EC-03,04,13,36,37       ──► M2
  EC-05,06,07,24          ──► M3
  EC-08,09,20,25,26,28,38 ──► M4
  EC-10,11,27,29,40       ──► M5
  EC-35                   ──► M0
```

## Spec inherits

| Spec | Relationship |
|------|--------------|
| [SPEC-027](../027-api-edgequake-audit/) | Auth identity SSOT |
| [SPEC-101](../101-wizard-mode-tenant-workspace/) | Workspace onboarding |
| [SPEC-091](../091-simplify-data-layer/) | Typed fleet ANN |
| [SPEC-098](../098-data-access-hardening/) | RLS / lifecycle / cascade |
| [SPEC-103](../103-llm-cache/) | Cache key extension |
| [SPEC-142](../142-precise-links-on-query/) | Citations authorized-only |
| [SPEC-032](../032-graph/) | Graph hub/occurrence context |
| SPEC-084 / GH-319 | **Override** status_counts when ABAC on |
| [zz-raw.md](zz-raw.md) | Ideas Lab source |

## Document ↔ lens

| Doc | Lenses |
|-----|--------|
| 00-why / 00-first-principles | PO, Security |
| 03-code-as-is | Fullstack |
| 04-target-architecture | Fullstack, Security, API |
| 05-data-model | Database, Embedding/Graph |
| 06-ux-ui-spec | UX, Front, Doc Manager |
| 07-implementation-plan | Fullstack |
| 08 / 09 / 10 | All (gates) |
| 11-threat-model | Security |
| 12-catalog | PO, API, Security |
| 13-honest-assessment | All |
| 14-roadblocks | Fullstack, Database, Security |
| 15-design-review | All (first principles + AI eng) |

## Cross-refs

- Findings → [01-finding-register.md](01-finding-register.md)  
- Laws → [00-first-principles.md](00-first-principles.md)  
- Lenses → [05-lenses/README.md](05-lenses/README.md)  
- Honest → [13-honest-assessment.md](13-honest-assessment.md)  
- Roadblocks → [14-roadblocks.md](14-roadblocks.md)  
- Design review → [15-design-review.md](15-design-review.md)  
