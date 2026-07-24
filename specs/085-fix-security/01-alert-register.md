# SPEC-085 — Alert Register
> **SSOT for open Dependabot packages**  
> **Audit**: 2026-07-24 · **Implementation**: 2026-07-24 · **29 FIXED / 0 PARTIAL** (was 133 open / 29 packages)  
> **Status legend**: OPEN | PARTIAL | FIXED | RETRACTED

DRY rule: one row per package. Alert numbers are listed; studies are package-level.

| Package | Max sev | GHSAs | Alerts | Floor | Current | Surfaces | Wave | Study | Status |
|---------|---------|-------|--------|-------|---------|----------|------|-------|--------|
| `vitest` | critical | 1 | #301 | ≥3.2.6 | ts-sdk lock (Critical); webui has vitest ^4.1.0 | ts-sdk | 0 | [PKG-vitest.md](packages/PKG-vitest.md) | FIXED |
| `next` | high | 9 | #390,#391,#392,#393,#394,#395,#396,#397+10 | ≥16.2.11 | 16.2.6 direct webui | webui | 0 | [PKG-next.md](packages/PKG-next.md) | FIXED |
| `com.fasterxml.jackson.core:jackson-databind` | high | 7 | #337,#338,#339,#340,#341,#342,#343,#344+6 | ≥2.18.9 | 2.18.3 Maven property | java,kotlin | 4 | [PKG-jackson-databind.md](packages/PKG-jackson-databind.md) | FIXED |
| `vite` | high | 5 | #162,#165,#168,#172,#173,#175,#182,#317+5 | ≥7.3.5 | webui 7.3.1 / website 6.4.1+7.3.5 / ts-sdk 6.4.1 | ts-sdk,website,webui | 6 | [PKG-vite.md](packages/PKG-vite.md) | FIXED |
| `axios` | high | 10 | #350,#352,#353,#354,#355,#363,#364,#367+2 | ≥1.18.0 | ^1.16.0 → resolved ~1.16.0 webui | webui | 1 | [PKG-axios.md](packages/PKG-axios.md) | FIXED |
| `astro` | high | 6 | #324,#325,#326,#356,#357,#358,#359,#361+1 | ≥7.1.0 | ^6.1.10 website | website | 2 | [PKG-astro.md](packages/PKG-astro.md) | FIXED |
| `hono` | high | 8 | #319,#320,#321,#322,#323,#379,#380,#381 | ≥4.12.27 | 4.12.23 transitive mcp | mcp | 3 | [PKG-hono-stack.md](packages/PKG-hono-stack.md) | FIXED |
| `fast-uri` | high | 4 | #240,#245,#372,#375,#382,#383 | ≥3.1.4 | 3.1.0 website / 3.1.2 mcp | mcp,website | 3 | [PKG-transitive-npm.md](packages/PKG-transitive-npm.md) | FIXED |
| `postcss` | high | 2 | #215,#217,#219,#408,#409,#410 | ≥8.5.12 | mixed ≤8.5.11 in some trees | ts-sdk,website,webui | 6 | [PKG-postcss.md](packages/PKG-postcss.md) | FIXED |
| `aws-lc-sys` | high | 5 | #80,#82,#84,#117,#119 | ≥0.39.0 | sdks/rust lock <0.39 | rust-sdk | 5 | [PKG-aws-lc-sys.md](packages/PKG-aws-lc-sys.md) | FIXED |
| `js-yaml` | high | 2 | #347,#349,#360,#366 | ≥4.3.0 | 4.1.1 / 4.2.0 mixed | website,webui | 6 | [PKG-js-yaml.md](packages/PKG-js-yaml.md) | FIXED |
| `brace-expansion` | high | 1 | #351,#365,#371 | ≥5.0.7 | 2.1.0 / 5.0.5 / 2.0.2 | ts-sdk,webui | 6 | [PKG-transitive-npm.md](packages/PKG-transitive-npm.md) | FIXED |
| `picomatch` | high | 2 | #132,#135,#137 | ≥4.0.4 | <4.0.4 in some trees | ts-sdk,website | 6 | [PKG-transitive-npm.md](packages/PKG-transitive-npm.md) | FIXED |
| `minimatch` | high | 1 | #76,#77 | ≥9.0.7 | needs ≥9.0.7 for alert floor | ts-sdk,webui | 6 | [PKG-transitive-npm.md](packages/PKG-transitive-npm.md) | FIXED |
| `rollup` | high | 1 | #70,#71 | ≥4.59.0 | webui override ≥4.59.0; mcp/ts-sdk may lag | mcp,ts-sdk | 6 | [PKG-transitive-npm.md](packages/PKG-transitive-npm.md) | FIXED |
| `sharp` | high | 1 | #374,#377 | ≥0.35.0 | 0.34.5 transitive | website,webui | 6 | [PKG-sharp.md](packages/PKG-sharp.md) | FIXED |
| `flatted` | high | 1 | #123 | ≥3.4.2 | webui lock | webui | 6 | [PKG-transitive-npm.md](packages/PKG-transitive-npm.md) | FIXED |
| `form-data` | high | 1 | #328 | ≥4.0.6 | 4.0.5 via axios | webui | 1 | [PKG-transitive-npm.md](packages/PKG-transitive-npm.md) | FIXED |
| `svgo` | high | 1 | #373 | ≥4.0.2 | 4.0.1 website | website | 6 | [PKG-transitive-npm.md](packages/PKG-transitive-npm.md) | FIXED |
| `dompurify` | medium | 9 | #310,#311,#312,#313,#314,#315,#316,#327+1 | ≥3.4.12 | ^3.4.0 webui | webui | 1 | [PKG-dompurify.md](packages/PKG-dompurify.md) | FIXED |
| `jsonwebtoken` | medium | 1 | #15,#348 | ≥10.3.0 | auth 10.3+/10.4; api/workspace 9.3 | rust-core | 5 | [PKG-jsonwebtoken.md](packages/PKG-jsonwebtoken.md) | FIXED |
| `opentelemetry_sdk` | medium | 1 | #345,#346 | ≥0.32.1 | 0.32.1 only (pdf2md 0.9.8 → llm 0.10.2) | rust-core | 5 | [PKG-opentelemetry-sdk.md](packages/PKG-opentelemetry-sdk.md) | FIXED |
| `@hono/node-server` | medium | 1 | #378 | ≥2.0.5 | 1.19.13 mcp | mcp | 3 | [PKG-hono-stack.md](packages/PKG-hono-stack.md) | FIXED |
| `ip-address` | medium | 1 | #235 | ≥10.1.1 | mcp transitive | mcp | 3 | [PKG-transitive-npm.md](packages/PKG-transitive-npm.md) | FIXED |
| `smol-toml` | medium | 1 | #127 | ≥1.6.1 | website | website | 6 | [PKG-transitive-npm.md](packages/PKG-transitive-npm.md) | FIXED |
| `yaml` | medium | 1 | #138 | ≥2.8.3 | website | website | 6 | [PKG-transitive-npm.md](packages/PKG-transitive-npm.md) | FIXED |
| `esbuild` | low | 1 | #302,#303,#304,#305 | ≥0.28.1 | 0.25–0.27 trees | mcp,ts-sdk,website,webui | 6 | [PKG-transitive-npm.md](packages/PKG-transitive-npm.md) | FIXED |
| `@babel/core` | low | 1 | #333,#334 | ≥7.29.6 | 7.29.0 | website,webui | 6 | [PKG-transitive-npm.md](packages/PKG-transitive-npm.md) | FIXED |
| `body-parser` | low | 1 | #370 | ≥2.3.0 | 2.2.2 mcp | mcp | 3 | [PKG-transitive-npm.md](packages/PKG-transitive-npm.md) | FIXED |

---

## Wave summary

| Wave | Packages | Intent |
|------|----------|--------|
| 0 | vitest, next | Critical + production SSR/runtime high |
| 1 | axios, dompurify, form-data | WebUI direct deps |
| 2 | astro | Website major (required floor ≥7.1) |
| 3 | hono, @hono/node-server, fast-uri, body-parser, ip-address | MCP stack |
| 4 | jackson-databind | Maven SDKs |
| 5 | jsonwebtoken, opentelemetry_sdk, aws-lc-sys | Rust |
| 6 | remaining npm transitives | Overrides + lock regen |

---

## GHSA index (package → advisories)

### `vitest`

| GHSA | Notes |
|------|-------|
| [GHSA-5xrq-8626-4rwp](https://github.com/advisories/GHSA-5xrq-8626-4rwp) | see Dependabot |

- When Vitest UI server is listening, arbitrary file can be read and executed

### `next`

| GHSA | Notes |
|------|-------|
| [GHSA-4633-3j49-mh5q](https://github.com/advisories/GHSA-4633-3j49-mh5q) | see Dependabot |
| [GHSA-4c39-4ccg-62r3](https://github.com/advisories/GHSA-4c39-4ccg-62r3) | see Dependabot |
| [GHSA-68g3-v927-f742](https://github.com/advisories/GHSA-68g3-v927-f742) | see Dependabot |
| [GHSA-6gpp-xcg3-4w24](https://github.com/advisories/GHSA-6gpp-xcg3-4w24) | see Dependabot |
| [GHSA-89xv-2m56-2m9x](https://github.com/advisories/GHSA-89xv-2m56-2m9x) | see Dependabot |
| [GHSA-955p-x3mx-jcvp](https://github.com/advisories/GHSA-955p-x3mx-jcvp) | see Dependabot |
| [GHSA-m99w-x7hq-7vfj](https://github.com/advisories/GHSA-m99w-x7hq-7vfj) | see Dependabot |
| [GHSA-p9j2-gv94-2wf4](https://github.com/advisories/GHSA-p9j2-gv94-2wf4) | see Dependabot |
| [GHSA-q8wf-6r8g-63ch](https://github.com/advisories/GHSA-q8wf-6r8g-63ch) | see Dependabot |

- Next.js: Cache confusion of response bodies for requests with bodies
- Next.js: Cache confusion of response bodies for requests with bodies containing invalid UTF-8 byte sequences
- Next.js: Denial of Service in App Router using Server Actions
- Next.js: Denial of Service in the Image Optimization API using SVGs
- Next.js: Middleware / Proxy bypass in App Router applications using Turbopack and single locale
- Next.js: Server-Side Request Forgery in Server Actions on custom servers
- Next.js: Server-Side Request Forgery in rewrites via attacker-controlled destination hostname
- Next.js: Unauthenticated disclosure of internal Server Function endpoints
- Next.js: Unbounded Server Action payload in Edge runtime

### `com.fasterxml.jackson.core:jackson-databind`

| GHSA | Notes |
|------|-------|
| [GHSA-3pjw-73gf-8qr5](https://github.com/advisories/GHSA-3pjw-73gf-8qr5) | see Dependabot |
| [GHSA-5gvw-p9qm-jgwh](https://github.com/advisories/GHSA-5gvw-p9qm-jgwh) | see Dependabot |
| [GHSA-5jmj-h7xm-6q6v](https://github.com/advisories/GHSA-5jmj-h7xm-6q6v) | see Dependabot |
| [GHSA-hgj6-7826-r7m5](https://github.com/advisories/GHSA-hgj6-7826-r7m5) | see Dependabot |
| [GHSA-j3rv-43j4-c7qm](https://github.com/advisories/GHSA-j3rv-43j4-c7qm) | see Dependabot |
| [GHSA-mhm7-754m-9p8w](https://github.com/advisories/GHSA-mhm7-754m-9p8w) | see Dependabot |
| [GHSA-rmj7-2vxq-3g9f](https://github.com/advisories/GHSA-rmj7-2vxq-3g9f) | see Dependabot |

- jackson-databind has a PolymorphicTypeValidator bypass via generic type parameters that allows arbitrary class instantiation
- jackson-databind has an array subtype allowlist bypass in BasicPolymorphicTypeValidator (allowIfSubTypeIsArray)
- jackson-databind has case-insensitive deserialization bypasses per-property @JsonIgnoreProperties
- jackson-databind: @JsonIgnore on a Record property is bypassed with a PropertyNamingStrategy
- jackson-databind: @JsonView ypassed for @JsonUnwrapped container properties on deserialization
- jackson-databind: InetSocketAddress deserialization triggers eager DNS resolution (SSRF)
- jackson-databind: `@JsonView` bypass for creator properties with `@JsonTypeInfo(include=As.EXTERNAL_PROPERTY)`

### `vite`

| GHSA | Notes |
|------|-------|
| [GHSA-4w7w-66w2-5vf9](https://github.com/advisories/GHSA-4w7w-66w2-5vf9) | see Dependabot |
| [GHSA-fx2h-pf6j-xcff](https://github.com/advisories/GHSA-fx2h-pf6j-xcff) | see Dependabot |
| [GHSA-p9ff-h696-f583](https://github.com/advisories/GHSA-p9ff-h696-f583) | see Dependabot |
| [GHSA-v2wj-q39q-566r](https://github.com/advisories/GHSA-v2wj-q39q-566r) | see Dependabot |
| [GHSA-v6wh-96g9-6wx3](https://github.com/advisories/GHSA-v6wh-96g9-6wx3) | see Dependabot |

- Vite Vulnerable to Arbitrary File Read via Vite Dev Server WebSocket
- Vite Vulnerable to Path Traversal in Optimized Deps `.map` Handling
- Vite: `server.fs.deny` bypassed with queries
- launch-editor: NTLMv2 hash disclosure via UNC path handling on Windows
- vite: `server.fs.deny` bypass on Windows alternate paths

### `axios`

| GHSA | Notes |
|------|-------|
| [GHSA-42h9-826w-cgv3](https://github.com/advisories/GHSA-42h9-826w-cgv3) | see Dependabot |
| [GHSA-7q8q-rj6j-mhjq](https://github.com/advisories/GHSA-7q8q-rj6j-mhjq) | see Dependabot |
| [GHSA-f4gw-2p7v-4548](https://github.com/advisories/GHSA-f4gw-2p7v-4548) | see Dependabot |
| [GHSA-gcfj-64vw-6mp9](https://github.com/advisories/GHSA-gcfj-64vw-6mp9) | see Dependabot |
| [GHSA-hcpx-6fm6-wx23](https://github.com/advisories/GHSA-hcpx-6fm6-wx23) | see Dependabot |
| [GHSA-jqh4-m9w3-8hp9](https://github.com/advisories/GHSA-jqh4-m9w3-8hp9) | see Dependabot |
| [GHSA-mmx7-hfxf-jppx](https://github.com/advisories/GHSA-mmx7-hfxf-jppx) | see Dependabot |
| [GHSA-mwf2-3pr3-8698](https://github.com/advisories/GHSA-mwf2-3pr3-8698) | see Dependabot |
| [GHSA-pmv8-rq9r-6j72](https://github.com/advisories/GHSA-pmv8-rq9r-6j72) | see Dependabot |
| [GHSA-xj6q-8x83-jv6g](https://github.com/advisories/GHSA-xj6q-8x83-jv6g) | see Dependabot |

- Axios Node HTTP adapter can use an inherited proxy after interceptor config cloning
- Axios form serializer maxDepth bypass via {} metatoken
- Axios: Deep formToJSON Key Recursion Can Cause Denial of Service
- Axios: Excessive recursion in formDataToJSON can cause denial of service
- Axios: Fetch adapter `ReadableStream` uploads bypass `maxBodyLength`
- Axios: HTTP/2 streamed uploads bypass `maxBodyLength`
- Axios: NO_PROXY bypass for 0.0.0.0 local addresses in axios
- Axios: Nested axios option objects can consume polluted prototype values
- Axios: Prototype pollution auth subfields can inject Basic auth
- Axios: Prototype pollution gadgets can alter axios request construction

### `astro`

| GHSA | Notes |
|------|-------|
| [GHSA-2pvr-wf23-7pc7](https://github.com/advisories/GHSA-2pvr-wf23-7pc7) | see Dependabot |
| [GHSA-4g3v-8h47-v7g6](https://github.com/advisories/GHSA-4g3v-8h47-v7g6) | see Dependabot |
| [GHSA-7pw4-f3q4-r2p2](https://github.com/advisories/GHSA-7pw4-f3q4-r2p2) | see Dependabot |
| [GHSA-8hv8-536x-4wqp](https://github.com/advisories/GHSA-8hv8-536x-4wqp) | see Dependabot |
| [GHSA-f48w-9m4c-m7f5](https://github.com/advisories/GHSA-f48w-9m4c-m7f5) | see Dependabot |
| [GHSA-jrpj-wcv7-9fh9](https://github.com/advisories/GHSA-jrpj-wcv7-9fh9) | see Dependabot |

- Astro: Cross-site scripting via unescaped transition:* directive values on hydrated islands
- Astro: Host header SSRF in prerendered error page fetch
- Astro: Reflected XSS via unescaped View Transition animation properties
- Astro: Reflected XSS via unescaped slot name
- Astro: XSS via Unescaped Attribute Names in Spread Props
- Astro: XSS via unescaped spread attribute names in renderHTMLElement (incomplete fix for CVE-2026-54298)

### `hono`

| GHSA | Notes |
|------|-------|
| [GHSA-88fw-hqm2-52qc](https://github.com/advisories/GHSA-88fw-hqm2-52qc) | see Dependabot |
| [GHSA-hvrm-45r6-mjfj](https://github.com/advisories/GHSA-hvrm-45r6-mjfj) | see Dependabot |
| [GHSA-j6c9-x7qj-28xf](https://github.com/advisories/GHSA-j6c9-x7qj-28xf) | see Dependabot |
| [GHSA-rv63-4mwf-qqc2](https://github.com/advisories/GHSA-rv63-4mwf-qqc2) | see Dependabot |
| [GHSA-w62v-xxxg-mg59](https://github.com/advisories/GHSA-w62v-xxxg-mg59) | see Dependabot |
| [GHSA-wgpf-jwqj-8h8p](https://github.com/advisories/GHSA-wgpf-jwqj-8h8p) | see Dependabot |
| [GHSA-wwfh-h76j-fc44](https://github.com/advisories/GHSA-wwfh-h76j-fc44) | see Dependabot |
| [GHSA-xgm2-5f3f-mvvc](https://github.com/advisories/GHSA-xgm2-5f3f-mvvc) | see Dependabot |

- Hono: API Gateway v1 adapter can drop a distinct repeated request header value during de-duplication
- Hono: Server-Side XSS via JSX Escaping Bypass in cx() Utility
- hono/jsx does not isolate context per request, leading to cross-request data disclosure
- hono: AWS Lambda adapter merges multiple `Set-Cookie` headers into one value, dropping cookies on ALB single-header and Lattice
- hono: Body Limit Middleware can be bypassed on AWS Lambda by understating `Content-Length`
- hono: CORS Middleware reflects any Origin with credentials when `origin` defaults to the wildcard
- hono: Lambda@Edge adapter keeps only the last value of a repeated request header, dropping the rest
- hono: Path traversal in `serve-static` on Windows via encoded backslash (`%5C`)

### `fast-uri`

| GHSA | Notes |
|------|-------|
| [GHSA-4c8g-83qw-93j6](https://github.com/advisories/GHSA-4c8g-83qw-93j6) | see Dependabot |
| [GHSA-q3j6-qgpj-74h6](https://github.com/advisories/GHSA-q3j6-qgpj-74h6) | see Dependabot |
| [GHSA-v2hh-gcrm-f6hx](https://github.com/advisories/GHSA-v2hh-gcrm-f6hx) | see Dependabot |
| [GHSA-v39h-62p7-jpjc](https://github.com/advisories/GHSA-v39h-62p7-jpjc) | see Dependabot |

- fast-uri vulnerable to host confusion via failed IDN canonicalization
- fast-uri vulnerable to host confusion via literal backslash authority delimiter
- fast-uri vulnerable to host confusion via percent-encoded authority delimiters
- fast-uri vulnerable to path traversal via percent-encoded dot segments

### `postcss`

| GHSA | Notes |
|------|-------|
| [GHSA-6g55-p6wh-862q](https://github.com/advisories/GHSA-6g55-p6wh-862q) | see Dependabot |
| [GHSA-qx2v-qp2m-jg93](https://github.com/advisories/GHSA-qx2v-qp2m-jg93) | see Dependabot |

- PostCSS has XSS via Unescaped </style> in its CSS Stringify Output
- PostCSS: Arbitrary file read and information disclosure via attacker-controlled sourceMappingURL in CSS comments

### `aws-lc-sys`

| GHSA | Notes |
|------|-------|
| [GHSA-394x-vwmw-crm3](https://github.com/advisories/GHSA-394x-vwmw-crm3) | see Dependabot |
| [GHSA-65p9-r9h6-22vj](https://github.com/advisories/GHSA-65p9-r9h6-22vj) | see Dependabot |
| [GHSA-9f94-5g5w-gf6r](https://github.com/advisories/GHSA-9f94-5g5w-gf6r) | see Dependabot |
| [GHSA-hfpc-8r3f-gw53](https://github.com/advisories/GHSA-hfpc-8r3f-gw53) | see Dependabot |
| [GHSA-vw5v-4f2q-w9xf](https://github.com/advisories/GHSA-vw5v-4f2q-w9xf) | see Dependabot |

- AWS-LC X.509 Name Constraints Bypass via Wildcard/Unicode CN
- AWS-LC has PKCS7_verify Certificate Chain Validation Bypass
- AWS-LC has PKCS7_verify Signature Validation Bypass
- AWS-LC has Timing Side-Channel in AES-CCM Tag Verification
- CRL Distribution Point Scope Check Logic Error in AWS-LC

### `js-yaml`

| GHSA | Notes |
|------|-------|
| [GHSA-52cp-r559-cp3m](https://github.com/advisories/GHSA-52cp-r559-cp3m) | see Dependabot |
| [GHSA-h67p-54hq-rp68](https://github.com/advisories/GHSA-h67p-54hq-rp68) | see Dependabot |

- JS-YAML: Quadratic-complexity DoS in merge key handling via repeated aliases
- js-yaml: YAML merge-key chains can force quadratic CPU consumption

### `brace-expansion`

| GHSA | Notes |
|------|-------|
| [GHSA-3jxr-9vmj-r5cp](https://github.com/advisories/GHSA-3jxr-9vmj-r5cp) | see Dependabot |

- brace-expansion: DoS via exponential-time expansion of consecutive non-expanding {} groups

### `picomatch`

| GHSA | Notes |
|------|-------|
| [GHSA-3v7f-55p6-f55p](https://github.com/advisories/GHSA-3v7f-55p6-f55p) | see Dependabot |
| [GHSA-c2c7-rcm5-vvqj](https://github.com/advisories/GHSA-c2c7-rcm5-vvqj) | see Dependabot |

- Picomatch has a ReDoS vulnerability via extglob quantifiers
- Picomatch: Method Injection in POSIX Character Classes causes incorrect Glob Matching

### `minimatch`

| GHSA | Notes |
|------|-------|
| [GHSA-7r86-cg39-jmmj](https://github.com/advisories/GHSA-7r86-cg39-jmmj) | see Dependabot |

- minimatch has ReDoS: matchOne() combinatorial backtracking via multiple non-adjacent GLOBSTAR segments

### `rollup`

| GHSA | Notes |
|------|-------|
| [GHSA-mw96-cpmx-2vgc](https://github.com/advisories/GHSA-mw96-cpmx-2vgc) | see Dependabot |

- Rollup 4 has Arbitrary File Write via Path Traversal

### `sharp`

| GHSA | Notes |
|------|-------|
| [GHSA-f88m-g3jw-g9cj](https://github.com/advisories/GHSA-f88m-g3jw-g9cj) | see Dependabot |

- sharp inherited vulnerabilities in libvips: CVE-2026-33327, CVE-2026-33328, CVE-2026-35590, CVE-2026-35591

### `flatted`

| GHSA | Notes |
|------|-------|
| [GHSA-rf6f-7fwh-wjgh](https://github.com/advisories/GHSA-rf6f-7fwh-wjgh) | see Dependabot |

- Prototype Pollution via parse() in NodeJS flatted

### `form-data`

| GHSA | Notes |
|------|-------|
| [GHSA-hmw2-7cc7-3qxx](https://github.com/advisories/GHSA-hmw2-7cc7-3qxx) | see Dependabot |

- form-data: CRLF injection in form-data via unescaped multipart field names and filenames

### `svgo`

| GHSA | Notes |
|------|-------|
| [GHSA-2p49-hgcm-8545](https://github.com/advisories/GHSA-2p49-hgcm-8545) | see Dependabot |

- SVGO removeScripts plugin leaves some executable scripts intact

### `dompurify`

| GHSA | Notes |
|------|-------|
| [GHSA-76mc-f452-cxcm](https://github.com/advisories/GHSA-76mc-f452-cxcm) | see Dependabot |
| [GHSA-c2j3-45gr-mqc4](https://github.com/advisories/GHSA-c2j3-45gr-mqc4) | see Dependabot |
| [GHSA-cmwh-pvxp-8882](https://github.com/advisories/GHSA-cmwh-pvxp-8882) | see Dependabot |
| [GHSA-gvmj-g25r-r7wr](https://github.com/advisories/GHSA-gvmj-g25r-r7wr) | see Dependabot |
| [GHSA-hpcv-96wg-7vj8](https://github.com/advisories/GHSA-hpcv-96wg-7vj8) | see Dependabot |
| [GHSA-r47g-fvhr-h676](https://github.com/advisories/GHSA-r47g-fvhr-h676) | see Dependabot |
| [GHSA-rp9w-3fw7-7cwq](https://github.com/advisories/GHSA-rp9w-3fw7-7cwq) | see Dependabot |
| [GHSA-vxr8-fq34-vvx9](https://github.com/advisories/GHSA-vxr8-fq34-vvx9) | see Dependabot |
| [GHSA-x4vx-rjvf-j5p4](https://github.com/advisories/GHSA-x4vx-rjvf-j5p4) | see Dependabot |

- DOMPurify IN_PLACE Sanitization Bypass via Attached Shadow Root Inside <template>.content
- DOMPurify: Cross-realm IN_PLACE sanitization leaves executable markup intact via realm-bound `instanceof` checks
- DOMPurify: Hook mutation of `data.allowedTags` / `data.allowedAttributes` permanently pollutes `DEFAULT_ALLOWED_TAGS` / `DEFAULT_ALLOWED_ATTR`
- DOMPurify: IN_PLACE mode preserves attributes of a clobbered root element, allowing XSS via attacker-controlled root DOM
- DOMPurify: Permanent `ALLOWED_ATTR` pollution via `setConfig()` bypassing the hook clone-guard (incomplete fix of the 3.4.7 hook-pollution patch)
- DOMPurify: SAFE_FOR_TEMPLATES bypass - template expressions survive sanitization inside <template> content when using DOM output modes
- DOMPurify: Trusted Types policy survives `clearConfig()` and can poison later `RETURN_TRUSTED_TYPE` output
- DOMPurify: `CUSTOM_ELEMENT_HANDLING` bypasses `afterSanitizeElements` for allowed custom elements.
- DOMPurify: `IN_PLACE` mode trusts attacker-controlled `nodeName` on live non-form nodes, allowing script retention and XSS via attacker-supplied DOM objects

### `jsonwebtoken`

| GHSA | Notes |
|------|-------|
| [GHSA-h395-gr6q-cpjc](https://github.com/advisories/GHSA-h395-gr6q-cpjc) | see Dependabot |

- jsonwebtoken has Type Confusion that leads to potential authorization bypass

### `opentelemetry_sdk`

| GHSA | Notes |
|------|-------|
| [GHSA-w9wp-h8wv-79jx](https://github.com/advisories/GHSA-w9wp-h8wv-79jx) | see Dependabot |

- opentelemetry_sdk has unbounded memory allocation in W3C Baggage propagation

### `@hono/node-server`

| GHSA | Notes |
|------|-------|
| [GHSA-frvp-7c67-39w9](https://github.com/advisories/GHSA-frvp-7c67-39w9) | see Dependabot |

- Node.js Adapter for Hono: Path traversal in `serve-static` on Windows via encoded backslash (`%5C`)

### `ip-address`

| GHSA | Notes |
|------|-------|
| [GHSA-v2v4-37r5-5v8g](https://github.com/advisories/GHSA-v2v4-37r5-5v8g) | see Dependabot |

- ip-address has XSS in Address6 HTML-emitting methods

### `smol-toml`

| GHSA | Notes |
|------|-------|
| [GHSA-v3rj-xjv7-4jmq](https://github.com/advisories/GHSA-v3rj-xjv7-4jmq) | see Dependabot |

- smol-toml: Denial of Service via TOML documents containing thousands of consecutive commented lines

### `yaml`

| GHSA | Notes |
|------|-------|
| [GHSA-48c2-rrv3-qjmp](https://github.com/advisories/GHSA-48c2-rrv3-qjmp) | see Dependabot |

- yaml is vulnerable to Stack Overflow via deeply nested YAML collections

### `esbuild`

| GHSA | Notes |
|------|-------|
| [GHSA-g7r4-m6w7-qqqr](https://github.com/advisories/GHSA-g7r4-m6w7-qqqr) | see Dependabot |

- esbuild allows arbitrary file read when running the development server on Windows

### `@babel/core`

| GHSA | Notes |
|------|-------|
| [GHSA-4x5r-pxfx-6jf8](https://github.com/advisories/GHSA-4x5r-pxfx-6jf8) | see Dependabot |

- @babel/core: Arbitrary File Read via sourceMappingURL Comment

### `body-parser`

| GHSA | Notes |
|------|-------|
| [GHSA-v422-hmwv-36x6](https://github.com/advisories/GHSA-v422-hmwv-36x6) | see Dependabot |

- body-parser vulnerable to denial of service when invalid limit value silently disables size enforcement

