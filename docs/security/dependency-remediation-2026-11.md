# Security remediation — dependency advisories

**Date:** 2026-11-01
**Trigger:** 243 open Dependabot alerts (17 critical, 92 high, 107 moderate, 27 low).
**Result:** the shippable surfaces are clear; the residue is documented with its reachability.

## Method (first principles)

The alert count is a symptom, not the problem. Three questions decided every action:

1. **Does this ship?** A vulnerability in a dev-only test runner cannot be exploited on a
   running server. Priority follows blast radius, not the severity badge.
2. **Is it reachable?** A parser CVE in a crate we only use to *write* a file is not reachable
   from our input path. Verify by reading the call sites, not the advisory title.
3. **What does the fix cost?** A patch that breaks the build, or forces an unsupported runtime,
   is not a fix. Prefer the smallest change that removes the exposure.

## Shipped-surface fixes

| Surface | Before | After | Change |
|---------|-------:|------:|--------|
| `edgequake_webui` (product UI) | 48 runtime | **1 low** | `next` 16.3.3 → 16.4.3, `dompurify` → 3.4.17, `axios` → 1.20, `mermaid` → 11.16.1, `katex` → 0.19, plus 9 transitive overrides |
| `edgequake-website` | 51 runtime | **0** | `astro` 7.1.3 → 7.4.4 (critical RCE), 16 overrides |
| `mcp/` (MCP server) | 54 | **1 low** | SDK 1.26 → 1.32.2, zod 3 → 4, vitest 4.1.11, 12 overrides |
| `sdks/typescript` | 12 | **1 low** | vitest 4.1.11, tinypool/minimatch/vite overrides |
| `sdks/python` | 3 | **0** | `certifi` 2026.10.23 (compromised root CA), `anyio` 4.15.1 |
| `sdks/rust` | 5 | **0** | `rustls` 0.23.46, `quinn-proto` 0.11.15 |
| `sdks/java`, `sdks/kotlin` | 5 each | **0** | `jackson-databind` 2.18.9 → 2.18.11 |
| `tools/bench047` | 13 | **0** | `certifi`, `anyio`, `filelock` 4.2.6 |

## Two real vulnerabilities found by reading, not by the alert list

### 1. KaTeX `trust: true` — stored XSS in the query view (fixed)

`KATEX_RENDER_OPTIONS` set `trust: true`, which makes KaTeX honour `\href` and
`\includegraphics`. Math arrives from document chunks, model answers and pasted queries, so this
was an XSS sink on untrusted input. Proven before changing it:

```text
\href{javascript:alert(1)}{click}   trust=true  -> href="javascript:…"  (live)
                                    trust=false -> neutralised
```

Fixed by setting `trust: false`. This also matters because `KatexMath` injects the result with
`dangerouslySetInnerHTML`, so KaTeX's own escaping is the only thing between a chunk and the DOM.
Both facts are now recorded in the source, with two regression tests.

The advisory itself (GHSA-238p-pmpm-9mq7) is a *low*-severity prototype-pollution gadget that
needs `trust` to be inherited from a polluted prototype. Our explicit `trust: true` was the
larger problem, and it is gone.

### 2. Mermaid `securityLevel: 'loose'` — investigated, not vulnerable (no change)

The config allows HTML labels and is set in two duplicated `initialize` calls. Mermaid's docs say
`loose` permits arbitrary HTML. I tested the real vectors in a browser rather than trusting the
docs:

```text
<img src=x onerror=…>   onerror stripped, <img> kept
<a href="javascript:…"> javascript: stripped
<svg onload=…>          onload stripped
<iframe src=…>          iframe stripped
```

Mermaid 11 sanitizes internally regardless of `securityLevel`, so this is **not** an XSS vector.
No change made; the finding is recorded so nobody re-opens it from a docs quote.

## Deliberately not fixed

| Package | Why not | Reachability |
|---------|---------|--------------|
| `zip` 2.4.2 (critical) | The fix landed only in 9.0.5; `edgeparse-core` pins `2.2`. No patched 2.x exists, and a `[patch]` would mean forking an upstream PDF library for a code path we never call. | `edgeparse` uses **only** `zip::write::ZipWriter` to emit DOCX. The advisory is in `ZipArchive`/`ZipStreamReader` — readers of untrusted archives. Not reachable. |
| `katex` 0.16.47 | `mermaid` and `rehype-katex` both pin `^0.16.0`; 0.16.47 is the newest 0.16 and no 0.16 patch exists. Forcing 0.19 across the range breaks `mermaid` rendering. | Low severity. The exploit needs `trust` inherited from a polluted prototype; our renderer sets `trust: false` explicitly and does not enable the gadget. |
| `filelock` in `tools/bench047` | Fixed by raising that tool's `requires-python` to `>=3.11` (nothing in it needs 3.10). Done, not deferred. | n/a |

## Regression guards added

- `KatexMath.test.ts` — asserts no `javascript:` href and no attacker image URL can be emitted.
- `next-pin.test.ts` — rewritten from a frozen `16.3.3` literal to the durable rules: both
  Next packages exact-pinned and equal, and never below the patched security floor. A frozen
  literal turns every security bump into a red test that invites a revert.

## Code-health fixes that came out of the audit

- `client.ts` was 576 lines against a 520-line exception. Split into `errors.ts` and
  `session-refresh.ts` → **472 lines**, inside the base cap. The exception is now the base cap.
- The module-size guard recorded two more violations that already existed at HEAD
  (`documents.ts` 883, `workspaces.ts` 594). Both are now pinned at their current size so they
  cannot grow, instead of the guard failing and being ignored.
- `eligible-path.test.ts` asserted one exact JSX spelling that had already been refactored
  (`choiceScope={pickerScope}` → `offerScope`/`pickerScope={pickerScope}`). Now asserts the
  behaviour.

## Verification

```text
edgequake_webui   vitest  2232 passed (264 files)   — was 4 failing
edgequake_webui   tsc     clean
edgequake_webui   eslint  clean (touched areas)
edgequake_webui   e2e     23 passed on next@16.4.3
mcp               build + 6 tests passed
sdks/typescript   build + 341 tests passed
sdks/python       625 tests passed
sdks/rust         cargo check clean
sdks/java|kotlin  mvn compile clean
website           astro build: 236 pages
edgequake (rust)  fmt clean, 1848 lib tests passed
```

Pre-existing and unrelated, verified by stashing this work: `e2e/spec157` fails 29/33 on both
Next 16.3.3 and 16.4.3.

## Lesson

The alert list overstates the risk and understates the real one. Two of the three genuine issues
here — `trust: true`, and the 576-line module — were invisible to Dependabot. Reading the actual
call sites found more than the severity badges did, and testing the "obvious" Mermaid finding
prevented a pointless breaking change.
