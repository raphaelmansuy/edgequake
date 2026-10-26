# Lens — Product Owner

[Pack index](../README.md) · [Requirements](../03-product-spec.md) · [Assessment](../02-current-implementation.md) · [Plan](../13-implementation-plan.md) · [Tests](../12-e2e-test-matrix.md) · [Research](../14-research-and-benchmark.md)

**First principle:** Make correct access decisions part of completing the document job.

## Problem and priority

Upload intent is currently disconnected from document Access properties (F-180-01/02). Prioritize shared upload/property flows and reliable saved-state feedback before advanced policy features.

## Key decisions

Retain one fixed sensitivity plus all-required security tags. P0 offers local file review and direct initial labeling for cleared writers; reviewing existing unverified content remains a steward action. P1 bulk and filters are specified separately.

## Acceptance

Use R-180-01/02/03/04/13/15/23 and benchmark tasks A–F. Require task completion and comprehension together; no conversion gain offsets a security regression.

## Tradeoff

A pre-upload review adds an explicit action to the current immediate-upload flow. Measure the time cost against fewer correction steps; retain a compact review, never remove the security decision through an unreviewed quick-upload shortcut.

## Edge focus

EC-180-01/04/13/15/22/23/25. New users without assignable reviewed labels can still upload Needs review; they are not forced to administer vocabulary.

## Implementation ownership

Sign off WP-02 capability decisions, WP-04 upload behavior, WP-06 bounded bulk semantics and WP-08 outcomes. Product decides policy language; the server enforces it.

## Release question

Can an uploader explain who can read this document, finish the task in context, and recover without losing intent? Validate through T-180-01/03/04/15/23/25.

The lens is a review perspective applied within this pack, not a claim of a separate independent expert review. Resolve conflicting recommendations against [the policy laws](../01-first-principles.md), then record a concrete requirement/test change.
