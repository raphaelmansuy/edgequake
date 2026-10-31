# Lens — RAG expert

Parent: [README](../README.md) · Spec: [02-research](../02-research.md), [09-retrieval-and-fusion](../09-retrieval-and-fusion.md)

## View

Normal retrieval is the baseline the papers actually recommend for lookup: BM25 plus dense, fused by RRF with k = 60, then an optional reranker (R-180-3). It is not a graph query with the graph arms set to zero. Those arms still call the graph ([F-180-10](../04-code-findings.md)).

`ts_rank_cd` is not the lexical leg. Fusing it with vectors has been measured as worse than vectors alone (R-180-4). The default fusion in this codebase, sparse-first, drops dense-only hits ([F-180-11](../04-code-findings.md)). RRF plus `min_score` 0.1 would drop every hit. Both are fixed for the normal path only. Graph workspaces keep their current fusion so SPEC-046 does not move under this spec.

Image and text share a space, so a text query can rank an image. Fusion is on ranks because cosine magnitudes are not comparable across a text query and an image document. Empty captions stay in the result. The answer model sees the caption, not the pixels, in this pack.

ColPali-style late interaction is out. Index-time page embeddings plus BM25 are the MIDR lesson we can ship on one vector column (R-180-6).

## Risks

| Risk | What we do |
|------|------------|
| Built-in BM25 is too weak | W0 compares it to `ts_rank` before W3 freezes SQL. |
| Stopword queries return nothing | Lexical leg is skipped. Dense remains. |
| Long questions AND-miss | Candidates are OR. |
| Reranker is a text cross-encoder | Image rows with empty content keep their RRF slot. |
| Someone "simplifies" by setting mix weight naive=1 | That still runs graph admission. The normal path does not enter that function. |

## Acceptance

- `T-180-rrf-keeps-hits` would fail on today's `sparse_retrieval.rs` line 188 if pointed at RRF scores.
- `T-180-or-query` finds a chunk that shares one rare term with a long question.
- `T-180-mode-default` does not call graph storage.
- Graph hybrid tests remain green.

## Laws owned

LAW-180-6, LAW-180-7, LAW-180-8, LAW-180-16. WHY-C.
