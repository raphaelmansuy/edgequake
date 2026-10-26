# 00 — Five WHYs

[Index](README.md) · Next: [First principles](01-first-principles.md) · Evidence: [Current implementation](02-current-implementation.md)

## WHY 1 — Why improve document access at upload?

An uploader needs to decide who may use a document before it contributes to search and answers. The current upload dispatcher carries parsing/extraction options, but no typed clearance label. Admission stamps an omitted label as unverified when enforcement is on. Correct protection can therefore require a second workflow after upload. See F-180-01 and [R-180-01](03-product-spec.md#r-180-01).

## WHY 2 — Why is a second workflow a problem?

The user must find the document, open its preview, find Access, and understand classification plus “Departments and projects.” That adds memory work and encourages an unreviewed backlog. Backlog size and abandonment are hypotheses to measure, not observed production metrics. See [the product lens](lenses/LENS-product-owner.md) and [benchmark tasks](14-research-and-benchmark.md).

## WHY 3 — Why do labels need explanation?

A familiar chip suggests organization. Here a security tag narrows access: a document tagged HR and Atlas requires **both** tags. “Public” still requires workspace authorization and a sufficient clearance rank. An ordinary taxonomy interpretation can cause accidental disclosure or unexpected denial. See [policy semantics](04-policy-and-capabilities.md), F-180-04, and [R-180-04](03-product-spec.md#r-180-04).

## WHY 4 — Why does editing need a deliberate commit?

`DocumentAccessControl.handleOpenChange` currently submits a changed draft when the popover closes. The user receives no dedicated review step for removing a compartment or lowering sensitivity, and the PATCH has no revision precondition. Closing and committing express different intentions. See F-180-02, F-180-05, [screen W-180-05](06-screens-and-navigation.md#w-180-05), and [the API contract](07-api-contract.md).

## WHY 5 — Why solve this as a shared domain workflow?

The same access intent crosses upload, property editing, batches, API clients and MCP. Independent UI patches would duplicate normalization, permissions, warnings and payload construction. The root design problem is an incomplete user-visible contract between **intent, effective policy and committed state**. See [LAW-180-03](01-first-principles.md#law-180-03), [frontend architecture](09-frontend-architecture.md), and [the implementation plan](13-implementation-plan.md).

## Root cause and design response

Preserve the existing policy evaluator. Introduce one server-derived capability/read model and one reusable access draft/editor. Present “Sensitivity,” “Security tags” and “Who can access” where the user makes the decision. Commit the approved label before processing becomes visible, then propagate it atomically to derived resources.

## Hypotheses to test before claiming success

| Hypothesis | Falsifier | Evidence gate |
|---|---|---|
| Upload-first labeling reduces rework | Users still navigate to settings to complete a normal upload | T-180-01, usability task A |
| A rule sentence explains AND semantics | Users interpret HR + Atlas as either tag | T-180-04, comprehension study |
| Explicit Save prevents accidental mutations | Escape/outside click still sends PATCH | T-180-02 |
| Shared controls reduce inconsistent behavior | Same draft serializes differently for PDF/text/MCP | T-180-08 |
| Trust improves adoption | Completion rises while mislabeling also rises | Guardrail metrics in [Product](03-product-spec.md) |
