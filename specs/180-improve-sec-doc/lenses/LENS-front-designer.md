# Lens — Front Designer

[Pack index](../README.md) · [Requirements](../03-product-spec.md) · [Assessment](../02-current-implementation.md) · [Plan](../13-implementation-plan.md) · [Tests](../12-e2e-test-matrix.md) · [Research](../14-research-and-benchmark.md)

**First principle:** Translate the reference into precise, stable, accessible components.

## Visual contract

Use the token mapping in 05-ux-ui-spec: warm neutrals, restrained accent, 4px spacing, shared gutters, modest radius/shadows and independently tested dark mode. Keep Geist and Lucide already used by EdgeQuake.

## Component craft

One reusable form body serves upload and edit. Saved properties use quiet labels; drafts show Unsaved changes. In tables use bounded chips with accessible overflow; in editors wrap full names.

## Motion and layout

Use short restrained transitions, fixed-height loading slots and scroll padding above sticky footers. Reduced motion removes translation. Open/close must preserve document preview scroll and avoid layout jumps.

## Implementation boundaries

Render components receive typed values/options/callbacks. Query hooks handle fetching, reducer handles draft lifecycle, API adapter handles transport. Don't add hidden mutation effects or split security state across local stores.

## Edge focus

EC-180-02/06/18/19/21/25/31. Test event bubbling from upload settings, stale preview response, long localized names, responsive reflow and dark focus contrast.

## Acceptance

T-180-01/02/06/18/19/21/25/31. Visual QA at 320/375/768/1440px, keyboard and touch. Compare actual implementation screenshots to wireframes and reference hierarchy; pixel matching isn't the purpose.

## Implementation ownership

Lead shared picker/fields and upload review in WP-04, property editor in WP-05, bulk layouts in WP-06; ensure testability without component-internal selectors.

The lens is a review perspective applied within this pack, not a claim of a separate independent expert review. Resolve conflicting recommendations against [the policy laws](../01-first-principles.md), then record a concrete requirement/test change.
