# SPEC-085 — Version Pins (audit-day SSOT)

> **Audit date**: 2026-07-24  
> **Laws**: LAW-15 (floor) · LAW-18 (prefer patched minor)  
> **Rule**: Implementers must re-query registries at execute time; this file records what was verified when the pack was written.

---

## Security floor vs latest (high-signal packages)

| Package | LAW-15 floor | Latest verified 2026-07-24 | Implement with |
|---------|--------------|----------------------------|----------------|
| `vitest` | 3.2.6 (3.x) / 4.1.0 (4.x) | 4.1.10 | ts-sdk: `^3.2.6` or align `^4.1.10` |
| `next` | **16.2.11** | 16.2.11 | exact `16.2.11` |
| `astro` | **7.1.0** | 7.1.3 | `^7.1.3` |
| `axios` | 1.18.0 | 1.18.1 | `^1.18.1` |
| `dompurify` | 3.4.12 | 3.4.12 | `^3.4.12` |
| `hono` | 4.12.27 | 4.12.31 | `>=4.12.27` (prefer latest 4.12.x) |
| `@hono/node-server` | 2.0.5 | (override) | `>=2.0.5` |
| `jackson-databind` | **2.18.9** | 2.18.9 (2.18 line); Central also has 2.22.1 | **`2.18.9`** (do not jump to 2.22 without SDK retest) |
| `jsonwebtoken` | 10.3.0 | 10.4.0 | `10.3`+ / resolved 10.4.0 OK |
| `opentelemetry_sdk` | 0.32.1 | 0.32.1 | `0.32` → resolve ≥0.32.1 |
| `aws-lc-sys` | 0.39.0 | 0.43.0 | `cargo update` (≥0.39) |
| `postcss` | 8.5.12 | 8.5.22 | `>=8.5.12` |
| `sharp` | 0.35.0 | 0.35.3 | `>=0.35.0` |
| `js-yaml` | 4.3.0 | 5.2.2 | `>=4.3.0` **stay on 4.x** unless parents need 5 |
| `vite` | 6.4.3 / 7.3.5 | 8.1.5 | line-aware floors only |
| `form-data` | 4.0.6 | 4.0.6 | `>=4.0.6` |
| `fast-uri` | 3.1.4 | (override) | `>=3.1.4` |
| `esbuild` | 0.28.1 | (override) | `>=0.28.1` |
| `@babel/core` | 7.29.6 | (override) | `>=7.29.6` |

---

## Re-query commands

```bash
npm view next version
npm view axios version
npm view dompurify version
npm view astro version
npm view hono version
npm view vitest version
curl -sL -A 'edgequake' https://crates.io/api/v1/crates/jsonwebtoken | jq -r .crate.max_stable_version
gh api /advisories/GHSA-m99w-x7hq-7vfj --jq .vulnerabilities
```

If a newer patch lands on the **same minor line** and still satisfies LAW-15, prefer it. If only a new **major** exists above the floor, keep the floor unless a dedicated migration wave is opened.
