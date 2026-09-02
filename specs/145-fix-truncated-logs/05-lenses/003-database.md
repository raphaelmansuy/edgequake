# Lens — Database Expert

## EdgeQuake Postgres

**No migration.** Observation Input/Output is not stored in EdgeQuake tables.
Traces leave via OTLP/HTTP or native ingestion only.

## Langfuse storage

Self-hosted / Cloud use ClickHouse (and related stores) with unbounded
`String` fields for observation I/O. Product 512-byte cuts are not a DB
constraint.

Langfuse may log oversized OTEL fields (~1 MiB) and reject enormous **request
bodies** (`LANGFUSE_OTEL_INGESTION_MAX_BODY_BYTES`, default 512 MiB) with
HTTP 413. EdgeQuake must not fail the user query on export 413 (LAW-124-4).

## Retention / GDPR

Bodies may contain workspace content. Retention and deletion are Langfuse
project / ops concerns. EdgeQuake redacts **secrets**, not all document text
(LAW-145-2). Spec fixtures contain no personal data (LAW-145-10).

## Cross-refs

- Architecture: [../04-target-architecture.md](../04-target-architecture.md)
- Edges: [../09-edge-cases.md](../09-edge-cases.md)
