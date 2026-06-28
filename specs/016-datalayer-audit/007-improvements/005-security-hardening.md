# Security Hardening

Security analysis of the data layer across the four mandated dimensions, with concrete
mitigations.

## S1 — 🟠 F8: Cypher built by string interpolation

**Surface:** all graph ops format the `node_id` and properties directly into the Cypher
string ([graph/mod.rs#L213+](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs#L213)),
escaped only by the hand-rolled `escape_cypher_string`
([helpers.rs#L233](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/helpers.rs#L233)):

```rust
s.replace('\\', "\\\\").replace('\'', "\\'").replace('"', "\\\"")
 .replace('\n', "\\n").replace('\r', "\\r").replace('\t', "\\t")
```

And the Cypher is wrapped in a `$$ … $$` dollar-quoted SQL string
([helpers.rs#L101](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/helpers.rs#L101)):

```rust
format!("SELECT * FROM cypher('{}', $$ {} $$) AS (a agtype)", self.graph_name, cypher)
```

**Why it's a risk:** entity names/descriptions originate from **LLM output over
arbitrary document content** — i.e. attacker-influenceable. Hand-rolled escaping is a
denylist; denylists are fragile. Two specific concerns:

1. **`$$` sequence in input** — if any property value contains `$$`, it can terminate
   the SQL dollar-quote early and inject SQL. `escape_cypher_string` does **not** neutralize
   `$$`.
2. **Unicode / alternate quote tricks** — escaping only handles ASCII `'` `"` `\` and
   whitespace; it does not consider Cypher's full lexical surface.

**Mitigations (in order of preference):**

1. **Parameterize** via AGE's `cypher(graph, $$ … $$, $params::agtype)` third argument so
   values never enter the query text (validate version support — see
   [`004-edge-cases-and-mitigations.md#sc1`](004-edge-cases-and-mitigations.md#sc1)).
2. If string-building must remain, use a **unique dollar-quote tag** (`$eq_2f9c$ … $eq_2f9c$`)
   generated per call and assert the tag does not occur in the payload (reject if it does).
3. Add **fuzz tests** feeding quotes, backslashes, `$$`, `*/`, newlines, and Unicode
   confusables through every graph write.
4. Run the DB role with **least privilege** (see S3) to bound blast radius.

## S2 — Workspace isolation

Isolation is by **table/graph name prefix** (`eq_{prefix}_vectors`, `eq_{prefix}_graph`).
This is strong for *separation* (a query can't accidentally read another workspace's
table) but:

- **`prefix` derivation must be injection-safe.** It is formatted into DDL/table names
  ([graph/mod.rs#L107](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/mod.rs#L107),
  [vector.rs `create_table`](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/vector.rs#L97)).
  **Mitigation:** validate `prefix` against `^[a-z0-9_]+$` at construction; reject
  anything else. (Confirm `table_prefix()` sanitizes — add a test.)
- No **row-level security**; isolation depends entirely on the app passing the right
  prefix. Acceptable for the current model; document the trust boundary.

## S3 — Database privileges & secrets

- Connection uses `CREATE EXTENSION` ([connection.rs#L124](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/connection.rs#L124))
  which requires elevated privileges. **Mitigation:** run extension setup once at
  provisioning with an admin role; run the *application* with a least-privilege role that
  can DML but not `CREATE EXTENSION`/superuser. `LOAD 'age'` requires the library to be
  allow-listed — document the operator requirement.
- **Secrets:** `connection_url()` embeds credentials; ensure it is never logged. Audit
  `tracing` calls in the adapters for accidental URL/credential logging.

## S4 — Denial of service

| Vector                           | Mechanism                           | Mitigation                     |
| -------------------------------- | ----------------------------------- | ------------------------------ |
| Unbounded traversal (F9)         | `[*1..depth]` no LIMIT on hub nodes | QW4: cap + depth clamp         |
| Huge `ef_search`                 | latency amplification               | QW3: clamp + `max_scan_tuples` |
| Oversized batch arrays (QW2/SC1) | memory/parse pressure               | chunk arrays to safe sizes     |
| Long transactions (SC2)          | lock/bloat                          | document-scoped txns           |
| Large chunk text in GIN (F5)     | write amplification                 | SC3 relocation                 |

## S5 — Input validation at boundaries

Per the project's security posture (OWASP), validate at the **system boundary** (API),
not deep in the adapter. Specifically: embedding dimension (already checked), `prefix`
shape (S2), `depth`/`top_k`/`ef` ranges (QW3/QW4). Avoid adding redundant validation
inside hot loops.

## Net security verdict

No critical (RCE/auth-bypass) issue found. The one **real** finding is **F8** (Cypher
string interpolation fed by LLM-influenced content with a `$$`-blind escaper) — rated
🟠 because it requires attacker-controlled document content reaching graph properties,
but it should be fixed via parameterization. Everything else is hardening/DoS bounding.
