# Lens — AI Engineer

Parent: [README](../README.md) · Spec: [03](../03-product-spec.md) · [08](../08-api-contract.md)

## Why the AI engineer owns part of this pack

Creating a scope is where every model decision is made for the life of that scope. Get it wrong
and:

- every document in the workspace is embedded with a model the organization did not approve,
- the extraction pipeline runs with a model the tenant ceiling forbids,
- a rebuild is required after the first upload, because the default changed under the data,
- the Acc benchmark moves because a default silently changed.

The pack does not change the model **resolution** ladder
([SPEC-123](../../123-env-config-priority/), [SPEC-175](../../175-perfect-settings/),
[SPEC-178](../../178-provider-and-model-choice/)). It changes **where the choice is surfaced at
creation**, and it makes inheritance visible instead of implicit.

## What creation decides, per usage

| Usage | Decided at tenant create | Decided at workspace create | Inherited afterwards |
|-------|--------------------------|------------------------------|---------------------|
| `extract` | `default_llm_provider/model` | `llm_provider/model` (usually omitted) | tenant → server → env |
| `query` | inherits `extract` unless overridden | `llm_roles.query` | workspace → tenant → server |
| `summary` | inherits | `llm_roles.summary` | |
| `vlm` | `default_vision_*` | `vision_llm_*` | |
| `keyword` | — | — | server only |
| `embedding` | `default_embedding_*` + dimension | `embedding_*` | tenant → server |
| `decision` | — | `decision_model`, `extraction_mode` | server default `tev1:0.8b` |

The short dialog exposes the three that matter for cost and quality: **extract**, **embedding**,
**vision**. Everything else is in the drawer with its inherited value printed.

## The three invariants that must survive the UI change

### 1. Omitted field stays inherited (LAW-123-8)

```rust
// workspace_crud.rs — the comment is the law
// SPEC-032 / LAW-123-8: the tenant must exist, but omitted model fields are NOT copied
// from the tenant. They stay inherited and resolve at read time, so a later tenant
// default change still reaches this workspace.
```

The short dialog must therefore send **nothing** when the person chooses "Inherit". A UI that
helpfully materializes the resolved value into the request would freeze the workspace to today's
tenant default and break the cascade for everyone created after this pack.

Gate: `R-create-two-requests` asserts the POST body for an inheriting create contains no
`llm_model` / `embedding_model` keys.

### 2. Eligibility is judged before storage (SPEC-178)

`ensure_tenant_choice` and `ensure_workspace_choice` run before anything is written. The tenancy
service calls them; it does not re-implement them. The short dialog surfaces the refusal next to
the control (`422 models_not_eligible`), which is strictly better than the current
`describeCreateFailure` → toast → jump to the models step.

Gate: `E-model-not-eligible`.

### 3. The profile is immutable (SPEC-180)

`rag_profile`, `embedding_modalities`, `fts_regconfig` are fixed at create and enforced by a DB
trigger (`eq_workspace_profile_immutable`). That is exactly why **retrieval type is a top-level
decision in the short dialog**, not a drawer row: it cannot be changed later, so burying it
would be a trap.

The copy says so: *"This is permanent."*

Gate: `E-create-ws-three-decisions` asserts the permanence line is present.

## Model defaults shown at creation (the inherit row)

```text
  Models
  ( • )  Inherit from Acme
           Extract    gpt-5-nano · openai
           Embed      text-embedding-3-small · openai · 1536
           Vision     not set
  ( )    Choose models for this workspace
```

Rendered from `GET /config/effective?tenant_id=…` (SPEC-175) so the line a person reads is the
model that will run, not a guess from the tenant row
([LAW-175-1](../../175-perfect-settings/01-first-principles.md)).

`Vision: not set` is stated, not omitted. Absence of a vision model is a real state that changes
PDF parsing behaviour, and hiding it is how operators end up with a silently text-only pipeline.

## Decision engine at creation

`extraction_mode` (`llm` | `decision`) and `decision_model` are drawer rows, defaulting to
`inherit` (`EDGEQUAKE_EXTRACTION_MODE`, default `llm`). A workspace created in `decision` mode
with a model the deployment has not pulled will fail every extract call.

Mitigation, already available: `GET /decision/status` reports the model and its readiness
([SPEC-160](../../160-tev1/)). The drawer row calls it and prints `ready` / `model not pulled`
next to the choice. That is a two-line addition and it removes the most common "my ingest is
stuck" ticket for decision-mode workspaces.

## MCP parity

The MCP surface (`eq_ingest`, `eq_upload_*`, `eq_workspace_list`) must see the same tenancy
rules. Two changes:

| Tool | Change |
|------|--------|
| `eq_workspace_list` | row-scoped by `me/access` (it already calls the same route, so this is inherited) |
| `eq_ingest` | a workspace the token cannot write → `capability_required:workspace.content.write`, not a 500 |

`EQ_MCP_SCOPE` semantics are unchanged ([SPEC-152 §08](../../152-new-mcp-contract/08-security-observability.md)).
Gate: `R-mcp-scope`.

## What must not move

| Thing | Why | Gate |
|-------|-----|------|
| Extraction prompts | Any change to them moves Acc | `make bench` in the DoD |
| `resolve_embedding_choice` | Dimension is derived from the model card | `R-create-two-requests` (dimension present when a model is chosen, absent when inherited) |
| `EDGEQUAKE_NORMAL_*` defaults | SPEC-180 retrieval tuning | unchanged path |
| Chunking defaults (1200/100) | Acc-sensitive | drawer row shows the inherited value; nothing writes it unless edited |
| The decision gate preset default (`balanced`) | Cost and recall | drawer row shows it |

## Cost implication of the short path

A workspace that inherits everything costs the operator nothing new: the models are the tenant
defaults, which were already chosen once. The pack removes the *re-statement*, not the decision.

A workspace that overrides is one click deeper and shows the eligibility verdict inline, which
reduces the number of workspaces created with a model the ceiling will later refuse — a
correctness and a cost win at the same time.

## Telemetry worth adding (small, privacy-safe)

| Event | Fields | Why |
|-------|--------|-----|
| `create_workspace` | `inherit_models: bool`, `decision_count`, `elapsed_ms`, `rag_profile` | proves [LAW-181-4](../01-first-principles.md) and tells us which drawer rows actually get edited |
| `create_organization` | `owner_kind`, `plan`, `elapsed_ms` | tells us whether the owner field is used |
| `create_refused` | `reason_code`, `capability` | must trend to zero on create routes |

No prompt, no content, no model output. Counts and enums only, consistent with SPEC-124's
observability posture.

## The Acc guard

This pack touches `create_workspace` (the quota check moves, the defaults do not). The Acc
benchmark must be unchanged. `make bench001-doctor && make bench` is in the definition of done
for that reason, and the review question is one line: *did any default that reaches the extract
path change?* The answer must be no, and `R-create-two-requests` is the proof at the request
boundary.
