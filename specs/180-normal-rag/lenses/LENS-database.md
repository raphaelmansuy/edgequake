# Lens — Database

Parent: [README](../README.md) · Spec: [06-data-and-db-contract](../06-data-and-db-contract.md)

## View

The profile is a column with a check and a trigger, not a JSON key, because immutability in application code has already failed for embedding dimensions (advisory only, [F-180-20](../04-code-findings.md)). JSON remains for model names that rebuild is allowed to change.

BM25 statistics are tables EdgeQuake owns. The image does not gain a BM25 extension. `df` updates take a workspace row lock so two ingests cannot lose increments. Delete adjusts `df` in the same transaction as the chunk delete, before cascade would otherwise drop postings and leave `df` high.

`doc_kind` is generated from `content_type` so writers cannot store a kind that disagrees with the MIME type.

## Risks

| Risk | What we do |
|------|------------|
| `ADD COLUMN` rewrite on a large `workspaces` | Default on PostgreSQL 11+ is a catalog-only change. The migration comment says so. |
| HNSW build on 3072 locks writes longer than 768 | Follow the lock style of migration 132. Document duration in the spike if a staging database is large. Empty `chunk_embeddings` at those widths makes the build cheap on existing installs. |
| RLS missing on new tables | Same policy as `chunks`. `T-180-rls-terms`. |
| English generated `content_tsv` used for French | Normal BM25 reads `chunk_terms`. OR candidate generation for non-english regconfig does not use the generated column. |

## Acceptance

- Migration 225 is expand-only, registered, checksummed, `compat_serve_max = 225`.
- Trigger test updates the column and expects an exception.
- No change to migration 136.

## Laws owned

LAW-180-1, LAW-180-5, LAW-180-7, LAW-180-10, LAW-180-12.
