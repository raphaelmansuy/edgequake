# Iteration 09: Observe - Entity Deduplication & Normalization

## Topic: Entity Deduplication and Knowledge Graph Consistency

### Codebase Research Results

#### The Core Problem (normalizer.rs)

```
Without normalization, same entity becomes multiple nodes:
- "John Doe" (chunk 1)
- "john doe" (chunk 2)
- "JOHN DOE" (chunk 3)
- "The John Doe" (chunk 4)

Result: 4 nodes instead of 1
        Graph fragmentation
        Lost relationships
        Query failures
```

#### Normalization Rules (normalizer.rs)

```rust
/// Applies the following transformations:
/// - Trims whitespace
/// - Removes common prefixes (The, A, An)
/// - Removes possessive suffixes ('s)
/// - Converts to title case
/// - Replaces spaces with underscores
/// - Converts to uppercase
```

Examples:

- "John Doe" → "JOHN_DOE"
- "the company" → "COMPANY"
- " Sarah Chen " → "SARAH_CHEN"
- "John's Project" → "JOHNS_PROJECT"

#### Merger Logic (merger.rs)

```rust
async fn merge_entity(&self, entity: ExtractedEntity) -> Result<bool> {
    let entity_key = normalize_entity_name(&entity.name);

    // Check if entity exists
    let existing = self.graph_storage.get_node(&entity_key).await?;

    match existing {
        Some(mut node) => {
            // Update existing entity (merge descriptions)
            self.update_entity_node(&mut node, &entity).await?;
            self.graph_storage.upsert_node(&node.id, node.properties).await?;
            Ok(false)  // Not new
        }
        None => {
            // Create new entity
            let node = self.create_entity_node(&entity)?;
            self.graph_storage.upsert_node(&node.id, node.properties).await?;
            Ok(true)  // New
        }
    }
}
```

#### Description Merging Strategy

When entity exists, descriptions are merged:

1. **LLM Summarization** (if enabled): Intelligent merge via LLM
2. **Simple Concatenation** (fallback): Sentence-level deduplication

```rust
fn merge_descriptions(existing: &str, new: &str, max_length: usize) -> String {
    // Check if new content adds meaningful information
    let new_sentences: Vec<&str> = new.split(['.', '!', '?']).collect();

    for sentence in new_sentences {
        if !existing.contains(sentence) {
            additions.push(sentence);
        }
    }

    let combined = format!("{} {}", existing, additions.join(". "));
    truncate_description(&combined, max_length)
}
```

#### Merge Statistics (MergeStats)

```rust
pub struct MergeStats {
    pub entities_created: usize,
    pub entities_updated: usize,  // Deduplication happened
    pub relationships_created: usize,
    pub relationships_updated: usize,
    pub errors: usize,
}
```

Deduplication ratio = entities_updated / total_entities × 100

#### Production Metrics (from production_pipeline.rs)

```
Entity deduplication: 40% (20→12 nodes)
```

Real LLM extracts 20 entities from document, merge produces 12 unique nodes.

#### Business Rules Enforced

- **BR0005**: Entity description max 512 tokens
- **BR0007**: Lineage records append-only (source_id accumulation)
- **BR0008**: Entity names normalized (UPPERCASE_UNDERSCORE)

### Key Differentiators

1. **Consistent normalization**: Same entity always maps to same node
2. **Description aggregation**: Information accumulates, doesn't replace
3. **Source lineage**: Full history of where entity was mentioned
4. **LLM summarization**: Intelligent description merging (optional)
5. **Sentence-level dedup**: Avoid repeating same facts
