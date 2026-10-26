# 14 — Research, reference design and competitive benchmark

[Index](README.md) · Problem: [Five WHYs](00-five-whys.md) · Targets: [Product](03-product-spec.md) · Visual adaptation: [UX/UI](05-ux-ui-spec.md) · Claims: [Marketing/Growth lens](lenses/LENS-marketing-growth.md)

Research date: 2026-10-26. Google discovery and primary documentation were consulted. The requested `fetch_webpage` tool is not available in this session; the available web search/open/find/click tools were used instead. Relevant documentation links were followed, including Microsoft’s end-user labeling instructions. This is bounded task research, not an exhaustive crawl or hands-on benchmark of every market product.

## Primary-source evidence and design implications

Each row distinguishes a documented fact from the proposed EdgeQuake response. Brief paraphrases below are the research evidence; the full requirements elsewhere are our design choices, not claims made by these sources.

| Source | Verified fact | Proposed application |
|---|---|---|
| [NIST SP 800-162](https://csrc.nist.gov/pubs/sp/800/162/upd2/final) | ABAC evaluates subject, object, operation and sometimes environmental attributes | Describe EdgeQuake's bounded rank/tag model accurately; preserve operation RBAC |
| [Microsoft Purview sensitivity labels](https://learn.microsoft.com/en-us/purview/sensitivity-labels) | One sensitivity label applies per item; policy can require labeling and downgrade justification | One sensitivity choice and deliberate broadening reason; do not equate labels with universal encryption |
| [Microsoft end-user label instructions](https://support.microsoft.com/en-us/office/security-privacy/apply-sensitivity-labels-to-your-files) | Users can change labels; configured mandatory labeling and downgrade prompts affect that interaction | Make the saved label visible and explain why a change is blocked |
| [Google Drive labels](https://support.google.com/drive/answer/13495216?hl=en) | Admins create labels; file/label permissions govern use; bulk application and side-panel edits are documented | Governed picker, explicit Save, bulk workflow and scoped catalog |
| [Box classification policies](https://support.box.com/hc/en-us/articles/1500003376782-Classification-Policies-Automated-Classification) | Classified files show badges in lists and classification details in the preview sidebar | Persistent list summary plus document-property context |
| [OWASP authorization](https://cheatsheetseries.owasp.org/cheatsheets/Authorization_Cheat_Sheet.html) | Deny by default and validate authorization on every request | New endpoints share server policy; direct API bypass attempts are tests |
| [PostgreSQL 18 RLS](https://www.postgresql.org/docs/18/ddl-rowsecurity.html) | Owners and BYPASSRLS roles can bypass ordinary row policy enforcement | Test with real application role; retain restrictive scope and clearance checks |
| [PostgreSQL row locking](https://www.postgresql.org/docs/18/explicit-locking.html) | FOR UPDATE blocks conflicting row modification/locking until transaction ends | Serialize current-state mutation and parent/child label writes |
| [W3C modal dialog pattern](https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/) | Modal focus stays within the dialog; dismissal and return focus are explicit interactions | Dialog focus contract and nested picker dismissal tests |
| [WCAG 2.2 target size](https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html) | AA minimum target guidance uses 24 CSS pixels with defined exceptions | Meet minimum, target 44px touch areas without copying dense controls blindly |
| [Radix Popover](https://www.radix-ui.com/primitives/docs/components/popover) | Exposes focus, Escape and outside-interaction hooks | Separate picker close from security mutation; use existing wrapper |
| [TanStack Query v5 invalidation](https://tanstack.com/query/v5/docs/react/guides/invalidations-from-mutations) | Awaiting mutation-success invalidation keeps pending status until refresh completes | Reconcile before Saved, plus explicit purge for sensitive stale content |
| [Playwright best practices](https://playwright.dev/docs/best-practices) | User-facing locators and retrying assertions produce resilient behavior tests | Assert actual request/download/security behavior, not only component CSS |

No source demonstrates that EdgeQuake is currently superior, or that any competitor lacks an equivalent feature. The research supports design patterns and a comparison protocol, not negative claims about undocumented competitor behavior.

## Reference design evidence

| gr8r-studio artifact | Observed pattern | Adaptation boundary |
|---|---|---|
| [tokens.css](/Users/raphaelmansuy/Github/03-working/gr8r-studio/src/styles/tokens.css) | Warm neutrals, Geist, restrained accent, independent light/dark palettes, 4px spacing | Map into EdgeQuake semantic tokens; independently check contrast |
| [drawer.js](/Users/raphaelmansuy/Github/03-working/gr8r-studio/src/overlays/drawer.js) | Labelled property grid, compact chips, side-panel/full-page context | Separate sensitivity/security tags and preserve document preview context |
| [popovers.js](/Users/raphaelmansuy/Github/03-working/gr8r-studio/src/overlays/popovers.js) | Searchable label list, selected state, contextual placement | Checkbox-based accessible picker with server assignability |
| [panels.css](/Users/raphaelmansuy/Github/03-working/gr8r-studio/src/styles/panels.css) | Quiet borders, aligned row insets, sparse elevation | One structured access panel rather than nested cards |
| [preview.png](/Users/raphaelmansuy/Github/03-working/gr8r-studio/docs/preview.png) | Shared gutters, clear hierarchy, calm information density | Match clarity; don't copy tiny targets or use color as the sole signal |

Reference repository is read-only input to this task; no files there are modified. No browser/runtime audit of gr8r-studio was performed; source and its supplied preview were inspected.

## Task-based superiority hypothesis

The proposed differentiator is a coherent workflow across upload, document properties and RAG-derived access: one control, explicit consequence, deliberate save and verifiable enforcement. Many competitors already have labeling, bulk operations and policy explanations. Winning requires execution and measured outcomes.

Benchmark against configured current Microsoft 365/Purview, Google Drive labels and Box Shield setups, with equivalent user authority and as close a policy model as each product supports. Record plan/license, configuration, client version and date. If a product cannot express equivalent all-tag clearance, mark the task non-equivalent; do not count it as a competitor failure. RAG-specific tests are EdgeQuake security gates rather than generic document-product comparisons.

| Task | Scenario | Measures |
|---|---|---|
| A: label before upload | One confidential HR document, then mixed-format batch with one override | Time, clicks, completion, wrong label and rework |
| B: edit in context | Change a visible document's sensitivity and tags while reviewing content | Time, navigation hops, unintentional commits, recovery |
| C: explain audience | HR + Atlas, then Public, then unverified | Independent correct answers; confidence versus actual policy |
| D: handle broadening | Remove Atlas and lower sensitivity | Recognizes impact, reason completion, correct authority |
| E: conflict recovery | Two editors change one label | Lost updates, successful informed recovery, errors |
| F: bulk review | Apply a tag to 20 documents with mixed initial labels | Completion, preservation of existing constraints, blocked-item clarity |
| G: accessible operation | Complete A/B using keyboard/AT and small viewport | Completion, focus faults, critical accessibility failures |

Use counterbalanced task order, identical instructions, realistic but synthetic documents and facilitator-neutral help rules. Proposed first study uses 12 participants with relevant roles. Capture raw trials, median/p90, failures and qualitative confusion; do not infer population-wide superiority from this sample. Use the [Product](03-product-spec.md#outcome-and-guardrail-scorecard) targets as release objectives, then expand sample size for public comparative claims.

Do not gate security behind upsell prompts or run an A/B experiment that changes enforcement. Product analytics remain content-free. Successful labeling, comprehension and trustworthy recovery are activation signals; more clicks on security controls are not success by themselves.

Fixture hash generation used the installed Argon2id library with a fresh random salt, following the [argon2-cffi API](https://argon2-cffi.readthedocs.io/en/stable/api.html). No database password hash was copied into the specification artifacts.
