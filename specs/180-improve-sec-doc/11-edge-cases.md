# 11 — Edge cases and mitigations

[Index](README.md) · Norms: [Policy](04-policy-and-capabilities.md) / [API](07-api-contract.md) · Proof: [Tests](12-e2e-test-matrix.md) · Scheduling: [Plan](13-implementation-plan.md)

This catalog covers envisioned boundaries, not a claim that no future edge case exists. Each case has a mitigation and a concrete acceptance test. Add a case/test pair whenever implementation discovers a new branch. Disclosure, authorization bypass, lost updates and partial propagation are release blockers.

## EC-180-01

**Upload intent before network admission.** Files are dropped or selected before access is chosen. Keep files local until explicit Upload; show the frozen effective label per file.

Requirement: [R-180-01](03-product-spec.md#r-180-01) · Test: [T-180-01](12-e2e-test-matrix.md#t-180-01) · Work: WP-03/04 in [Plan](13-implementation-plan.md).

## EC-180-02

**Dismissal is not a commit.** Escape, outside click, Cancel, or route navigation occurs with a dirty editor. Inner Escape closes picker; outer dismissal offers discard; no PATCH from close/unmount.

Requirement: [R-180-02](03-product-spec.md#r-180-02) · Test: [T-180-02](12-e2e-test-matrix.md#t-180-02) · Work: WP-05 in [Plan](13-implementation-plan.md).

## EC-180-03

**Privacy-safe audience explanation.** A user tries another member, hidden document, or profile identity in preview. Default to policy explanation; steward-only scoped member checks; hidden IDs return ordinary 404.

Requirement: [R-180-03](03-product-spec.md#r-180-03) · Test: [T-180-03](12-e2e-test-matrix.md#t-180-03) · Work: WP-02 in [Plan](13-implementation-plan.md).

## EC-180-04

**AND semantics and scoped Public.** A user assumes multiple tags mean OR or Public means anonymous. Explain every-tag requirement and workspace scope; test users with disjoint and combined grants.

Requirement: [R-180-04](03-product-spec.md#r-180-04) · Test: [T-180-04](12-e2e-test-matrix.md#t-180-04) · Work: WP-02/07 in [Plan](13-implementation-plan.md).

## EC-180-05

**RBAC and clearance capability intersection.** A writer has clearance, an admin lacks it, or a steward is readonly. Derive operation and label capabilities separately; exact target recheck on commit.

Requirement: [R-180-05](03-product-spec.md#r-180-05) · Test: [T-180-05](12-e2e-test-matrix.md#t-180-05) · Work: WP-02 in [Plan](13-implementation-plan.md).

## EC-180-06

**Catalog loading, empty and unavailable.** Search is empty, fails, is paginated, or selected tags are not on the current page. Separate statuses; preserve selection; expose only assignable governed tags.

Requirement: [R-180-06](03-product-spec.md#r-180-06) · Test: [T-180-06](12-e2e-test-matrix.md#t-180-06) · Work: WP-02/04 in [Plan](13-implementation-plan.md).

## EC-180-07

**Initial exposure window.** A worker or read races the upload shell/original creation. Write validated access with protected references before commit and enqueue; no follow-up PATCH.

Requirement: [R-180-07](03-product-spec.md#r-180-07) · Test: [T-180-07](12-e2e-test-matrix.md#t-180-07) · Work: WP-03 in [Plan](13-implementation-plan.md).

## EC-180-08

**Transport and metadata equivalence.** PDF/image/text/Markdown/JSON/batch/MCP use different label encodings. One canonical envelope and reserved security metadata validation.

Requirement: [R-180-08](03-product-spec.md#r-180-08) · Test: [T-180-08](12-e2e-test-matrix.md#t-180-08) · Work: WP-03 in [Plan](13-implementation-plan.md).

## EC-180-09

**Stale edit and policy epoch.** Two tabs edit the same document or another security mutation invalidates epoch. Require versions and epoch; lock then authorize current state; compare before retry.

Requirement: [R-180-09](03-product-spec.md#r-180-09) · Test: [T-180-09](12-e2e-test-matrix.md#t-180-09) · Work: WP-01/02 in [Plan](13-implementation-plan.md).

## EC-180-10

**Mixed broadening and review.** Rank rises but one tag disappears, or an uploader reviews their own unverified file. Treat any tag removal and rank drop as downgrade; require steward and reason.

Requirement: [R-180-10](03-product-spec.md#r-180-10) · Test: [T-180-10](12-e2e-test-matrix.md#t-180-10) · Work: WP-02/05 in [Plan](13-implementation-plan.md).

## EC-180-11

**Audit and reason privacy.** Audit storage fails, reason text is sensitive, or retry repeats a committed save. Audit inside transaction; receipt dedupe; restricted audit reader and content-free analytics.

Requirement: [R-180-11](03-product-spec.md#r-180-11) · Test: [T-180-11](12-e2e-test-matrix.md#t-180-11) · Work: WP-01/02 in [Plan](13-implementation-plan.md).

## EC-180-12

**Child propagation and rollback.** A failure occurs halfway through copying labels to derived rows. All protected children update atomically or none do; verify reference joins too.

Requirement: [R-180-12](03-product-spec.md#r-180-12) · Test: [T-180-12](12-e2e-test-matrix.md#t-180-12) · Work: WP-01/07 in [Plan](13-implementation-plan.md).

## EC-180-13

**Off, null and unknown policy.** Enforcement is off, policy fetch fails, or a legacy row has no rank. Truthful state copy, no pretend protection, fail closed on unavailable decisions.

Requirement: [R-180-13](03-product-spec.md#r-180-13) · Test: [T-180-13](12-e2e-test-matrix.md#t-180-13) · Work: WP-02/05 in [Plan](13-implementation-plan.md).

## EC-180-14

**Filtered counts and pagination.** A sensitivity/tag filter matches rows beyond the loaded page. Filter under clearance before count and pagination; same predicate everywhere.

Requirement: [R-180-14](03-product-spec.md#r-180-14) · Test: [T-180-14](12-e2e-test-matrix.md#t-180-14) · Work: WP-06 in [Plan](13-implementation-plan.md).

## EC-180-15

**Bulk mixed state and atomic failure.** A 100-document selection contains unchanged, blocked, deleted or stale members. Explicit operation; preview diffs; all-or-nothing; no Mixed wire value.

Requirement: [R-180-15](03-product-spec.md#r-180-15) · Test: [T-180-15](12-e2e-test-matrix.md#t-180-15) · Work: WP-06 in [Plan](13-implementation-plan.md).

## EC-180-16

**Foreign or invalid vocabulary reference.** Tag is foreign-workspace, retired, classification-kind, unknown or policy-only referenced. Scope-aware validation/constraints; block destructive vocabulary deletion.

Requirement: [R-180-16](03-product-spec.md#r-180-16) · Test: [T-180-16](12-e2e-test-matrix.md#t-180-16) · Work: WP-01/02 in [Plan](13-implementation-plan.md).

## EC-180-17

**Scope changes and dedup.** Workspace/principal switches, or hidden bytes collide during upload. Clear in-memory scope/drafts, reauthorize dedup; never reuse/replace a hidden document.

Requirement: [R-180-17](03-product-spec.md#r-180-17) · Test: [T-180-17](12-e2e-test-matrix.md#t-180-17) · Work: WP-03/05/07 in [Plan](13-implementation-plan.md).

## EC-180-18

**List/detail/reload consistency.** List and detail use different formatters or background refresh overwrites a draft. One saved read model; version-aware synchronization; never label draft as saved.

Requirement: [R-180-18](03-product-spec.md#r-180-18) · Test: [T-180-18](12-e2e-test-matrix.md#t-180-18) · Work: WP-05 in [Plan](13-implementation-plan.md).

## EC-180-19

**Accessibility and responsive state.** Keyboard, screen reader, zoom, forced-colors or small viewport changes operation. Labelled checkbox controls/dialogs, focus management, reflow and non-color meaning.

Requirement: [R-180-19](03-product-spec.md#r-180-19) · Test: [T-180-19](12-e2e-test-matrix.md#t-180-19) · Work: WP-04/05/07 in [Plan](13-implementation-plan.md).

## EC-180-20

**High-cardinality performance.** 10k vocabulary or large descendants make picker/save unusable. Bounded keyset search, measured indexes, no N+1 and no weakened enforcement.

Requirement: [R-180-20](03-product-spec.md#r-180-20) · Test: [T-180-20](12-e2e-test-matrix.md#t-180-20) · Work: WP-01/07 in [Plan](13-implementation-plan.md).

## EC-180-21

**Network errors and unknown commit.** Connection fails before commit, after commit, or while status is pending. Retain draft, distinguish uncertain outcome, reconcile idempotently.

Requirement: [R-180-21](03-product-spec.md#r-180-21) · Test: [T-180-21](12-e2e-test-matrix.md#t-180-21) · Work: WP-02/05 in [Plan](13-implementation-plan.md).

## EC-180-22

**Old clients and upgrade ordering.** Legacy upload omits access or PATCH lacks version after cutoff. Keep omission default; publish deprecation; enforce 428 at final release.

Requirement: [R-180-22](03-product-spec.md#r-180-22) · Test: [T-180-22](12-e2e-test-matrix.md#t-180-22) · Work: WP-02/08 in [Plan](13-implementation-plan.md).

## EC-180-23

**Trust metrics without data leakage.** Analytics collects tag names or a conversion experiment weakens access. Allowlist telemetry fields and keep security decisions invariant across experiments.

Requirement: [R-180-23](03-product-spec.md#r-180-23) · Test: [T-180-23](12-e2e-test-matrix.md#t-180-23) · Work: WP-08 in [Plan](13-implementation-plan.md).

## EC-180-24

**False green security gate.** DB tests skip or only mocked browser tests run. Dedicated live job fails when DB/role/seed unavailable; mock evidence labelled separately.

Requirement: [R-180-24](03-product-spec.md#r-180-24) · Test: [T-180-24](12-e2e-test-matrix.md#t-180-24) · Work: WP-07 in [Plan](13-implementation-plan.md).

## EC-180-25

**Batch defaults and retry snapshots.** Default changes after a file override or one upload already started. Freeze per-file access, preserve overrides, retry failed items only.

Requirement: [R-180-01](03-product-spec.md#r-180-01) · Test: [T-180-25](12-e2e-test-matrix.md#t-180-25) · Work: WP-03/04 in [Plan](13-implementation-plan.md).

## EC-180-26

**Set normalization and duplicates.** IDs repeat, arrive out of order, or a no-op draft differs only in order. Sort/dedupe once; set equality; no spurious downgrade or version bump.

Requirement: [R-180-06](03-product-spec.md#r-180-06) · Test: [T-180-26](12-e2e-test-matrix.md#t-180-26) · Work: WP-01/02 in [Plan](13-implementation-plan.md).

## EC-180-27

**Lock ordering and concurrent bulk.** Two batches overlap in reverse order or profile updates race relabel. Workspace lock then sorted documents then tags then children; bounded retry.

Requirement: [R-180-09](03-product-spec.md#r-180-09) · Test: [T-180-27](12-e2e-test-matrix.md#t-180-27) · Work: WP-01/02 in [Plan](13-implementation-plan.md).

## EC-180-28

**Worker writes after relabel.** Extraction/retry inserts or upserts a child from old task metadata. Parent FOR SHARE derives latest label; relabel FOR UPDATE serializes; stale payload ignored.

Requirement: [R-180-12](03-product-spec.md#r-180-12) · Test: [T-180-28](12-e2e-test-matrix.md#t-180-28) · Work: WP-03/07 in [Plan](13-implementation-plan.md).

## EC-180-29

**Revocation during RAG and streams.** Cached answer, graph, citation, context handle or history retains restricted facts. Epoch fence before response/frame release; revalidate stored material and purge UI.

Requirement: [R-180-12](03-product-spec.md#r-180-12) · Test: [T-180-29](12-e2e-test-matrix.md#t-180-29) · Work: WP-07 in [Plan](13-implementation-plan.md).

## EC-180-30

**RLS role and pooled-session contamination.** Owner/BYPASSRLS tests pass or a pool leaks prior user's settings. Use real tenant app role and transaction-local context; no user-session reuse.

Requirement: [R-180-16](03-product-spec.md#r-180-16) · Test: [T-180-30](12-e2e-test-matrix.md#t-180-30) · Work: WP-01/07 in [Plan](13-implementation-plan.md).

## EC-180-31

**Long names, hostile strings and locales.** Names contain HTML/control chars, Unicode equivalence, duplicate display names or translation expansion. Normalize controlled vocabulary, escape output, disambiguate authorized values, support RTL/wrapping.

Requirement: [R-180-19](03-product-spec.md#r-180-19) · Test: [T-180-31](12-e2e-test-matrix.md#t-180-31) · Work: WP-02/04 in [Plan](13-implementation-plan.md).

## EC-180-32

**MCP/API key attribution.** A service key has write scope but no uploader identity or revoked session. Require scoped principal attribution and current clearance at finalization.

Requirement: [R-180-08](03-product-spec.md#r-180-08) · Test: [T-180-32](12-e2e-test-matrix.md#t-180-32) · Work: WP-03/07 in [Plan](13-implementation-plan.md).

## EC-180-33

**Break-glass and uploader exception.** Emergency access or denied steward compartments alter expected visibility. Show emergency state; preserve actual evaluator exceptions, audit bypass and do not infer universal deny precedence.

Requirement: [R-180-13](03-product-spec.md#r-180-13) · Test: [T-180-33](12-e2e-test-matrix.md#t-180-33) · Work: WP-02/07 in [Plan](13-implementation-plan.md).

## EC-180-34

**Duplicate reingestion changes identity.** A visible duplicate requests a new label or hidden duplicate triggers replacement. Reingestion keeps existing label; an access change requires explicit separate authorized transaction.

Requirement: [R-180-17](03-product-spec.md#r-180-17) · Test: [T-180-34](12-e2e-test-matrix.md#t-180-34) · Work: WP-03/07 in [Plan](13-implementation-plan.md).

## EC-180-35

**Migration and dual representation drift.** Join rows/array differ, old rows have nulls, or migration is repeated. Audit drift, backfill version without relabeling, validate constraints; preserve data on rollback.

Requirement: [R-180-16](03-product-spec.md#r-180-16) · Test: [T-180-35](12-e2e-test-matrix.md#t-180-35) · Work: WP-01 in [Plan](13-implementation-plan.md).

## EC-180-36

**Malformed payload and limits.** Null/partial/oversized access, repeated multipart fields, invalid rank or >32 tags arrives. Strict bounded parser and shared schema before side effects.

Requirement: [R-180-08](03-product-spec.md#r-180-08) · Test: [T-180-36](12-e2e-test-matrix.md#t-180-36) · Work: WP-02/03 in [Plan](13-implementation-plan.md).

## EC-180-37

**No-op and idempotency lifecycle.** Same key retries, receipt expires or actor loses document visibility after commit. Authorize replay, bind canonical request, do not duplicate audit; never echo hidden details.

Requirement: [R-180-21](03-product-spec.md#r-180-21) · Test: [T-180-37](12-e2e-test-matrix.md#t-180-37) · Work: WP-01/02 in [Plan](13-implementation-plan.md).

## EC-180-38

**Tag rename, retirement and deletion.** Selected tag renamed/retired while editor open or referenced only by rules. Identity remains UUID; increment epoch; block deletion; preserve existing retired requirement.

Requirement: [R-180-16](03-product-spec.md#r-180-16) · Test: [T-180-38](12-e2e-test-matrix.md#t-180-38) · Work: WP-01/02/05 in [Plan](13-implementation-plan.md).

## EC-180-39

**Oversized bulk and child count.** More than 100 documents or 500k descendants exceed safe synchronous bounds. Reject before mutation with bounded preflight; never partial async propagation.

Requirement: [R-180-15](03-product-spec.md#r-180-15) · Test: [T-180-39](12-e2e-test-matrix.md#t-180-39) · Work: WP-06/07 in [Plan](13-implementation-plan.md).

## EC-180-40

**Admission/outbox/object failure.** DB, object reference, receipt or outbox fails mid-admission; process restarts. Single transactional protected state, inaccessible staged bytes, TTL cleanup and idempotent outbox.

Requirement: [R-180-07](03-product-spec.md#r-180-07) · Test: [T-180-40](12-e2e-test-matrix.md#t-180-40) · Work: WP-03/07 in [Plan](13-implementation-plan.md).
