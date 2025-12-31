# Data Model and Schema Comparison

## 1. Overview

This document compares the data models and storage schemas between LightRAG (Python) and EdgeQuake (Rust). Both systems use similar high-level concepts but differ in implementation details, field naming, and lineage tracking.

---

## 2. Entity/Node Data Models

### LightRAG Entity Schema

LightRAG stores entities with a flexible property bag approach:

```python
# From operate.py - entity structure after extraction
entity_data = {
    "entity_name": "OPENAI",                    # Uppercase normalized name
    "entity_type": "ORGANIZATION",              # Entity category
    "description": "An AI research company...", # Accumulated description
    "source_id": "chunk1|chunk2|chunk3",        # Pipe-separated chunk IDs
}

# Node properties in graph storage
node_properties = {
    "entity_type": str,        # Entity category
    "description": str,        # Merged description
    "source_id": str,          # Pipe-separated source chunks
    "weight": float,           # Optional importance weight
}
```

**Key Characteristics:**

- Uses `entity_name` as primary key (normalized to uppercase)
- Properties stored as flat dict with string values
- Source tracking via pipe-separated string
- No explicit file path tracking
- No timestamp tracking in core schema

### EdgeQuake Entity Schema

EdgeQuake uses strongly-typed structs with comprehensive lineage:

```rust
// From extractor.rs - extraction output
pub struct ExtractedEntity {
    pub name: String,                      // Entity name (normalized)
    pub entity_type: String,               // e.g., "PERSON", "ORGANIZATION"
    pub description: String,               // Entity description
    pub importance: f32,                   // 0.0 to 1.0
    pub source_spans: Vec<String>,         // Original text excerpts
    pub embedding: Option<Vec<f32>>,       // Pre-computed embedding
    pub source_chunk_ids: Vec<String>,     // Source chunk references
    pub source_document_id: Option<String>,// Parent document ID
    pub source_file_path: Option<String>,  // Original file path
}

// From entity.rs - stored entity
pub struct GraphEntity {
    pub id: String,                        // Primary key (normalized name)
    pub entity_name: String,               // Display name (uppercase)
    pub entity_type: String,               // Entity category
    pub description: String,               // Aggregated description
    pub source_id: String,                 // Pipe-separated chunk IDs
    pub file_path: Option<String>,         // Source file paths
    pub created_at: DateTime<Utc>,         // Creation timestamp
}
```

**Key Characteristics:**

- Two-phase model: `ExtractedEntity` → `GraphEntity`
- Explicit importance scoring
- Source span preservation for citations
- File path tracking for provenance
- Timestamp tracking
- Pre-computed embeddings at extraction

### Entity Comparison Matrix

| Field         | LightRAG          | EdgeQuake            | Notes                     |
| ------------- | ----------------- | -------------------- | ------------------------- |
| Primary Key   | entity_name       | id                   | Both uppercase normalized |
| Display Name  | entity_name       | entity_name          | Identical                 |
| Type          | entity_type       | entity_type          | Identical                 |
| Description   | description       | description          | Both aggregate            |
| Source Chunks | source_id (pipe)  | source_id (pipe)     | Compatible format         |
| Importance    | Optional weight   | importance (0.0-1.0) | EdgeQuake always has      |
| Embedding     | Computed on query | Pre-stored           | EdgeQuake more efficient  |
| File Path     | ❌                | file_path            | EdgeQuake only            |
| Timestamp     | ❌                | created_at           | EdgeQuake only            |
| Source Spans  | ❌                | source_spans         | EdgeQuake only            |

---

## 3. Relationship/Edge Data Models

### LightRAG Relationship Schema

```python
# From operate.py - relationship extraction
relationship_data = {
    "src_id": "OPENAI",           # Source entity name
    "tgt_id": "GPT_4",            # Target entity name
    "weight": 1.0,                # Relationship strength
    "keywords": "develops, created, builds",  # Comma-separated
    "description": "OpenAI develops GPT-4...",
    "source_id": "chunk1|chunk2", # Pipe-separated source chunks
}

# Edge properties in graph storage
edge_properties = {
    "weight": float,
    "keywords": str,              # Comma-separated keywords
    "description": str,
    "source_id": str,
}
```

**Key Characteristics:**

- Uses (src_id, tgt_id) tuple as edge key
- Keywords as comma-separated string
- Weight represents relationship strength
- No explicit relationship type (uses description/keywords)

### EdgeQuake Relationship Schema

```rust
// From extractor.rs - extraction output
pub struct ExtractedRelationship {
    pub source: String,                     // Source entity name
    pub target: String,                     // Target entity name
    pub relation_type: String,              // Relationship type/verb
    pub description: String,                // Full description
    pub weight: f32,                        // 0.0 to 1.0
    pub keywords: Vec<String>,              // Typed keyword list
    pub embedding: Option<Vec<f32>>,        // Pre-computed embedding
    pub source_chunk_id: Option<String>,    // Source chunk
    pub source_document_id: Option<String>, // Parent document
    pub source_file_path: Option<String>,   // Original file
}

// Graph edge storage
pub struct GraphEdge {
    pub source: String,                     // Source node ID
    pub target: String,                     // Target node ID
    pub properties: HashMap<String, Value>, // Flexible properties
}
```

**Key Characteristics:**

- Explicit relation_type field
- Keywords as typed Vec<String>
- Pre-computed embeddings
- Full lineage tracking
- Flexible property bag for storage

### Relationship Comparison Matrix

| Field        | LightRAG         | EdgeQuake          | Notes               |
| ------------ | ---------------- | ------------------ | ------------------- |
| Source       | src_id           | source             | Same concept        |
| Target       | tgt_id           | target             | Same concept        |
| Type         | ❌ (in keywords) | relation_type      | EdgeQuake explicit  |
| Description  | description      | description        | Identical           |
| Weight       | weight           | weight             | Both 0.0-1.0        |
| Keywords     | Comma string     | Vec<String>        | EdgeQuake typed     |
| Embedding    | On query         | Pre-stored         | EdgeQuake efficient |
| Source Chunk | source_id (pipe) | source_chunk_id    | Format differs      |
| Document ID  | ❌               | source_document_id | EdgeQuake only      |
| File Path    | ❌               | source_file_path   | EdgeQuake only      |

---

## 4. Chunk/Document Data Models

### LightRAG Chunk Schema

```python
# TextChunkSchema from base.py
class TextChunkSchema(TypedDict):
    tokens: int                   # Token count
    content: str                  # Chunk text
    full_doc_id: str              # Parent document ID
    chunk_order_index: int        # Position in document

# Extended chunk in KV storage
chunk_data = {
    "tokens": 512,
    "content": "The quick brown fox...",
    "full_doc_id": "doc_abc123",
    "chunk_order_index": 0,
    # Optionally:
    "file_path": "path/to/file.txt",
}
```

### EdgeQuake Chunk Schema

```rust
// From chunker.rs
pub struct TextChunk {
    pub id: String,               // Unique chunk ID
    pub content: String,          // Chunk text content
    pub index: usize,             // Position in document
    pub start_offset: usize,      // Character offset start
    pub end_offset: usize,        // Character offset end
    pub start_line: usize,        // Line number start (1-based)
    pub end_line: usize,          // Line number end (1-based)
    pub token_count: usize,       // Approximate token count
    pub embedding: Option<Vec<f32>>, // Pre-computed embedding
}

// Lineage tracking
pub struct ChunkLineage {
    pub chunk_id: String,
    pub document_id: String,
    pub file_path: Option<String>,
    pub chunk_index: usize,
    pub start_offset: usize,
    pub end_offset: usize,
    pub start_line: usize,
    pub end_line: usize,
}
```

### Chunk Comparison Matrix

| Field        | LightRAG            | EdgeQuake        | Notes                |
| ------------ | ------------------- | ---------------- | -------------------- |
| ID           | full_doc_id + index | id               | EdgeQuake composite  |
| Content      | content             | content          | Identical            |
| Token Count  | tokens              | token_count      | Identical            |
| Document ID  | full_doc_id         | document_id      | Same concept         |
| Index        | chunk_order_index   | index            | Same concept         |
| Char Offsets | ❌                  | start/end_offset | EdgeQuake only       |
| Line Numbers | ❌                  | start/end_line   | EdgeQuake only       |
| Embedding    | On query            | Pre-stored       | EdgeQuake efficient  |
| File Path    | Optional            | In lineage       | EdgeQuake structured |

---

## 5. Storage Trait Comparison

### LightRAG Storage Abstractions

```python
# From base.py - three storage types

@dataclass
class BaseVectorStorage(StorageNameSpace, ABC):
    """Vector storage for similarity search"""
    async def query(self, query: str, top_k: int, ...) -> list[dict]
    async def upsert(self, data: dict[str, dict]) -> None
    async def delete(self, ids: list[str]) -> None
    async def get_by_id(self, id: str) -> dict | None
    async def get_by_ids(self, ids: list[str]) -> list[dict]

@dataclass
class BaseKVStorage(StorageNameSpace, ABC):
    """Key-value storage for documents and chunks"""
    async def get_by_id(self, id: str) -> dict | None
    async def get_by_ids(self, ids: list[str]) -> list[dict]
    async def filter_keys(self, keys: set[str]) -> set[str]
    async def upsert(self, data: dict[str, dict]) -> None
    async def delete(self, ids: list[str]) -> None

@dataclass
class BaseGraphStorage(StorageNameSpace, ABC):
    """Graph storage for entities and relationships"""
    async def has_node(self, node_id: str) -> bool
    async def has_edge(self, src: str, tgt: str) -> bool
    async def node_degree(self, node_id: str) -> int
    async def get_node(self, node_id: str) -> dict | None
    async def get_edge(self, src: str, tgt: str) -> dict | None
    async def upsert_node(self, node_id: str, properties: dict)
    async def upsert_edge(self, src: str, tgt: str, properties: dict)
```

### EdgeQuake Storage Traits

```rust
// From storage traits

#[async_trait]
pub trait VectorStorage: Send + Sync {
    async fn add_entities(&self, entities: Vec<VectorRecord>) -> Result<()>;
    async fn add_relationships(&self, relationships: Vec<VectorRecord>) -> Result<()>;
    async fn add_chunks(&self, chunks: Vec<VectorRecord>) -> Result<()>;
    async fn search_entities(&self, embedding: &[f32], top_k: usize) -> Result<Vec<VectorSearchResult>>;
    async fn search_relationships(&self, embedding: &[f32], top_k: usize) -> Result<Vec<VectorSearchResult>>;
    async fn search_chunks(&self, embedding: &[f32], top_k: usize) -> Result<Vec<VectorSearchResult>>;
}

#[async_trait]
pub trait KVStorage: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;
    async fn set(&self, key: &str, value: &[u8]) -> Result<()>;
    async fn delete(&self, key: &str) -> Result<()>;
    async fn exists(&self, key: &str) -> Result<bool>;
}

#[async_trait]
pub trait GraphStorage: Send + Sync {
    async fn add_node(&self, node: GraphNode) -> Result<()>;
    async fn add_edge(&self, edge: GraphEdge) -> Result<()>;
    async fn get_node(&self, id: &str) -> Result<Option<GraphNode>>;
    async fn get_edge(&self, src: &str, tgt: &str) -> Result<Option<GraphEdge>>;
    async fn get_neighbors(&self, id: &str, direction: Direction) -> Result<Vec<String>>;
    async fn get_subgraph(&self, node_ids: &[String], depth: u32) -> Result<KnowledgeGraph>;
}
```

### Storage Trait Comparison

| Capability          | LightRAG      | EdgeQuake      | Notes                   |
| ------------------- | ------------- | -------------- | ----------------------- |
| **Vector Storage**  |
| Entity search       | ✅            | ✅             | Both supported          |
| Relationship search | ✅            | ✅             | Both supported          |
| Chunk search        | ✅            | ✅             | Both supported          |
| Batch upsert        | ✅            | ✅             | Both supported          |
| **KV Storage**      |
| Get by ID           | ✅            | ✅             | Both supported          |
| Batch get           | ✅ get_by_ids | ❌ Single only | LightRAG more efficient |
| Filter keys         | ✅            | ❌             | LightRAG only           |
| **Graph Storage**   |
| Node operations     | ✅            | ✅             | Both supported          |
| Edge operations     | ✅            | ✅             | Both supported          |
| Node degree         | ✅            | ❌ Direct      | LightRAG has            |
| Edge degree         | ✅            | ❌             | LightRAG has            |
| Subgraph query      | ✅            | ✅             | Both supported          |
| Batch operations    | ✅            | ❌             | LightRAG has            |

---

## 6. Lineage and Provenance

### LightRAG Lineage

LightRAG tracks lineage through pipe-separated source_id fields:

```python
# Entity lineage
entity["source_id"] = "chunk1|chunk2|chunk3"

# Relationship lineage
relationship["source_id"] = "chunk1|chunk2"

# No structured lineage objects
# File paths tracked inconsistently
```

### EdgeQuake Lineage

EdgeQuake has dedicated lineage structures:

```rust
// From lineage.rs
pub struct ChunkLineage {
    pub chunk_id: String,
    pub document_id: String,
    pub file_path: Option<String>,
    pub chunk_index: usize,
    pub start_offset: usize,
    pub end_offset: usize,
    pub start_line: usize,
    pub end_line: usize,
}

pub struct EntitySource {
    pub entity_id: String,
    pub chunk_id: String,
    pub document_id: String,
    pub file_path: Option<String>,
    pub text_span: Option<String>,  // Original text excerpt
}

pub struct EntityLineage {
    pub entity_id: String,
    pub sources: Vec<EntitySource>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct RelationshipLineage {
    pub relationship_id: String,
    pub source_entity_id: String,
    pub target_entity_id: String,
    pub chunk_id: String,
    pub document_id: String,
    pub file_path: Option<String>,
}
```

### Lineage Comparison

| Capability         | LightRAG       | EdgeQuake                | Notes     |
| ------------------ | -------------- | ------------------------ | --------- |
| Chunk→Document     | ✅ full_doc_id | ✅ document_id           | Both      |
| Entity→Chunk       | ✅ source_id   | ✅ source_chunk_ids      | Both      |
| Entity→Document    | ❌             | ✅ source_document_id    | EdgeQuake |
| Entity→File        | ❌             | ✅ source_file_path      | EdgeQuake |
| Relationship→Chunk | ✅ source_id   | ✅ source_chunk_id       | Both      |
| Character offsets  | ❌             | ✅                       | EdgeQuake |
| Line numbers       | ❌             | ✅                       | EdgeQuake |
| Text spans         | ❌             | ✅ source_spans          | EdgeQuake |
| Timestamps         | ❌             | ✅ created_at/updated_at | EdgeQuake |
| Structured lineage | ❌             | ✅ Dedicated structs     | EdgeQuake |

---

## 7. Knowledge Graph Export Format

### LightRAG KnowledgeGraph

```python
# From types.py
class KnowledgeGraphNode(BaseModel):
    id: str
    labels: list[str]
    properties: dict[str, Any]

class KnowledgeGraphEdge(BaseModel):
    id: str
    type: Optional[str]
    source: str
    target: str
    properties: dict[str, Any]

class KnowledgeGraph(BaseModel):
    nodes: list[KnowledgeGraphNode]
    edges: list[KnowledgeGraphEdge]
    is_truncated: bool
```

### EdgeQuake KnowledgeGraph

```rust
// From graph.rs
pub struct GraphNode {
    pub id: String,
    pub properties: HashMap<String, serde_json::Value>,
}

pub struct GraphEdge {
    pub source: String,
    pub target: String,
    pub properties: HashMap<String, serde_json::Value>,
}

pub struct KnowledgeGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub is_truncated: bool,
}
```

### KG Export Comparison

| Field           | LightRAG            | EdgeQuake           | Notes                |
| --------------- | ------------------- | ------------------- | -------------------- |
| Node ID         | id                  | id                  | Identical            |
| Node Labels     | labels: list[str]   | ❌                  | LightRAG multi-label |
| Node Properties | properties: dict    | properties: HashMap | Same concept         |
| Edge ID         | id                  | ❌                  | LightRAG explicit ID |
| Edge Type       | type: Optional[str] | ❌                  | LightRAG explicit    |
| Edge Source     | source              | source              | Identical            |
| Edge Target     | target              | target              | Identical            |
| Edge Properties | properties: dict    | properties: HashMap | Same concept         |
| Truncation flag | is_truncated        | is_truncated        | Identical            |

---

## 8. Recommendations for EdgeQuake

### Priority 1: Schema Compatibility

1. **Add node labels support**

   - Add `labels: Vec<String>` to GraphNode
   - Enables multi-label entities

2. **Add edge ID and type**
   - Add `id: String` to GraphEdge
   - Add `edge_type: String` for explicit typing

### Priority 2: Batch Operations

1. **KV batch get**

   - Add `get_batch(keys: &[String])` method
   - Improves query performance

2. **Graph batch operations**
   - Add `get_nodes_batch(ids: &[String])`
   - Add `get_edges_batch(pairs: &[(String, String)])`

### Priority 3: Degree Operations

1. **Node degree**

   - Add `node_degree(id: &str) -> usize`
   - Used for ranking entities

2. **Edge degree**
   - Add `edge_degree(src: &str, tgt: &str) -> usize`
   - Sum of endpoint degrees

---

_Document Version: 1.0_
_Last Updated: 2025-01-01_
