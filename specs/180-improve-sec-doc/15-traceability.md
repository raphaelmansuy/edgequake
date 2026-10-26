# 15 — Bidirectional traceability

[Index](README.md) · [Laws](01-first-principles.md) · [Findings](02-current-implementation.md) · [Requirements](03-product-spec.md) · [Edge cases](11-edge-cases.md) · [Tests](12-e2e-test-matrix.md) · [Plan](13-implementation-plan.md)

R-180 IDs are specification requirements, not claims that global FEAT registry entries are implemented. Allocate existing repository feature annotations during implementation under the repository traceability rules. Every row below has a normative requirement, governing law, evidence locator, edge case, acceptance test and work package. Supplemental tests cover cross-cutting race and fault boundaries. Test status is planned unless specifically recorded in the assessment validation section.

## Requirement coverage

| Requirement | Governing law | Current evidence | Edge case / acceptance | Work |
|---|---|---|---|---|
| [R-180-01](03-product-spec.md#r-180-01) | [LAW-180-04](01-first-principles.md#law-180-04) | F-180-01 in [Assessment](02-current-implementation.md) | [EC-180-01](11-edge-cases.md#ec-180-01) / [T-180-01](12-e2e-test-matrix.md#t-180-01) | WP-03/04 in [Plan](13-implementation-plan.md) |
| [R-180-02](03-product-spec.md#r-180-02) | [LAW-180-05](01-first-principles.md#law-180-05) | F-180-02 in [Assessment](02-current-implementation.md) | [EC-180-02](11-edge-cases.md#ec-180-02) / [T-180-02](12-e2e-test-matrix.md#t-180-02) | WP-05 in [Plan](13-implementation-plan.md) |
| [R-180-03](03-product-spec.md#r-180-03) | [LAW-180-06](01-first-principles.md#law-180-06) | F-180-11 in [Assessment](02-current-implementation.md) | [EC-180-03](11-edge-cases.md#ec-180-03) / [T-180-03](12-e2e-test-matrix.md#t-180-03) | WP-02/05 in [Plan](13-implementation-plan.md) |
| [R-180-04](03-product-spec.md#r-180-04) | [LAW-180-02](01-first-principles.md#law-180-02) | F-180-04 in [Assessment](02-current-implementation.md) | [EC-180-04](11-edge-cases.md#ec-180-04) / [T-180-04](12-e2e-test-matrix.md#t-180-04) | WP-02/04 in [Plan](13-implementation-plan.md) |
| [R-180-05](03-product-spec.md#r-180-05) | [LAW-180-01](01-first-principles.md#law-180-01) | F-180-12 in [Assessment](02-current-implementation.md) | [EC-180-05](11-edge-cases.md#ec-180-05) / [T-180-05](12-e2e-test-matrix.md#t-180-05) | WP-02 in [Plan](13-implementation-plan.md) |
| [R-180-06](03-product-spec.md#r-180-06) | [LAW-180-03](01-first-principles.md#law-180-03) | F-180-03 in [Assessment](02-current-implementation.md) | [EC-180-06](11-edge-cases.md#ec-180-06) / [T-180-06](12-e2e-test-matrix.md#t-180-06) | WP-02/04 in [Plan](13-implementation-plan.md) |
| [R-180-07](03-product-spec.md#r-180-07) | [LAW-180-04](01-first-principles.md#law-180-04) | F-180-07 in [Assessment](02-current-implementation.md) | [EC-180-07](11-edge-cases.md#ec-180-07) / [T-180-07](12-e2e-test-matrix.md#t-180-07) | WP-01/03 in [Plan](13-implementation-plan.md) |
| [R-180-08](03-product-spec.md#r-180-08) | [LAW-180-03](01-first-principles.md#law-180-03) | F-180-01 in [Assessment](02-current-implementation.md) | [EC-180-08](11-edge-cases.md#ec-180-08) / [T-180-08](12-e2e-test-matrix.md#t-180-08) | WP-03 in [Plan](13-implementation-plan.md) |
| [R-180-09](03-product-spec.md#r-180-09) | [LAW-180-05](01-first-principles.md#law-180-05) | F-180-05 in [Assessment](02-current-implementation.md) | [EC-180-09](11-edge-cases.md#ec-180-09) / [T-180-09](12-e2e-test-matrix.md#t-180-09) | WP-01/02 in [Plan](13-implementation-plan.md) |
| [R-180-10](03-product-spec.md#r-180-10) | [LAW-180-05](01-first-principles.md#law-180-05) | F-180-05 in [Assessment](02-current-implementation.md) | [EC-180-10](11-edge-cases.md#ec-180-10) / [T-180-10](12-e2e-test-matrix.md#t-180-10) | WP-02/05 in [Plan](13-implementation-plan.md) |
| [R-180-11](03-product-spec.md#r-180-11) | [LAW-180-05](01-first-principles.md#law-180-05) | F-180-06 in [Assessment](02-current-implementation.md) | [EC-180-11](11-edge-cases.md#ec-180-11) / [T-180-11](12-e2e-test-matrix.md#t-180-11) | WP-01/02 in [Plan](13-implementation-plan.md) |
| [R-180-12](03-product-spec.md#r-180-12) | [LAW-180-07](01-first-principles.md#law-180-07) | F-180-06 in [Assessment](02-current-implementation.md) | [EC-180-12](11-edge-cases.md#ec-180-12) / [T-180-12](12-e2e-test-matrix.md#t-180-12) | WP-01/07 in [Plan](13-implementation-plan.md) |
| [R-180-13](03-product-spec.md#r-180-13) | [LAW-180-06](01-first-principles.md#law-180-06) | F-180-08 in [Assessment](02-current-implementation.md) | [EC-180-13](11-edge-cases.md#ec-180-13) / [T-180-13](12-e2e-test-matrix.md#t-180-13) | WP-02/05 in [Plan](13-implementation-plan.md) |
| [R-180-14](03-product-spec.md#r-180-14) | [LAW-180-01](01-first-principles.md#law-180-01) | F-180-09 in [Assessment](02-current-implementation.md) | [EC-180-14](11-edge-cases.md#ec-180-14) / [T-180-14](12-e2e-test-matrix.md#t-180-14) | WP-06 in [Plan](13-implementation-plan.md) |
| [R-180-15](03-product-spec.md#r-180-15) | [LAW-180-05](01-first-principles.md#law-180-05) | F-180-05 in [Assessment](02-current-implementation.md) | [EC-180-15](11-edge-cases.md#ec-180-15) / [T-180-15](12-e2e-test-matrix.md#t-180-15) | WP-06 in [Plan](13-implementation-plan.md) |
| [R-180-16](03-product-spec.md#r-180-16) | [LAW-180-02](01-first-principles.md#law-180-02) | F-180-13 in [Assessment](02-current-implementation.md) | [EC-180-16](11-edge-cases.md#ec-180-16) / [T-180-16](12-e2e-test-matrix.md#t-180-16) | WP-01/02 in [Plan](13-implementation-plan.md) |
| [R-180-17](03-product-spec.md#r-180-17) | [LAW-180-07](01-first-principles.md#law-180-07) | F-180-07 in [Assessment](02-current-implementation.md) | [EC-180-17](11-edge-cases.md#ec-180-17) / [T-180-17](12-e2e-test-matrix.md#t-180-17) | WP-03/05/07 in [Plan](13-implementation-plan.md) |
| [R-180-18](03-product-spec.md#r-180-18) | [LAW-180-03](01-first-principles.md#law-180-03) | F-180-10 in [Assessment](02-current-implementation.md) | [EC-180-18](11-edge-cases.md#ec-180-18) / [T-180-18](12-e2e-test-matrix.md#t-180-18) | WP-05 in [Plan](13-implementation-plan.md) |
| [R-180-19](03-product-spec.md#r-180-19) | [LAW-180-09](01-first-principles.md#law-180-09) | F-180-02 in [Assessment](02-current-implementation.md) | [EC-180-19](11-edge-cases.md#ec-180-19) / [T-180-19](12-e2e-test-matrix.md#t-180-19) | WP-04/05/07 in [Plan](13-implementation-plan.md) |
| [R-180-20](03-product-spec.md#r-180-20) | [LAW-180-10](01-first-principles.md#law-180-10) | F-180-09 in [Assessment](02-current-implementation.md) | [EC-180-20](11-edge-cases.md#ec-180-20) / [T-180-20](12-e2e-test-matrix.md#t-180-20) | WP-01/07 in [Plan](13-implementation-plan.md) |
| [R-180-21](03-product-spec.md#r-180-21) | [LAW-180-08](01-first-principles.md#law-180-08) | F-180-05 in [Assessment](02-current-implementation.md) | [EC-180-21](11-edge-cases.md#ec-180-21) / [T-180-21](12-e2e-test-matrix.md#t-180-21) | WP-02/05 in [Plan](13-implementation-plan.md) |
| [R-180-22](03-product-spec.md#r-180-22) | [LAW-180-08](01-first-principles.md#law-180-08) | F-180-01 in [Assessment](02-current-implementation.md) | [EC-180-22](11-edge-cases.md#ec-180-22) / [T-180-22](12-e2e-test-matrix.md#t-180-22) | WP-02/08 in [Plan](13-implementation-plan.md) |
| [R-180-23](03-product-spec.md#r-180-23) | [LAW-180-10](01-first-principles.md#law-180-10) | F-180-04 in [Assessment](02-current-implementation.md) | [EC-180-23](11-edge-cases.md#ec-180-23) / [T-180-23](12-e2e-test-matrix.md#t-180-23) | WP-08 in [Plan](13-implementation-plan.md) |
| [R-180-24](03-product-spec.md#r-180-24) | [LAW-180-10](01-first-principles.md#law-180-10) | F-180-14 in [Assessment](02-current-implementation.md) | [EC-180-24](11-edge-cases.md#ec-180-24) / [T-180-24](12-e2e-test-matrix.md#t-180-24) | WP-07 in [Plan](13-implementation-plan.md) |

## Supplemental cross-cutting coverage

| Requirement | Scenario | Edge / test | Work |
|---|---|---|---|
| [R-180-01](03-product-spec.md#r-180-01) | Batch defaults and retry snapshots | [EC-180-25](11-edge-cases.md#ec-180-25) / [T-180-25](12-e2e-test-matrix.md#t-180-25) | WP-03/04 in [Plan](13-implementation-plan.md) |
| [R-180-06](03-product-spec.md#r-180-06) | Set normalization and duplicates | [EC-180-26](11-edge-cases.md#ec-180-26) / [T-180-26](12-e2e-test-matrix.md#t-180-26) | WP-01/02 in [Plan](13-implementation-plan.md) |
| [R-180-09](03-product-spec.md#r-180-09) | Lock ordering and concurrent bulk | [EC-180-27](11-edge-cases.md#ec-180-27) / [T-180-27](12-e2e-test-matrix.md#t-180-27) | WP-01/02 in [Plan](13-implementation-plan.md) |
| [R-180-12](03-product-spec.md#r-180-12) | Worker writes after relabel | [EC-180-28](11-edge-cases.md#ec-180-28) / [T-180-28](12-e2e-test-matrix.md#t-180-28) | WP-03/07 in [Plan](13-implementation-plan.md) |
| [R-180-12](03-product-spec.md#r-180-12) | Revocation during RAG and streams | [EC-180-29](11-edge-cases.md#ec-180-29) / [T-180-29](12-e2e-test-matrix.md#t-180-29) | WP-07 in [Plan](13-implementation-plan.md) |
| [R-180-16](03-product-spec.md#r-180-16) | RLS role and pooled-session contamination | [EC-180-30](11-edge-cases.md#ec-180-30) / [T-180-30](12-e2e-test-matrix.md#t-180-30) | WP-01/07 in [Plan](13-implementation-plan.md) |
| [R-180-19](03-product-spec.md#r-180-19) | Long names, hostile strings and locales | [EC-180-31](11-edge-cases.md#ec-180-31) / [T-180-31](12-e2e-test-matrix.md#t-180-31) | WP-02/04 in [Plan](13-implementation-plan.md) |
| [R-180-08](03-product-spec.md#r-180-08) | MCP/API key attribution | [EC-180-32](11-edge-cases.md#ec-180-32) / [T-180-32](12-e2e-test-matrix.md#t-180-32) | WP-03/07 in [Plan](13-implementation-plan.md) |
| [R-180-13](03-product-spec.md#r-180-13) | Break-glass and uploader exception | [EC-180-33](11-edge-cases.md#ec-180-33) / [T-180-33](12-e2e-test-matrix.md#t-180-33) | WP-02/07 in [Plan](13-implementation-plan.md) |
| [R-180-17](03-product-spec.md#r-180-17) | Duplicate reingestion changes identity | [EC-180-34](11-edge-cases.md#ec-180-34) / [T-180-34](12-e2e-test-matrix.md#t-180-34) | WP-03/07 in [Plan](13-implementation-plan.md) |
| [R-180-16](03-product-spec.md#r-180-16) | Migration and dual representation drift | [EC-180-35](11-edge-cases.md#ec-180-35) / [T-180-35](12-e2e-test-matrix.md#t-180-35) | WP-01 in [Plan](13-implementation-plan.md) |
| [R-180-08](03-product-spec.md#r-180-08) | Malformed payload and limits | [EC-180-36](11-edge-cases.md#ec-180-36) / [T-180-36](12-e2e-test-matrix.md#t-180-36) | WP-02/03 in [Plan](13-implementation-plan.md) |
| [R-180-21](03-product-spec.md#r-180-21) | No-op and idempotency lifecycle | [EC-180-37](11-edge-cases.md#ec-180-37) / [T-180-37](12-e2e-test-matrix.md#t-180-37) | WP-01/02 in [Plan](13-implementation-plan.md) |
| [R-180-16](03-product-spec.md#r-180-16) | Tag rename, retirement and deletion | [EC-180-38](11-edge-cases.md#ec-180-38) / [T-180-38](12-e2e-test-matrix.md#t-180-38) | WP-01/02/05 in [Plan](13-implementation-plan.md) |
| [R-180-15](03-product-spec.md#r-180-15) | Oversized bulk and child count | [EC-180-39](11-edge-cases.md#ec-180-39) / [T-180-39](12-e2e-test-matrix.md#t-180-39) | WP-06/07 in [Plan](13-implementation-plan.md) |
| [R-180-07](03-product-spec.md#r-180-07) | Admission/outbox/object failure | [EC-180-40](11-edge-cases.md#ec-180-40) / [T-180-40](12-e2e-test-matrix.md#t-180-40) | WP-03/07 in [Plan](13-implementation-plan.md) |

## Lens ownership and review order

| Lens | Owns | Primary documents |
|---|---|---|
| [Product Owner](lenses/LENS-product-owner.md) | Jobs, priority, task completion, capability product decisions | [WHY](00-five-whys.md), [Product](03-product-spec.md), [Benchmark](14-research-and-benchmark.md) |
| [Full Stack Developer](lenses/LENS-full-stack.md) | Cross-transport contract and domain integration | [API](07-api-contract.md), [Frontend](09-frontend-architecture.md), [Sequences](10-security-and-sequences.md) |
| [Database Expert](lenses/LENS-database.md) | Integrity, concurrency, RLS and operational proof | [DB](08-data-and-db-contract.md), [Edge cases](11-edge-cases.md), [Tests](12-e2e-test-matrix.md) |
| [UX/UI Designer](lenses/LENS-ux-ui.md) | User model, information hierarchy, interaction and accessibility | [Policy](04-policy-and-capabilities.md), [UX/UI](05-ux-ui-spec.md), [Screens](06-screens-and-navigation.md) |
| [Front Designer](lenses/LENS-front-designer.md) | Tokens, components, state presentation and responsive craft | [UX/UI](05-ux-ui-spec.md), [Screens](06-screens-and-navigation.md), [Frontend](09-frontend-architecture.md) |
| [Marketing/Growth](lenses/LENS-marketing-growth.md) | Honest promise, activation, comparative evidence | [Product](03-product-spec.md), [Research](14-research-and-benchmark.md), [Rollout](13-implementation-plan.md#wp-08) |

Review laws and capability decisions first, API/DB contracts second, shared interaction third, tests/rollout last. A change to rank/tag semantics must update Policy, API, DB, Sequences, edge cases and relevant acceptance tests in the same PR. A purely visual change still updates UX/UI and accessibility/visual evidence where behavior or focus changes.

Run the pack validator to detect missing files/anchors, duplicate IDs, unpaired requirements/tests and malformed ASCII grids. Mermaid syntax validation uses the repository's installed Mermaid version separately. Neither documentation validation nor existing rule-format tests certify future security implementation.
