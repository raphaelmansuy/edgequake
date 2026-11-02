---
title: "Dependency remediation — November 2026"
description: "How 243 Dependabot advisories fell to 3 low, including a real KaTeX XSS sink, and what remains unfixed and why."
---

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

## Addendum — the release that shipped this work (v0.39.0)

Two corrections found while cutting v0.39.0. Both are recorded here because the sections above
were written from the alert list and from a test run, and neither caught them.

### The `>=4.0.33` override crossed a major and broke the app

The `@ai-sdk/provider-utils` override was written as `>=4.0.33`, with no ceiling. pnpm read that
literally and resolved **5.0.72**. `@ai-sdk/gateway@3.0.13` — reached through `ai@6` inside
`@scalar/api-reference`, which is the in-app API explorer — pins `provider-utils` at `^4.0.5` and
imports `createProviderToolFactoryWithOutputSchema`. That export exists in 4.0.5 and 4.0.63 and
does not exist in 5.0.72, so **every** page failed with a Next.js "Build Error: Export
createProviderToolFactoryWithOutputSchema doesn't exist in target module".

This was already broken on `edgequake-main`; its lockfile binds `gateway@3.0.13` to
`provider-utils@5.0.72`. It stayed invisible because the two `test-quality-gates` runs after the
security merge were **cancelled**, so no green build contradicted it, and the local e2e gate
retried into 366 timeouts that read as flakiness rather than as a single build failure.

The fix bounds the override to `>=4.0.33 <5`. Audit numbers are unchanged by it: `--prod` is the
same single low advisory, and the dev-inclusive count is the same 26 as `main`. A `>=` override
without a ceiling is the lesson — it silently crosses majors.

### The 1 low is KaTeX, and it is in the path this pack already closed

`pnpm audit --prod` for `edgequake_webui` reports exactly one advisory: `katex`, "existing
prototype pollution can bypass `trust` restrictions". The KaTeX fix above set `trust: false`, so
the bypass has no sink to reach. The remaining upstream fix is a `katex` bump that `mermaid` owns;
pinning it is tracked, not urgent.

Dev-only advisories remain at 26 (13 high, 12 moderate, 1 low) across `brace-expansion`,
`browserslist`, `js-yaml`, `undici` (via `jsdom`), `ajv`, `minimatch`, `@humanfs/node`, `vitest`
and `@vitest/mocker`. None ship: they are the test runner, the bundler and its glob machinery.

### Verification at v0.39.0

```text
edgequake_webui   vitest  2235 passed (265 files)
edgequake_webui   tsc     clean (tsconfig.release.json)
edgequake_webui   next build  clean, all routes emitted
edgequake_webui   pnpm audit --prod   1 low (katex)
edgequake (rust)  make release-gates  passed (fmt, clippy per crate + optional features,
                                      1849 lib tests, proofs, version parity, OpenAPI parity,
                                      checksums, epochs, schema docs)
edgequake (rust)  cargo doc -D warnings  clean
edgequake (rust)  cargo clippy -p edgequake-storage --features qdrant,neo4j,sqlite  clean
edgequake (rust)  e2e_spec181_tenancy 10 passed
edgequake (rust)  e2e_dal_provider_composition 2 passed, e2e_tenant_authorization 2 passed
                  (both on a freshly migrated database)
edgequake (docker) builder stage compiles: edgequake-api v0.39.0, 6m46s
edgequake (webui) e2e/spec180 19 passed   e2e/spec181 23 passed
website           astro build: 238 pages
```

Three more defects surfaced only once the tag existed and CI ran on the merge. They are recorded
here because each one shows how a gate can pass while the thing it guards is broken:

1. **`build_meta.rs` was not in the Docker build context.** Both `build.rs` files `include!` it,
   but the API Dockerfile copied only `crates/`, `src/`, `benches/`, `examples/` and
   `models.toml`. Both arches died at ~250s with `couldn't find file`. Every local
   `cargo build` passed because the file is on disk. `7aa324047` added the include after v0.38.4
   without touching the Dockerfile, so no release since then could produce an image.
   `check_docker_api_context.sh` now resolves every `build.rs` `include!` and fails if the target
   is not `COPY`ed.

2. **rustdoc refused a public doc linking a private item** in `services/capabilities.rs`, and CI
   builds docs with `-D warnings`. A one-line code-font fix, but it had been red on `main`
   unnoticed because the docs job is not on the branch anyone watches.

3. **The Qdrant adapter stopped compiling.** SPEC-180 added `kinds` to `MetadataFilter` and the
   adapter's test constructed the struct by hand. `qdrant` is an optional feature, so
   `cargo clippy --workspace` never compiled that module — not locally, not in any release gate.
   Fixed, and `release_gates.sh` now clippies `edgequake-storage` with
   `qdrant,neo4j,sqlite` so optional-feature code has to compile.

The common thread: **a gate that builds less than ships is not a gate.** Default-feature clippy,
a Docker context that differs from the working tree, and a docs job on an unwatched branch each
described a smaller system than the one being released.

## Lesson

The alert list overstates the risk and understates the real one. Two of the three genuine issues
here — `trust: true`, and the 576-line module — were invisible to Dependabot. Reading the actual
call sites found more than the severity badges did, and testing the "obvious" Mermaid finding
prevented a pointless breaking change.

The addendum adds a third instance of the same pattern: the override that *fixed* an advisory
broke the build, and no alert could have said so. A dependency override is a claim about an
interval; write the interval, not a lower bound.
