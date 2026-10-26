# 05 — UX/UI specification

[Index](README.md) · Behavior: [Product](03-product-spec.md) / [Policy](04-policy-and-capabilities.md) · Layouts: [Screens](06-screens-and-navigation.md) · Implementation: [Frontend](09-frontend-architecture.md)

## Design direction grounded in gr8r-studio

Reference inspected: [tokens.css](/Users/raphaelmansuy/Github/03-working/gr8r-studio/src/styles/tokens.css), [drawer.js](/Users/raphaelmansuy/Github/03-working/gr8r-studio/src/overlays/drawer.js), [popovers.js](/Users/raphaelmansuy/Github/03-working/gr8r-studio/src/overlays/popovers.js), [panels.css](/Users/raphaelmansuy/Github/03-working/gr8r-studio/src/styles/panels.css), and [preview.png](/Users/raphaelmansuy/Github/03-working/gr8r-studio/docs/preview.png). These absolute local reference links are intentional; portable findings are captured here and in [Research](14-research-and-benchmark.md).

Adopt the reference's calm surfaces, shared gutters, bordered panels, quiet property labels, searchable label popover, and contextual drawer/full-page pattern. Keep EdgeQuake's React/Radix/Tailwind components, Geist typography and Lucide icons. Do not copy its string-built DOM/event architecture or automatic save behavior for access changes.

| Reference token/pattern | SPEC-180 application |
|---|---|
| Light `--bg #fbfaf8`, `--surface #fff`, `--border #e9e7e2` | Warm neutral reference for access panel surfaces; map to existing semantic tokens, no page-specific hardcoded theme |
| Dark `--bg #1a1a19`, `--surface #1d1d1c`, text `#eceae5` | Independently designed dark palette; verify contrast before adoption |
| Accent `#4b5bd6` light / `#8e9af3` dark | Focus/primary action inspiration; one accent, no decorative gradients |
| 4-point spacing, gutter 24, radii 4/6/8/10 | 16px editor padding, 8px chip gaps, 24px section spacing, 8px panel radius |
| `dl.kv` property rows with pill values | Separate Sensitivity and Security tags rows; access rule underneath |
| Label picker `Filter labels...` | Searchable governed security tag picker with selected section |
| Drawer retains surrounding task context | Document properties remain beside preview; no forced settings visit |
| Small shadows, 140ms transitions | Restrained popover elevation; 120–160ms opacity/translation, zero motion when reduced |

## Interaction contract

W-180-01 keeps the access summary visible when parser options collapse. Clicking a security control must never open the file chooser through dropzone event bubbling. Files selected by drag, keyboard or picker enter the same local review state. Parser and model settings remain grouped separately from access.

W-180-02 defaults to one batch label with a clear “Applies to N files” sentence. Each file can override it; an override badge and Reset to batch action make the exception visible. A later shared-default change updates only non-overridden, unsubmitted items. Submission freezes each file's label and workspace identity; a later queue edit affects only not-yet-submitted items.

W-180-03 uses one search field and native checkbox rows. The group is labelled “Required security tags”; helper text says “Readers must hold every selected tag.” Search never hides selected values from the selected section. At most two chips show inline in a table plus a named “Show N more security tags” button. Full names are available on focus and in the editor, not hover alone. Sorting is stable by localized display name then ID. Duplicate names include an authorized key/description disambiguator.

W-180-04 owns the draft. Picker selections update only that draft. The footer shows Cancel and Save access; unchanged draft disables Save. Save is also disabled while capability/preview lookup is unresolved or failed. Inline status states Checking access, Saving, Saved, or Could not save. Toasts may supplement, never replace inline errors. No optimistic “Saved.”

W-180-05 appears only for broadening/review. It displays exact removed tags and old/new sensitivity, a reason field and the changed rule. A non-steward sees why it is blocked and can return to a permitted draft; there is no cosmetic confirmation that bypasses policy. No automatic Undo for security changes: a reversal is a new authorized, audited operation with a fresh preview.

## Responsive and accessible behavior

| Concern | Required behavior | Test |
|---|---|---|
| Desktop ≥1024px | 420–480px property drawer, 360–420px editor; collision-aware picker, no nested page scroll trap | T-180-19/20 |
| Tablet 768px | Drawer constrained to `min(480px, viewport - 32px)`; wrap tags; preview still reachable | T-180-19 |
| 320–767px / zoom reflow | Full-width modal editor; stacked properties; sticky footer with scroll padding so focused controls remain visible | T-180-19 |
| Keyboard | Tab visits search, checkboxes, summary, Cancel and Save in reading order; Space toggles checkbox; no hidden arrow-key-only multi-select | T-180-19 |
| Nested Escape | First Escape closes picker; outer Escape initiates discard if dirty; never saves | T-180-02/19 |
| Dialog semantics | Labelled dialog; modal only when background is inert; focus enters heading/search and returns to invoker or nearest surviving document row | T-180-19 |
| Errors | Focusable top error summary on failed submit, field links, `aria-invalid` / `aria-describedby`; do not shift focus on every checkbox toggle | T-180-21 |
| Announcements | Polite live region for preview/save result, assertive error only on failure; aggregate batch result, not hundreds of messages | T-180-19 |
| Contrast and targets | WCAG AA: 4.5:1 normal text, 3:1 large text and required non-text UI contrast; 24×24 CSS px minimum targets with applicable spacing rules; product target 44×44 touch hit areas | T-180-19 |
| Color and motion | Text + lock/review icon distinguish state; selected checkbox works in forced colors; respect reduced motion | T-180-19 |
| Localization | All strings through existing i18next; plural rules; 40% expansion; RTL ordering; filenames and tags safely rendered as text | T-180-31 |

The 44px touch target is a product choice above the [WCAG 2.2 minimum target guidance](https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html). Follow [W3C modal focus behavior](https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/) and the existing [Radix popover APIs](https://www.radix-ui.com/primitives/docs/components/popover) for dismissal/focus; no new UI library is necessary.

## Copy and state inventory

| State | Copy/action |
|---|---|
| Default enforced upload | “Needs review — you and authorized security stewards, until reviewed.” |
| Public chosen | “Public is a sensitivity level inside this workspace. It does not create a public link.” |
| Security tag group | “Readers must hold every selected tag.” |
| No selected tags | “No security tags required. Sensitivity and workspace permissions still apply.” |
| No assignable tags | “No security tags are available to you.” Show Manage security tags only with capability |
| Catalog network failure | “Security tags could not load. Retry.” Preserve selection; never render as an empty catalog |
| Missing document rank while enforcement on | “Access needs repair. This document is unavailable until reviewed.” No client-side default label |
| Policy off | “Document access rules are off for this workspace.” Optional owner/admin Settings link |
| Policy unavailable | “Access rules could not be checked.” Disable label commit; Retry |
| Conflict | “Access changed while you were editing. Review the latest values.” Offer Compare and Reload |
| Unknown save outcome | “We could not confirm whether the change was saved. Check status.” Retry uses the same operation ID |
| Broadening blocked | “A security steward must approve changes that broaden access.” |
| Break-glass | “Emergency access is active. Document access rules are bypassed and access is audited.” |

Do not say “Everyone,” “Only you,” “Private,” “Secure” or “Protected” without the corresponding actual policy context. The [Front Designer lens](lenses/LENS-front-designer.md) owns visual QA; [Marketing/Growth](lenses/LENS-marketing-growth.md) owns honest messaging.
