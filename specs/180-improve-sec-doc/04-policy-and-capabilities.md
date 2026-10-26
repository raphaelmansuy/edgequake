# 04 — Policy and capabilities

[Index](README.md) · Basis: [Core code assessment](02-current-implementation.md) · Contract: [API](07-api-contract.md) · Verification: [Tests](12-e2e-test-matrix.md)

## Vocabulary and security model

RBAC answers whether a caller may perform an operation. Clearance evaluates document attributes against the caller's compiled access profiles. This is a bounded ABAC model, not a general expression language for time, geography or arbitrary identity claims. [NIST SP 800-162](https://csrc.nist.gov/pubs/sp/800/162/upd2/final) provides the broader subject/object/operation/environment definition.

| User language | Existing storage/wire term | Contract |
|---|---|---|
| Sensitivity | `classification` / `classification_rank` | Exactly one fixed rank |
| Security tags | `compartment_ids` / `document_labels` | Set of governed workspace IDs, all required |
| Access profile | `security_roles` + rules | Subject clearance, not document sharing list |
| Workspace role | `memberships.role` | Operation authority, separate from clearance |
| Needs review | `unverified`, rank 5 | Uploader exception plus authorized steward coverage |

Ranks stay Public=1, Internal=2, Confidential=3, Restricted=4, Unverified=5. Do not reorder ranks or turn unverified into a sixth public-facing sensitivity option. It is a review state alongside the four sensitivity choices; existing storage remains unchanged.

## Decision rule

For normal authenticated use with enforcement on:

```text
read = tenant/workspace authorization
       AND operation permission
       AND (
           uploader reads own unverified document
           OR compiled steward coverage
           OR (reviewed rank <= subject rank AND all required tags held)
       )
```

Empty required tags still need sufficient rank and `allow_empty_compartments`. The compiler applies deny rules to grants; the uploader-unverified exception is a separate path in existing `decide_read`, so do not promise a universal deny precedence over that exception. Break-glass and enforcement-off alter clearance only; they do not remove authentication, operation or tenant scope checks.

Example: Confidential + HR + Atlas requires rank ≥3 **and HR and Atlas**. HR-only and Atlas-only readers cannot see it. A reader whose combined profiles provide HR and Atlas can qualify. `any_of`/`all_of` rule names must not be rendered as “any document tag is enough.”

## Capability matrix

This is the proposed behavior, resolving F-180-03/12. Existing UI owner/admin gating is intentionally replaced, not described as current functionality. All “yes” entries require active authorized scope and the actual policy decision.

| Actor | Upload unverified | Upload reviewed label | Strengthen visible label | Broaden/review unverified | Manage vocabulary/profiles | Check another member |
|---|---|---|---|---|---|---|
| Readonly member, any clearance | No | No | No | No | No | No |
| Writer without useful clearance | Yes, as uploader | No | Only if before/after readable; usually no | No | No | No |
| Writer with matching clearance | Yes | Yes, within clearance | Yes, within clearance | No | No | No |
| Owner/admin without steward grant | Yes | Within clearance | Within clearance | No | Yes | No |
| Steward with write permission | Yes | Within effective steward clearance | Yes if both labels covered | Yes if both covered + reason | Only if owner/admin too | Yes, scoped and audited |
| Service/MCP principal | Only with validated actor attribution or delegated uploader identity | Within explicit operation scope and clearance | Within explicit scope | Only explicit steward authority | Only explicit admin authority | Explicit capability only |

A human steward with readonly operation permission cannot mutate. An owner is not implicitly a steward, although enabling clearance currently assigns the enabling owner's membership the Steward profile. Do not conflate these facts.

## Capability response semantics

Expose `can_upload`, `can_assign_reviewed`, `can_edit`, `can_downgrade`, `can_manage_vocabulary`, `can_check_member`, allowed sensitivity choices and bounded assignable tag results. Boolean flags enable controls; server preview still validates the exact combination. A list of individually assignable tags does not authorize every combination or empty set.

Document edit capability requires operation authority and visibility of the current label. “Can edit” does not mean every target label is valid. No-op saves are harmless but still authorize the existing resource. A result the actor cannot read is rejected in this release, preserving `allows_relabel`; self-lockout workflows and request-for-review delegation are deferred.

## Downgrade and review

The existing evaluator regards lower rank **or any removed compartment** as downgrade. Reviewing unverified rank 5 into ranks 1–4 therefore needs steward authority. First admission of a reviewed label is a create decision (`allows(Write)`), not a fake rank-5 relabel; a sufficiently cleared writer may create such a document directly. This distinction makes upload labeling useful without weakening review authority.

No default selected reviewed sensitivity for a new enforced upload. Display Needs review unless the user explicitly selects an allowed reviewed label. Shared batch defaults apply only to unsubmitted files in that selection; per-file override is visible and frozen at commit. Do not persist security choices across workspace, principal, browser reload or unrelated upload sessions.

## Enforcement off

Show “Document access rules are off for this workspace.” Existing membership protection remains, but no label-based protection is promised. Upload proceeds through existing behavior without persisting new security labels; this release's label mutation controls are disabled in off-mode. Preview describes off-mode and cannot silently enable enforcement. Enabling remains a deliberate owner/admin Settings action with existing backfill/strict-start implications. Old clients attempting label writes in off-mode receive a typed `CLEARANCE_DISABLED` response, covered by compatibility tests.

## Audience summary

Default sentence: “Workspace members with Confidential access or higher and all required security tags: HR, Atlas.” Add “Authorized security stewards may also have access.” For unverified: “You and authorized security stewards, until reviewed.” Public: “Public is a sensitivity level inside this workspace. It does not create a public link.”

Show detailed member evaluation only to stewards allowed to inspect that member in the same workspace. Evaluate all assigned profiles, deny effects, operation read permission and uploader status. Suppress hidden profile names, global directory counts and inferred identities. The explanation is a policy snapshot with epoch, never a guarantee of future membership.
