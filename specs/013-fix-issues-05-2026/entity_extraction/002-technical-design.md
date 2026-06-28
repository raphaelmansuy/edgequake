# Technical design — `entity_types_strict`

## Single source of truth (pipeline)

`edgequake_pipeline::prompts::EntityExtractionSchema`:

```rust
pub struct EntityExtractionSchema {
    pub types: Vec<String>,
    pub strict: bool,
}
```

Built from workspace metadata via `EntityExtractionSchema::from_workspace_metadata(&metadata)`.

| Layer | Responsibility |
|-------|----------------|
| `entity_type_policy.rs` | Schema resolution, `enforce_entity_type`, JSON prompt section |
| `entity_extraction.rs` | SOTA system prompt strict vs permissive wording |
| `extractor/llm.rs` | JSON extractor uses schema for prompt + enforcement |
| `extractor/sota.rs` | SOTA uses schema for prompts + enforcement |
| `workspace_resolver.rs` | `with_entity_schema(schema)` on `LLMExtractor` |

## Metadata

```json
{
  "entity_types": ["PERSON", "ORGANIZATION", "OTHER"],
  "entity_types_strict": false
}
```

- Absent `entity_types_strict` ⇒ `strict = true`
- `entity_types_strict: false` stored explicitly
- `entity_types_strict: true` on update ⇒ key **removed** (canonical default)

## API surface

`WorkspaceResponse` / `UpdateWorkspaceApiRequest` / `CreateWorkspaceApiRequest`:

```typescript
entity_types_strict?: boolean;
```

Core `UpdateWorkspaceRequest.entity_types_strict: Option<bool>`.

## UI

`EntityTypeSelector` extended:

- `strictLimit: boolean`
- `onStrictLimitChange: (v: boolean) => void`
- Checkbox `data-testid="entity-types-strict-checkbox"`

Workspace page:

- State `selectedEntityTypesStrict`, init from `workspace.entity_types_strict ?? true`
- Save includes `entity_types_strict`
- View mode: badge “Strict limit: On/Off”

## Prompt text (JSON extractor)

**Strict:**

> Use ONLY these types exactly as written … If nothing fits, use OTHER when listed, otherwise CONCEPT.

**Permissive:**

> Prefer these types when they apply … You may use additional type labels … Do not use OTHER as a catch-all for unrelated entities.

## Enforcement

```rust
pub fn enforce_entity_type(raw: &str, schema: &EntityExtractionSchema) -> (String, bool)
```

- `schema.strict == false` && no allow-list match ⇒ `(normalize(raw), false)` — no fallback remap
- `schema.strict == true` ⇒ existing #217 behavior
