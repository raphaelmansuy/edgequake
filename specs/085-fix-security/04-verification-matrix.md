# SPEC-085 — Verification Matrix

> **Laws**: LAW-19, LAW-20  
> **Rule**: An alert is FIXED only when version proof **and** surface gate pass.

---

## Gate catalog

| Gate ID | Surface | Commands / assertions |
|---------|---------|------------------------|
| `sec085_vitest_floor` | ts-sdk | `npm ls vitest` ≥3.2.6; `npm test`; UI server not required in CI |
| `sec085_next_16_2_11` | webui | `pnpm why next` =16.2.11+; `pnpm run typecheck && pnpm test && pnpm run build` |
| `sec085_axios_118` | webui | `pnpm why axios` ≥1.18.0; document list/upload still works |
| `sec085_dompurify_3412` | webui | `pnpm why dompurify` ≥3.4.12; markdown/HTML sanitize unit paths |
| `sec085_form_data_406` | webui | `pnpm why form-data` ≥4.0.6 |
| `sec085_astro_71` | website | `pnpm why astro` ≥7.1.0; `pnpm run build`; v7 checklist |
| `sec085_hono_41227` | mcp | `npm ls hono` ≥4.12.27; `@hono/node-server` ≥2.0.5; `npm test` |
| `sec085_fast_uri_314` | mcp/website | resolved ≥3.1.4 |
| `sec085_body_parser_230` | mcp | resolved ≥2.3.0 |
| `sec085_ip_address_1011` | mcp | resolved ≥10.1.1 |
| `sec085_jackson_2189` | java+kotlin | property ≥2.18.9; `mvn -q test` both |
| `sec085_jwt_103` | rust-core | no 9.x in `cargo tree -i jsonwebtoken`; `cargo test -p edgequake-auth` |
| `sec085_otel_0321` | rust-core | `opentelemetry_sdk` ≥0.32.1; observability tests with otel feature |
| `sec085_aws_lc_039` | rust-sdk | `cargo tree -p aws-lc-sys` ≥0.39.0; `cargo test` |
| `sec085_vite_line` | npm surfaces | vite 6.x ≥6.4.3; 7.x ≥7.3.5; (Astro 7 may introduce 8.x — prove no open GHSA) |
| `sec085_postcss_8512` | npm surfaces | all resolved postcss ≥8.5.12 |
| `sec085_sharp_035` | webui/website | sharp ≥0.35.0 |
| `sec085_js_yaml_430` | webui/website | js-yaml ≥4.3.0 |
| `sec085_transitive_sweep` | all npm | Dependabot open=0 for Wave-6 packages; `pnpm/npm` audit delta documented |

---

## Wave → gates

| Wave | Required gates |
|------|----------------|
| 0 | `sec085_vitest_floor`, `sec085_next_16_2_11` |
| 1 | `sec085_axios_118`, `sec085_dompurify_3412`, `sec085_form_data_406` |
| 2 | `sec085_astro_71` (+ website vite/svgo/sharp as pulled) |
| 3 | `sec085_hono_41227`, `sec085_fast_uri_314`, `sec085_body_parser_230`, `sec085_ip_address_1011` |
| 4 | `sec085_jackson_2189` |
| 5 | `sec085_jwt_103`, `sec085_otel_0321`, `sec085_aws_lc_039` |
| 6 | `sec085_vite_line`, `sec085_postcss_8512`, `sec085_sharp_035`, `sec085_js_yaml_430`, `sec085_transitive_sweep` |

---

## Version proof snippets

```bash
# npm / pnpm
pnpm why next axios dompurify postcss sharp vite
npm ls vitest hono @hono/node-server --all

# cargo
cargo tree -i jsonwebtoken
cargo tree -i opentelemetry_sdk
cargo tree -p aws-lc-sys -i

# maven
mvn -q dependency:tree -Dincludes=com.fasterxml.jackson.core:jackson-databind

# Dependabot
gh api repos/raphaelmansuy/edgequake/dependabot/alerts --paginate \
  --jq '[.[] | select(.state=="open") | .security_vulnerability.package.name] | group_by(.) | map({pkg:.[0], n:length})'
```

---

## Regression focus (product paths)

| Surface | Must not regress |
|---------|------------------|
| webui | Document upload, list/filter, query UI, markdown sanitize (DOMPurify) |
| website | Marketing/docs build + key pages render |
| mcp | Tool listing / HTTP adapter smoke |
| ts-sdk | Unit tests / package build |
| java/kotlin | Serialization round-trips |
| rust-core | JWT auth validate/encode; OTEL init when featured |
| rust-sdk | Build + TLS stack tests |

---

## Fail-closed rules

1. If override does not stick (`pnpm why` still old) → do **not** mark FIXED.  
2. If Astro 7 build fails on HTML strictness → fix templates; do **not** pin below 7.1.  
3. If OTEL 0.32 breaks `tracing-opentelemetry` → bump companion crates in the same PR (LAW-16).  
4. Never dismiss Critical/High without version proof.
