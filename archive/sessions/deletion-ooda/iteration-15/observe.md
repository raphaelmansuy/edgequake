# OODA Iteration 15 - Observe

**Mission Re-read**: specs/033-study-delete-document/003-study-document.md

## Focus Area: Edge Case Testing - Circular References

The mission requires:

> "Comprehensive Edge cases must implemented in tests to ensure reliability"

## What are Circular References?

In a knowledge graph, entities can have bidirectional relationships:

```
┌─────────┐        COLLABORATES_WITH        ┌─────────┐
│  ALICE  │ ───────────────────────────────> │   BOB   │
│         │ <─────────────────────────────── │         │
└─────────┘        COLLABORATES_WITH        └─────────┘
```

Or self-references:

```
┌─────────┐
│  ALICE  │ ──> KNOWS_SELF
│         │ <──
└─────────┘
```

## Potential Risks

1. **Infinite Loop**: Cascade deletion could loop infinitely
2. **Double Deletion**: Entity deleted twice causing panic
3. **Orphan Edge**: Edge to self becomes orphaned
4. **Reference Count Bug**: source_ids removal could be incorrect

## Current Deletion Flow

From previous iterations, cascade delete:

1. List all entities with source_ids containing deleted document
2. For each entity:
   - If source_ids becomes empty → delete entity
   - Otherwise → update entity (remove doc from source_ids)
3. Delete orphan edges (edges with empty source_ids)

## Questions to Answer

1. Does the deletion handle bidirectional relationships correctly?
2. Does the deletion handle self-referential entities?
3. Is there any risk of infinite recursion?

## Test Cases Needed

1. **Bidirectional relationship**: A → B and B → A, delete A's document
2. **Self-referential entity**: A → A, delete A's document
3. **Triangle**: A → B → C → A, delete A's document
4. **Shared bidirectional**: Both A and B in doc1, A → B and B → A
