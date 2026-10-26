# SPEC-180 — Make document access understandable at upload and edit

**Start with [the five WHYs](00-five-whys.md).** Then read [first principles](01-first-principles.md), [the code assessment](02-current-implementation.md), and [the product contract](03-product-spec.md).

Status: implemented in the working tree, with live validation evidence in [the E2E runbook](e2e/README.md). Assessment completed 2026-10-26, Asia/Hong_Kong. Source baseline: `8d46d2ab4af5424c99c873404cad1c80948787b4` (working tree initially clean).

The outcome is one understandable document access workflow: select a sensitivity, apply governed security tags, see the resulting access rule, and commit deliberately. The same editor serves upload and document properties. Existing SPEC-174 clearance remains the policy engine. This is a focused product improvement, not a replacement authorization platform.

“Better than any product on the market” is the ambition. [The research and benchmark](14-research-and-benchmark.md) turn it into falsifiable quality gates; this specification makes no untested market leadership claim.

## Document map

| Document | Decision it owns |
|---|---|
| [00 Five WHYs](00-five-whys.md) | Root problem and hypotheses |
| [01 First principles](01-first-principles.md) | Non-negotiable laws |
| [02 Current implementation](02-current-implementation.md) | Evidence, gaps, reuse, scope |
| [03 Product specification](03-product-spec.md) | Requirements and success criteria |
| [04 Policy and capabilities](04-policy-and-capabilities.md) | RBAC × clearance behavior and terminology |
| [05 UX/UI specification](05-ux-ui-spec.md) | Visual tokens, interaction, accessibility, copy |
| [06 Screens and navigation](06-screens-and-navigation.md) | ASCII wireframes and screen states |
| [07 API contract](07-api-contract.md) | Proposed requests, responses, errors, compatibility |
| [08 Database contract](08-data-and-db-contract.md) | ER model, transactions, migration, indexing |
| [09 Frontend architecture](09-frontend-architecture.md) | Shared components, draft state, cache ownership |
| [10 Security and sequences](10-security-and-sequences.md) | Trust boundaries, sequence and state diagrams |
| [11 Edge cases](11-edge-cases.md) | Failure scenarios and mitigations |
| [12 Test matrix](12-e2e-test-matrix.md) | Unit, contract, database, live E2E, accessibility proof |
| [13 Implementation plan](13-implementation-plan.md) | Ordered work packages and release gates |
| [14 Research and benchmark](14-research-and-benchmark.md) | Primary research, reference design, comparative protocol |
| [15 Traceability](15-traceability.md) | Requirement → law → code → edge case → test → work |

## Six decision lenses

[Product Owner](lenses/LENS-product-owner.md) · [Full Stack Developer](lenses/LENS-full-stack.md) · [Database Expert](lenses/LENS-database.md) · [UX/UI Designer](lenses/LENS-ux-ui.md) · [Front Designer](lenses/LENS-front-designer.md) · [Marketing and Growth](lenses/LENS-marketing-growth.md).

## Relationship to existing specifications

- [SPEC-174](../174-document-security/README.md): reuse ranks, compartment subset semantics, steward rules, hidden-resource behavior, RLS and derived-content boundaries. Its findings describe a pre-implementation snapshot, not today's tree.
- [SPEC-175](../175-perfect-settings/README.md): keep workspace Access settings as the place to govern vocabulary and access profiles. Do not introduce a competing settings hub.
- [SPEC-173](../173-improve-mcp/README.md): every ingestion channel must honor admission labels; MCP parity is a release gate.
- [SPEC-168](../168-perfect-ui/README.md): preserve the existing shell, layout stability, light/dark behavior and accessible density.
- [SPEC-150 migration files](../../edgequake/migrations): use the repository's schema migration workflow; allocate a free migration number during implementation.

Normative terms: **MUST** is a release gate; **SHOULD** needs a documented exception; **MAY** is optional. IDs are scoped to SPEC-180. The assessment retains its original baseline. The API and database contracts describe the delivered design; the E2E runbook identifies executed tests and separates them from unmeasured acceptance targets.

Authoring checks and test limitations are in [Assessment validation](12-e2e-test-matrix.md#assessment-validation-distinct-from-future-acceptance). Run [scripts/validate.py](scripts/validate.py) to check this pack's links, IDs, traceability, examples and wireframe grids.
