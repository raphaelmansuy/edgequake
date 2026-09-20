# WHY performance contracts must count work, bytes, and round trips

The right goal is no accidental quadratic application work, no per-row remote calls, and no unbounded request-path scans. A universal “best O(n)” guarantee would be false. This document specifies achievable bounds and the conditions under which they hold; it does not present new measured benchmarks.

## Cost model

- `n`: input records; `u`: distinct scoped logical keys; `S`: total input/serialized bytes including strings/provenance.
- `N`: stored records in the selected index; `d`: vector dimension; `k`: requested ordered results.
- `Vv`, `Ev`: vertices and edges actually visited; `r`: requested page rows; `B`: configured row batch cap; `M`: byte cap.
- `q`: number of batches after **both** row and byte splitting. Equal-size rows give approximately `ceil(n/B)`; arbitrary row sizes do not.
- `R`: network latency per request. Wall time includes CPU + server/index work + queue/lock wait + network latency; O(1) round trips do not mean O(1) execution.

Hash-map bounds below are expected/amortized and count key bytes. They are not worst-case adversarial guarantees. Use the standard randomized hasher, capped key length and request size; choose a balanced tree (O(n log n)) where strict adversarial bounds are necessary. Reading n arbitrary records already costs Omega(n + S).

## Required algorithms

| Operation | Algorithm / application bound | Storage / I/O qualification |
|---|---|---|
| Canonicalize + dedupe batch | One pass hash map plus first-seen order; expected O(n + S), O(u + retained bytes) memory | Reuse graph dedupe; complete scope/type/model keys; merge contributions explicitly |
| Hydrate k candidate IDs | One scoped batch fetch; map once; reconstruct positions; expected O(k + returned bytes) | Indexed lookup typically O(k log N + output); q bounded calls; no positional `Vec::find` loop |
| Relational writes | Validate/serialize each row once: O(n + S); stream B/M chunks | B-tree maintenance can be O(n log N); constant statements per batch; transactions add begin/commit calls |
| Scoped keyset pagination | Consume O(r + page bytes); persist complete sort tuple | With matching index, seek + page approximately O(log N + r); planner/visibility may add work |
| BFS expansion | Hash visited set + `VecDeque`; each logical vertex expanded once, edge processed bounded times: expected O(Vv + Ev + bytes) | Per-frontier batched adjacency plus edge pages; indexed seeks may add Vv log N; enforce node/edge/deadline caps |
| Induced-edge subgraph | Batch endpoint set, index-backed edge lookup then membership check | Cost proportional to matched/inspected adjacency, not promised O(k); high-degree seeds require cap |
| Exact vector scan | Dot/norm scoring O(Nd), retain k with heap O(N log k), memory O(k + d) | For resident fixtures, linear selection + sort only k gives O(Nd + N + k log k), with O(N) score memory |
| ANN search | Adapter/index-specific measured behavior; bounded candidate/scan budget | No universal O(log N), exact recall, or k-result guarantee; filtered population matters |
| Provenance union/subtract | Hash-set membership over affected contributions: expected O(total input lineage bytes) | Canonical contributions/reverse index avoid repeated full-document or whole-graph scans |
| Document cascade | Page indexed contributions once, group by scoped key, batch targeted deletes/replacements | Output-sensitive in affected rows; shared lineage may be large; no n full-store scans |
| Rebuild/reconciliation | One streaming pass per sorted partition + change replay; O(N + bytes) comparison | Index scans/seek overhead and index construction are additional; bounded page memory |
| Health | Constant small probe, timeout and cached schema capabilities | No exact COUNT, ANN warmup, DDL or dataset scan |

**Sorted output matters:** returning all n records sorted has a comparison lower bound Omega(n log n). The exact top-k scan can avoid sorting N scores, but sorting k returned scores remains O(k log k). If d or k grows with N, “linear” is no longer a complete description.

The existing memory adapter's full score sort (F08/E23) is an opportunity for a heap/selection improvement. Existing hash-based graph dedupe is already appropriate (E22); do not replace it merely to claim a new optimization. Backend-native SQL array operations remain useful even though their index work is not linear.

## Bounded graph traversal

```text
frontier = validated unique seeds; visited = set(seeds)
while frontier not empty and budgets remain:
    fetch incident-edge pages for frontier in bounded batches
    batch validate candidate edge/node scope, revision and visibility
    for each valid edge:
        record edge once by full typed identity
        enqueue unseen endpoint only while node/depth budgets permit
    frontier = next_frontier
return nodes, edges, truncation reason, resumable state if supported
```

Never scan every edge once per seed. Maintain edge dedupe separately from visited nodes to retain multiple relationship types and prevent repeated output through cycles. A max-node budget alone does not bound dense graph edges; node, edge and byte budgets are all required. Persist bounded continuation state server-side if a complete visited set cannot safely fit an external cursor.

## Bounded filtering and refill

Compile document/ID/modality filter sets once: expected O(filter bytes). Existing linear membership checks inside each candidate can otherwise become O(candidates * filter_count). Use one semantic predicate definition plus provider-specific translations and parity fixtures.

For stale projection candidates, increase the requested candidate window geometrically up to explicit `candidate_limit` and deadline, deduping IDs across retries. Geometric window sizes bound the sum of returned candidates by a constant multiple of the maximum window, excluding provider scan cost. Validate/hydrate in batches; return underfill reason at the cap. Mandatory scope is never a client-only post-filter.

pgvector approximate scans can underfill after filtering; iterative scans stop at configured limits. Preserve index-compatible distance ordering and the current typed-dimension casts. Measure exact and ANN paths separately; do not “fix” underfill by removing workspace filters. [Official pgvector guidance](../references/official-sources.md#s07-pgvector-filtered-search).

## Initial budgets to calibrate in W0

These are proposed defaults, not measured optimal settings. Keep current deployment budgets until the baseline validates a change.

| Resource | Initial policy |
|---|---|
| Graph write rows | Preserve current default 500; effective limit is minimum of adapter row cap and byte cap |
| Vector write rows | Preserve current default 1,000; reduce automatically for high dimensions/metadata bytes |
| Serialized request | Start 4 MiB soft batch target; hard cap is smaller of configured and provider maximum; a single oversized item fails explicitly |
| Graph request traversal | 1,000 nodes / 5,000 edges / 5 hops as initial product cap; admin export uses separate paged path |
| Search | Initial k cap 1,000; candidate cap 10,000; endpoint deadline overrides lower-level retries |
| Worker concurrency | Per role/per provider semaphore and bounded queue; preserve cluster-wide provider budgets where applicable |
| Retry | Up to 5 attempts and elapsed deadline, jittered backoff; quarantine permanent errors; no stacked retries across layers |
| Memory | O(batch bytes + bounded frontier + candidates); total includes concurrency multiplier |

Every limit is schema-derived, validated at startup, exposed in diagnostics and recorded in benchmark manifests. Reject arithmetic overflow and k cast truncation. Cancellation propagates to provider deadlines; remote cancellation can have unknown write outcome.

## Performance acceptance, not an unsupported guarantee

W0 records dataset hash, hardware, service image digests, index definitions, dimension, filter selectivity, concurrency, warm/cold state, retries, p50/p95/p99, allocations, bytes and physical request counts. Use n=1k/10k/100k for pure transformations and representative indexed datasets at 10k/100k/1m rows where practical. These are deterministic storage fixtures; no LLM calls are required.

Assert physical requests <= `c*q + c0` for batch commands, with constants declared by adapter (including model lookup/transaction control). Test B-1/B/B+1 and byte-limit boundaries. Instrument counts as well as time; time alone cannot prove complexity. Assert no per-record remote query in a batch loop.

Proposed gates: zero semantic divergence; exact fixtures agree within documented score tolerance; approximate recall@10 >= 0.95 on the versioned representative dataset (a release target, not a universal guarantee); no >10% p95 regression versus P0 on the same workload without a recorded tradeoff; bounded memory and overload recovery. Scaling ratios flag regressions but do not constitute a mathematical proof of Big-O. An adapter that misses budgets is uncertified until tuned or assigned a narrower supported profile.

Reference existing [088 complexity matrix](../../088-data-layer/complexity-matrix.md) and benchmark evidence; do not copy its vendor-specific estimates into universal port guarantees. [contract validation](../validation/edge-cases-and-contract-tests.md) owns the release tests.
