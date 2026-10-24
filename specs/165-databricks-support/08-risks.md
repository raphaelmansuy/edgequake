# 08 — Risks

Parent: [README](README.md) · Previous: [07 Probe results](07-derisk-results.md) · Next: [09 Cross-reference](09-cross-ref.md)

Honest bounds. A row is a risk only where the probe or the code leaves it open.

| ID | Risk | Evidence | Mitigation in the plan |
|----|------|----------|------------------------|
| R1 | Postgres 16/17 pgvector 0.8.0 is under the CVE floor | Extension table, 16 Oct 2026. Floor is 0.8.2. Probe measured 0.8.6 only on PG 18 | WP-B refuses PG 17 |
| R2 | Pool dies after the OAuth password expires | `connect_role` uses one URL. TTL ≤ 3600 s. Default lifetime 1800 s | WP-A mints on each new connection |
| R3 | `gpt-oss` chat cannot be parsed | `content` was a list. Client field is `Option<String>` | WP-D pins string models |
| R4 | Wrong model name | Underscore UC name 404. Hyphenated `system.ai.gte-large-en` 200 | WP-D writes the service name, not the registry file name |
| R5 | Dimension mismatch | GTE returned 1024. Product default is often 1536 | Pin 1024 before the first document |
| R6 | AGE is impossible here | Allowed-list error. Not in `pg_available_extensions`. Role is not superuser | Use SPEC-164. Do not document Cypher on Lakebase |
| R7 | Migrate might still fail on a statement this study did not send | Boot path is read. Migrate was not executed | WP-C is the proof |
| R8 | App file limit and glibc | 10 MB doc. Binary not uploaded in this study | WP-E Volume + Ubuntu 22.04 build |
| R9 | pdfium in the binary | Server embeds pdfium. Volume avoids the 10 MB source limit. Loader still needs a compatible glibc | Build on glibc 2.35 or older |
| R10 | Scale-to-zero drops locks and `LISTEN` | Compatibility doc. Probe did not suspend under load. Probe suspend timeout is 300 s | Production `no_suspension`. Leave the probe timeout as it is |
| R11 | `lakebase_vector` looks available and does not load | `shared_preload_libraries` error | v1 uses pgvector HNSW, which succeeded |
| R12 | XLarge is still a small ingest host | 12 vCPU / 48 GB cap. No ingest benchmark on Apps | Measure on Large or XLarge in WP-E. Do not promise SPEC-001 rates |
| R13 | One app role sees every tenant | v1 identity is the service principal | Say so in operator docs. User header is a later spec |
| R14 | Probe token is in chat history | It was supplied in the task and used only as an environment variable | Rotate the token. It is not in git |
| R15 | Probe project costs money while ACTIVE | Endpoint was ACTIVE during the probe and suspends after 300 s idle | Left in place on purpose. Delete it in the workspace UI when the proof is done |
| R16 | GTE vectors are not normalized | Supported-models page for `gte-large-en` | Cosine still runs. Quality is not claimed |
| R17 | Legacy invocations still returned 200 | Gemma endpoint | Easy to wire the wrong URL. WP-D uses the Gateway base only |

## Claims this study does not make

- The EdgeQuake binary has booted on Lakebase.
- An App has served traffic.
- Every `system.ai` chat model returns a string.
- `lakebase_ann` is faster than HNSW on this data. It is not installed.
- Postgres 17 was tested.
