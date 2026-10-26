# 01 — First principles

[Index](README.md) · Inputs: [Five WHYs](00-five-whys.md) · Applied by: [Policy](04-policy-and-capabilities.md), [API](07-api-contract.md), [Tests](12-e2e-test-matrix.md)

## LAW-180-01

**Access is the intersection of independent gates.** Tenant/workspace scope, operation permission, and document clearance must all permit a request. A useful-looking UI cannot grant authority. Hidden resources return the normal 404 shape; capability discovery must not reveal their existence. [OWASP authorization guidance](https://cheatsheetseries.owasp.org/cheatsheets/Authorization_Cheat_Sheet.html) supports server-side checks on every request.

## LAW-180-02

**Security attributes have consequences.** A document has one sensitivity rank and a set of required compartments. More required tags normally means a narrower audience. Ordinary descriptive tags must never silently become policy inputs. This pack exposes existing compartments as “Security tags”; a separate general-purpose tagging system is out of scope.

## LAW-180-03

**One intent has one canonical representation.** Normalize and validate once on the server; reuse one browser draft model and serializer across upload and edit. The backend `allows` / `allows_relabel` functions remain authoritative. Generated types and shared contract fixtures prevent accidental duplication across languages.

## LAW-180-04

**Protection precedes exposure.** The accepted document shell, original, task and descendants must carry a validated label before reads, worker processing or progress broadcasts can expose them. Do not implement upload-labeling as upload followed by a browser PATCH. Missing labels under enforcement retain the unverified default.

## LAW-180-05

**A security change is a transaction with intent.** Save means commit; Escape, outside click and Cancel do not. Stale versions conflict, rather than overwrite. A sensitivity decrease or removal of any required compartment needs steward authority and an audit reason. A mixed change that raises rank but removes a tag is still a downgrade for authorization purposes.

## LAW-180-06

**Explain only what the server can establish.** A rule summary is not an exact audience count. Profile simulation is not a live member roster. Unverified means “You and authorized security stewards, until reviewed,” subject to the actual policy; it never means a private personal vault. Enforcement off and break-glass require truthful status copy.

## LAW-180-07

**Security changes invalidate derived access.** Labels apply to originals, text, chunks, pages, assets, tasks, contributions, search results and answer context. Epoch changes must fence caches and stale in-flight reads. Previously downloaded bytes or externally copied answers cannot be retracted by changing a label.

## LAW-180-08

**Recovery preserves intent without widening access.** Failed saves retain the draft. Conflict recovery never auto-merges an access relaxation. Retry is idempotent. Missing settings, unknown attributes and failed previews cannot silently select Public or an empty security set.

## LAW-180-09

**Usability includes keyboard, assistive technology and small screens.** The workflow must be operable without drag, hover or color perception. WCAG 2.2 AA and manual keyboard/screen-reader tests are release gates. [W3C dialog guidance](https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/) informs focus behavior, with implementation details in [UX/UI](05-ux-ui-spec.md).

## LAW-180-10

**Quality is measured against tasks.** Market leadership requires comparable task outcomes, comprehension, error rates and security proof. Publish no unsupported superiority claim. Optimize legitimate completion and correct decisions together. See [Research](14-research-and-benchmark.md) and [Marketing/Growth](lenses/LENS-marketing-growth.md).
