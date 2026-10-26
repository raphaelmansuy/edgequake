# Lens — UX and UI Designer

[Pack index](../README.md) · [Requirements](../03-product-spec.md) · [Assessment](../02-current-implementation.md) · [Plan](../13-implementation-plan.md) · [Tests](../12-e2e-test-matrix.md) · [Research](../14-research-and-benchmark.md)

**First principle:** Make access consequences understandable before the user commits.

## Design grounding

gr8r-studio provides calm surfaces, compact labelled properties, search-first labels and contextual drawers. Apply those patterns through EdgeQuake's existing tokens/components rather than copying its DOM implementation.

## Key decisions

Use Sensitivity, Security tags and Who can access as distinct information. ALL-tag explanation is adjacent to choices, not hidden in a tooltip. Save/Cancel own the outer draft; picker Done only closes the picker.

## Hierarchy

Lead with the file and effective access rule. Keep parser/model settings separate. Broadening review includes the exact removed requirements and reason; no decorative security dashboard blocks a simple task.

## Edge focus

EC-180-02/03/04/06/13/18/19/21/25/31/33. Empty, loading and failed catalog are separate states. Unverified acknowledges stewards; off-mode never looks protected.

## Accessibility acceptance

T-180-02/04/06/13/19/21/31, manual keyboard/AT, focus return, 400% reflow and forced colors. Use checkbox semantics rather than assuming a multi-select combobox pattern.

## Tradeoff

Avoid showing a complete audience list by default. A precise policy sentence protects directory privacy and is more truthful than a guessed count; scoped member checks are steward-only.

## Implementation ownership

Own W-180-01 through W-180-09 and copy in WP-04/05/06; sign off behavior screenshots and usability tasks in WP-07/08.

The lens is a review perspective applied within this pack, not a claim of a separate independent expert review. Resolve conflicting recommendations against [the policy laws](../01-first-principles.md), then record a concrete requirement/test change.
