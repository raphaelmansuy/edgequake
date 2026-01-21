# Orient: Multi-Document KG Analysis

## Observations

After uploading second document:

- Total entities: 16 (up from 12)
- New entities: Michael Wong, Neo4j, PostgreSQL, Redis
- Total relationships: 13 (including cross-document)

## Entity Deduplication

Common entities between documents:

- Sarah Chen ✅ (deduplicated)
- EdgeQuake Labs ✅ (deduplicated)
- Google ✅ (deduplicated)
- OpenAI ✅ (deduplicated)
- Microsoft ✅ (deduplicated)
- San Francisco ✅ (deduplicated)
- LightRAG ✅ (deduplicated)
- GraphRAG ✅ (deduplicated)

## Cross-Document Relationships

New relationships from doc 2:

- Sarah Chen WORKS_ON GraphRAG
- Michael Wong PREVIOUSLY_WORKED_AT Google
- Michael Wong WORKED_ON Neo4j
- EdgeQuake Labs SUPPORTS PostgreSQL
- EdgeQuake Labs PLAN_TO_SUPPORT Redis
- LightRAG BUILDS_ON GraphRAG

## Query Results

Query: "Who works at EdgeQuake Labs and what have they published?"
Answer: "Sarah Chen works at EdgeQuake Labs. She developed the LightRAG paper."

✅ Correct aggregation from both documents
