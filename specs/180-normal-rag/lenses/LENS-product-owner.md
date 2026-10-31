# Lens — Product owner

Parent: [README](../README.md) · Spec: [03-product-spec](../03-product-spec.md)

## View

The choice is "what kind of index am I paying for," not a tuning knob. Graph stays the default so current customers see no change. Normal exists because single-hop lookup and page finding do not need a graph (R-180-1, R-180-2), and those corpora should not pay extraction.

Permanence is part of the offer. A switch that rebuilds in place sounds flexible and creates a class of half-migrated workspaces. A new workspace is an honest cost.

## Risks

| Risk | What we do |
|------|------------|
| "No LLM" is read as "PDFs are free" | Copy says extraction is off. PDF vision is a separate line on the review screen. |
| Users pick Normal, then ask multi-hop questions, and blame quality | Review text says what Normal is for. The eval note records the gap instead of hiding it. |
| A client SDK sends the profile back on save and gets 409 | Release note. SDK strips the field. |
| Two profiles split the roadmap forever | Capabilities are one struct so a third profile does not fork the worker. |

## Acceptance

- Wizard defaults to Graph.
- Review states permanence and whether images are embedded.
- Settings cannot edit the profile.
- Success metrics in [03-product-spec](../03-product-spec.md) are the ones W6 reports.
- No dollar figure is invented.

## Laws owned

LAW-180-1, LAW-180-12. WHY-A.
