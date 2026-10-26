# 03 — Product specification

[Index](README.md) · Basis: [WHY](00-five-whys.md), [Evidence](02-current-implementation.md) · Detail: [Policy](04-policy-and-capabilities.md), [Screens](06-screens-and-navigation.md) · Verification: [Traceability](15-traceability.md)

## Jobs and scope

An uploader can label a file once, understand the access rule and start processing. A document editor can inspect saved access, change it deliberately and recover from a conflict. A security steward can review unverified documents and approve broadening with a reason. A reader can understand visible document access without receiving an edit control or a directory of hidden profiles.

P0 includes single and multi-file upload with shared defaults and per-file overrides, property editing, truthful summaries, capabilities, atomic admission/relabel, cache fencing and live security proof. P1 adds existing-document bulk editing, server security filters and expanded audit UI. Both priorities are specified; market benchmark claims require the complete release gate. Deferred features remain in [Assessment](02-current-implementation.md#scope-boundary).

## Requirements

### R-180-01

**P0 — Upload access.** Show a persistent “Access for this upload” summary before file selection, then a file review containing sensitivity and security tags before admission. Choosing or dropping files MUST NOT immediately admit them. “Upload N files” commits frozen per-file labels. See W-180-01/02 and T-180-01.

### R-180-02

**P0 — Explicit commit.** Property changes require Save. Cancel/Escape/outside dismissal never sends PATCH. Dirty outer-editor dismissal asks Discard/Keep editing; the inner picker Escape only closes that picker. See W-180-04, T-180-02.

### R-180-03

**P0 — Explain effective access.** Show a server-derived rule sentence and whether the caller can still read the result. A rule summary is always available on valid drafts; detailed “Check a member” is restricted to stewards. Never infer exact audience from a profile name. See [API](07-api-contract.md), T-180-03.

### R-180-04

**P0 — Honest names.** Use “Sensitivity,” “Security tags” and “Who can access.” Explain ALL-tag semantics and authenticated scope of Public. Distinguish review state from processing state. See [Policy](04-policy-and-capabilities.md), T-180-04.

### R-180-05

**P0 — Server-derived authority.** Separate create label, strengthen label, downgrade, manage vocabulary and inspect audience capabilities. Workspace writer role and sufficient clearance allow labeling/strengthening; downgrade additionally requires steward. Owner/admin alone grants vocabulary management, not automatic document visibility. See T-180-05 and [capability matrix](04-policy-and-capabilities.md#capability-matrix).

### R-180-06

**P0 — Governed picker.** Search only assignable existing tags, preserve stable UUIDs, expose loading/error/empty distinctly, and retain selected tags outside search results. Creating a tag requires vocabulary authority and deliberate navigation to Settings. No free-text security tags. See W-180-03, T-180-06.

### R-180-07

**P0 — Atomic admission.** Validate labels before side effects; establish shell/original/task label before any exposure. Under enforcement, omission means unverified with authenticated uploader. No temporary Public/Internal state. See [Sequence](10-security-and-sequences.md), T-180-07.

### R-180-08

**P0 — Transport parity.** PDF, image, text, Markdown, JSON, batch REST and MCP converge on one label admission contract. Generic metadata cannot override typed access. See T-180-08.

### R-180-09

**P0 — Concurrency.** Every edit supplies document label version and expected workspace policy epoch. Validate current state inside the transaction. Stale drafts receive a conflict requiring review. See T-180-09.

### R-180-10

**P0 — Safe broadening.** Lower rank, removal of any tag, or unverified-to-reviewed transition requires steward plus a trimmed 10–500 character reason. Show before/after and label the button “Save access change.” A rank increase with a tag removal remains a downgrade. See W-180-05, T-180-10.

### R-180-11

**P0 — Audit.** Audit every accepted admission label and changed label with actor, scope, source, before/after, version, policy epoch, reason when required and operation ID. Audit failure aborts a label mutation. No-op retries do not generate duplicate change events. See T-180-11.

### R-180-12

**P0 — Derived enforcement.** Relabel document and all protected descendants atomically; fence cached context, answers and in-flight responses using scope/epoch. Unknown child coverage blocks release. See T-180-12.

### R-180-13

**P0 — Truthful status.** Show on/off, unverified, legacy-null, unavailable and break-glass states distinctly. Unverified copy acknowledges authorized stewards. Never display “protected” when enforcement is off or policy lookup failed. See W-180-08, T-180-13.

### R-180-14

**P1 — Find and review.** Filter visible documents by sensitivity, required security tags and review state on the server before counting/paging. A filter never broadens clearance. Provide “Needs review” to authorized reviewers. See T-180-14.

### R-180-15

**P1 — Bulk safety.** Support explicit Add tags, Remove tags or Replace access on up to 100 selected visible documents. Show mixed state, changed/unchanged counts and blocked rows before commit. Commit is all-or-nothing; a stale/unauthorized member changes none. See W-180-06, T-180-15.

### R-180-16

**P0 — Vocabulary integrity.** Reject unknown, inactive, wrong-kind and foreign-workspace IDs. Prevent deletion of tags referenced by document labels or policy rules; retirement never strips protection. See T-180-16.

### R-180-17

**P0 — Scope and lifecycle.** Workspace/principal changes clear draft permissions and cached previews; reprocessing preserves current access; worker retry never restores an older label. Dedup never reveals hidden document IDs, titles or processing status. See T-180-17.

### R-180-18

**P0 — Consistent read model.** Library, preview, detail, upload result and filters share sensitivity labels, tag formatting, capabilities and versions. Keep saved state visually distinct from unsaved draft. See T-180-18.

### R-180-19

**P0 — Accessibility.** Keyboard-complete pickers/dialogs, screen-reader names, focus return, explicit errors, readable contrast, zoom/reflow and reduced-motion behavior. See [UX/UI](05-ux-ui-spec.md), T-180-19.

### R-180-20

**P0 — Performance.** Warm picker opens ≤100 ms p95; assignable-tag search ≤300 ms p95 at 10,000 tags; label save ≤750 ms p95 with 5,000 child rows; cold capability load ≤500 ms p95 in the controlled test environment. These are targets, not measured results. See T-180-20 and [DB](08-data-and-db-contract.md).

### R-180-21

**P0 — Recovery.** Retain drafts on validation/network errors, distinguish unknown commit outcome from failure, retry with an idempotency key, and never auto-resubmit after scope/auth changes. See T-180-21.

### R-180-22

**P0 — Compatibility.** Preserve the current wire words `classification` and `compartment_ids`. Old uploads still default to unverified under enforcement. Legacy PATCH preconditions need an explicit, time-bounded compatibility path; no indefinite lost-update bypass. See [API](07-api-contract.md), T-180-22.

### R-180-23

**P0 — Measurable trust.** Collect content-free completion/error metrics, measure comprehension and time on realistic tasks, and forbid experiments that relax protection to improve conversion. See [Research](14-research-and-benchmark.md), T-180-23.

### R-180-24

**P0 — Evidence before release.** Mocked UI screenshots alone cannot pass security acceptance. Require live PostgreSQL tests under the application RLS role, actual authenticated browser E2E and cross-channel retrieval/download checks. See [Tests](12-e2e-test-matrix.md), T-180-24.

## Outcome and guardrail scorecard

Proposed acceptance cohort: 12 representative participants (uploaders, editors and stewards), each performing the benchmark tasks. Report raw results, median, p90 and task failures; the small sample is directional, not statistical market proof.

| Metric | Target | Guardrail |
|---|---|---|
| Correctly label and upload one document | ≥95% completion; median ≤30 seconds after files selected | Zero unintended audience broadening |
| Correctly edit sensitivity plus two tags | ≥95% completion; median ≤20 seconds | Zero changes from Cancel/Escape |
| Explain HR + Atlas and Public correctly | ≥90% correct answers for each | Do not combine questions into one average |
| Review 20 existing documents | ≥90% complete within 2 minutes | No hidden-resource names or counts |
| Cross-surface label disagreement | Zero in deterministic contract suite | Fail release on stale saved-state claims |
| Security negative cases | 100% pass | No discretionary waiver for disclosure |

Events: `access_editor_opened`, `access_preview_completed`, `access_save_result`, `upload_access_review_completed`; dimensions limited to surface, capability class, result code, duration bucket and batch-size bucket. No filenames, tag names/IDs, document IDs, member IDs or reason text in product analytics. Security audit has separate scoped retention/access. See [Marketing/Growth](lenses/LENS-marketing-growth.md).
