# 00 — Why (SPEC-146)

## Five WHYs

### WHY-1 — Why do workspace walls leak?

Because GraphRAG treats the **workspace** as one corpus. Hybrid/local/global hops follow shared entity hubs. A Public viewer who never opened Doc B still receives Doc B’s facts when Entity X appears in both Doc A (Public) and Doc B (Secret).

NIST SP 800-162 requires subject + resource + environment attributes. Workspace ID alone is a coarse resource attribute — necessary, not sufficient for mixed-classification corpora.

### WHY-2 — Why isn’t coarse RBAC enough?

Because roles answer *capability* (“may this principal execute query?”), not *resource selection* (“which documents may enter ANN / hops / citations?”). Clearance × project × export-control × need-to-know cannot be encoded as `admin` / `user` / `readonly` alone.

Today EdgeQuake has **three diverging vocabularies** (global user role, membership role, WebUI labels) and a `Permission` enum that is **unused at HTTP handlers**. Capability RBAC is incomplete; document ABAC is absent.

### WHY-3 — Why can’t we redact after the LLM?

Because leakage happens **before** generation:

1. Typed ANN (`chunk_embeddings` / fleet) filters `workspace_id` only — unauthorized embeddings enter the candidate set.  
2. Graph expansion pulls unauthorized edge descriptions and `source_ids`.  
3. Caches and MCP `ret_*` can replay a richer context to a weaker principal.  
4. Titles leak via list, autocomplete, citations, and `doc="Title"` prompt headers.

Post-filter / post-LLM redaction cannot close ANN side channels, degree rankings, or cache hits. Fail-closed retrieval must **pre-filter**.

### WHY-4 — Why are workers and MCP dangerous?

Because they are **confused deputies**. An ingestion worker that inherits workspace-admin can query Secret docs. An MCP tool that inherits the gateway principal can fetch a `ret_*` minted for another user. API keys today collapse to Admin/User without document scope. Master keys must be break-glass, never silent all-doc Mix.

### WHY-5 — Why tag at upload?

Because labels that appear only after indexing create a searchable window of unlabeled (or mis-labeled) content. Security attributes must mint at **document admit** — same moment as `track_id` — or the document is quarantined (`labeling_failed`) and never enters ANN/citations.

## Causal ASCII

```ascii
  Mixed-classification corpus in one workspace
                 |
                 v
  Isolation = tenant_id + workspace_id only
                 |
     +-----------+-----------+------------------+
     |           |           |                  |
     v           v           v                  v
  List/GET    Typed ANN   Graph hops         Caches/MCP
  all titles  workspace   shared Entity X    no principal
              filter only  Secret+Public       in key
     |           |           |                  |
     +-----------+-----------+------------------+
                 |
                 v
  Unauthorized chunk / title / fact in LLM context
                 |
                 v
  NEED: PDP allow-set + ANN pre-filter + provenance
        hops + upload-time labels + bound caches
```

## Attack path (today)

```ascii
  User Public ──query Mix──► Entity ANN hits BRCA1 (from Doc A Public)
                                    |
                                    v
                           BFS depth=2 edges
                                    |
                    +---------------+---------------+
                    |                               |
                    v                               v
            Edge from Doc A                  Edge from Doc B Secret
            (authorized)                     (UNAUTHORIZED)
                    |                               |
                    +---------------+---------------+
                                    |
                                    v
                         KG→chunks hydrates Doc B
                         Citations name Doc B title
                         Answer leaks Secret facts
```

## Activation event

**Authorized neighborhood only.** A principal with workspace membership + `query.execute` receives chunks, edges, titles, and citations exclusively from the PDP allow-set. Unauthorized UUID enumeration returns 404. Zero allow-set looks like an empty corpus.

## Evidence (code-as-is)

| Gap | Anchor |
|-----|--------|
| No document ACL / classification columns | `documents.metadata` JSONB only — [03-code-as-is.md](03-code-as-is.md) |
| Typed ANN ignores `document_ids` | `typed_read.rs` workspace-only WHERE |
| Graph hops workspace-scoped | `graph_expand.rs` / BFS |
| Caches omit principal | SPEC-103 keys; MCP `RetrievalIdCache` |
| Role vocabulary drift | WebUI `developer`/`viewer` vs API `user`/`readonly` |
| `Permission` unused at HTTP | `edgequake-auth/src/rbac.rs` |

## Outcome

Document-level RBAC + ABAC that preserves GraphRAG quality for **authorized** neighborhoods while denying unauthorized chunks, edges, titles, residual embeddings, and side channels — without inventing Acc numbers.

## Cross-refs

- First principles → [00-first-principles.md](00-first-principles.md)  
- Findings → [01-finding-register.md](01-finding-register.md)  
- Threat model → [11-threat-model.md](11-threat-model.md)  
- Raw study → [zz-raw.md](zz-raw.md)  
