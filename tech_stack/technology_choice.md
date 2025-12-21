# LightRAG Rust Rewrite: Technology Stack Architecture Decision Record (ADR)

**Date**: December 20, 2025  
**Status**: Approved  
**Decision Makers**: Senior Principal Rust Architect & Engineering Lead  
**Context**: Greenfield rewrite of LightRAG (Python) to Rust

---

## Executive Summary

This ADR documents the technology choices for rebuilding LightRAG—an advanced Retrieval-Augmented Generation framework with knowledge graph capabilities—using the Rust ecosystem as of December 2025. The selected stack prioritizes **type safety**, **performance**, **developer productivity**, and **long-term maintainability** while preserving all critical features of the Python implementation.

### Key Decisions

| Layer | Technology | Rationale |
|-------|-----------|-----------|
| **Language** | Rust 2021 Edition | Memory safety, concurrency, performance |
| **Async Runtime** | Tokio | Industry standard, mature ecosystem |
| **Web Framework** | Axum 0.8+ with OpenAPI/Swagger | Ergonomic, Tower ecosystem integration, API documentation |
| **Primary Databases** | PostgreSQL AGE + pgvector, SurrealDB, FalkorDB | Multi-model: Graph (AGE, FalkorDB), Vector (pgvector), Document |
| **Graph Databases** | PostgreSQL AGE (primary), FalkorDB, SurrealDB | OpenCypher support, production-ready |
| **Vector Search** | pgvector, Qdrant (supplementary) | Native PostgreSQL extension, production-ready |
| **LLM Client** | async-openai | Comprehensive OpenAI API coverage |
| **Text Processing** | tiktoken-rs + text-splitter | Token counting + semantic chunking |
| **Frontend** | Open WebUI | Production-ready LLM interface with RAG support |
| **Graph Visualization** | Cytoscape.js | Interactive graph rendering |
| **API Documentation** | utoipa (OpenAPI/Swagger) | Auto-generated REST API specs |
| **Error Handling** | thiserror + anyhow | Library + application patterns |
| **Observability** | tracing + tracing-subscriber | Structured logging |
| **Testing** | cargo-nextest | Fast parallel test execution |

---

## 1. Core Language: Rust (2021 Edition)

### Decision
Use **Rust 2021 Edition** as the implementation language.

### Rationale

#### Why Rust Over Python?

| Aspect | Python (Current) | Rust (Proposed) | Impact |
|--------|------------------|-----------------|---------|
| **Performance** | Interpreted, GIL limitations | Compiled, zero-cost abstractions | 10-100x faster execution |
| **Memory Safety** | Runtime errors, reference counting | Compile-time borrow checker | Eliminates entire class of bugs |
| **Concurrency** | asyncio + GIL contention | Fearless concurrency with Tokio | True parallelism |
| **Type Safety** | Dynamic typing, runtime checks | Static types, compile-time guarantees | Catch errors before deployment |
| **Dependencies** | Heavy (FastAPI, SQLAlchemy, etc.) | Lean, compiled binaries | Smaller deployment footprint |
| **Deployment** | Docker + Python runtime | Single static binary | Simplified operations |

#### Alignment with Requirements

- **Performance**: Document processing pipelines benefit from compiled efficiency
- **Concurrency**: Simultaneous entity extraction from multiple chunks without GIL
- **Reliability**: Type system prevents entire categories of runtime errors
- **Scalability**: Native async/await without interpreter overhead

### Trade-offs

**Advantages**:
- Superior runtime performance
- Compile-time error detection
- Zero-cost abstractions
- Excellent tooling (cargo, clippy, rustfmt)

**Disadvantages**:
- Steeper learning curve
- Longer compile times
- Smaller ecosystem than Python (but growing rapidly)

---

## 2. Async Runtime: Tokio

### Decision
Use **Tokio** as the async runtime foundation.

### Rationale

#### Why Tokio?

As of December 2025, Tokio is the **de facto standard** for async Rust:

- **Maturity**: 5+ years of production use, battle-tested at scale
- **Ecosystem**: 90%+ of async crates are Tokio-compatible
- **Features**:
  - Multi-threaded work-stealing scheduler
  - Async I/O (TCP, UDP, file system)
  - Timer facilities
  - Synchronization primitives
- **Integration**: Required by Axum, Hyper, and most HTTP clients
- **Performance**: Benchmarks show best-in-class async performance

#### Alternatives Considered

| Runtime | Pros | Cons | Verdict |
|---------|------|------|---------|
| **async-std** | Simpler API, `std`-like | Smaller ecosystem, slower development | ❌ Rejected |
| **smol** | Lightweight | Limited ecosystem, task-local variables missing | ❌ Rejected |
| **Tokio** | Industry standard, vast ecosystem | Slightly heavier | ✅ **Selected** |

### Alignment with Requirements

- **Async Operations**: All document ingestion and query operations are async
- **Concurrency**: Parallel entity extraction from chunks
- **HTTP Server**: Axum requires Tokio
- **LLM Clients**: async-openai built on Tokio

### Example Usage

```rust
#[tokio::main]
async fn main() {
    let rag = LightRAG::new(config).await.unwrap();
    rag.insert("document text").await.unwrap();
}
```

---

## 3. Web Framework: Axum 0.8+

### Decision
Use **Axum** for the REST API layer.

### Rationale

#### Why Axum?

**Technical Superiority** (as of Dec 2025):
- **Tower Integration**: Inherits entire Tower middleware ecosystem
- **Type Safety**: Extractors prevent runtime errors
- **Performance**: Built on Hyper, near-metal HTTP performance
- **Ergonomics**: Macro-free API, excellent error messages
- **Maintenance**: Maintained by Tokio project team

#### Comparison with Alternatives

| Framework | Performance | Ergonomics | Ecosystem | Verdict |
|-----------|-------------|-----------|-----------|---------|
| **Actix-web** | Excellent | Good | Large | ✅ Viable |
| **Axum** | Excellent | Excellent | Tower | ✅ **Selected** |
| **Rocket** | Good | Excellent | Medium | ❌ Async maturity |
| **Warp** | Excellent | Poor (filter chains) | Medium | ❌ Complex API |

**Why Axum Over Actix-web?**

While Actix-web was the leader in 2020-2022, by 2025:
- Axum's Tower integration provides more composable middleware
- Better type inference and error messages
- More idiomatic Rust (no actor model complexity)
- Tokio team backing ensures long-term support

### API Design Example

```rust
use axum::{Router, extract::{State, Json}, routing::post};

async fn insert_document(
    State(rag): State<Arc<LightRAG>>,
    Json(req): Json<InsertRequest>,
) -> Result<Json<InsertResponse>, ApiError> {
    let track_id = rag.ainsert(&req.content).await?;
    Ok(Json(InsertResponse { track_id }))
}

let app = Router::new()
    .route("/documents", post(insert_document))
    .with_state(Arc::new(rag));
```

### Alignment with Requirements

- **REST API**: Python FastAPI → Rust Axum
- **OpenAPI**: Via `utoipa` crate integration
- **Middleware**: Tower for auth, logging, rate limiting
- **State Management**: Type-safe `State` extractor

---

## 4. Primary Database: SurrealDB

### Decision
Use **SurrealDB** as the primary multi-model database for graph, document, and vector storage.

### Rationale

#### Why SurrealDB is Perfect for LightRAG

**The Problem**: Python LightRAG uses **12 separate storage instances**:
- KV Storage (4 instances): docs, chunks, status, cache
- Graph Storage (1 instance): knowledge graph
- Vector Storage (3 instances): chunks, entities, relationships

**The Solution**: SurrealDB consolidates this into **one database**:

| Python LightRAG Storage | SurrealDB Equivalent |
|-------------------------|---------------------|
| JsonKVStorage (full_docs) | Document table |
| JsonKVStorage (doc_status) | Document table (status field) |
| JsonKVStorage (text_chunks) | Chunk table |
| JsonKVStorage (llm_cache) | Cache table with TTL |
| NetworkXStorage (graph) | Graph relations (native) |
| NanoVectorDBStorage (chunks) | Vector embeddings (native) |
| NanoVectorDBStorage (entities) | Vector embeddings (native) |
| NanoVectorDBStorage (relations) | Vector embeddings (native) |

#### SurrealDB Advantages

**Multi-Model in One**:
```sql
-- Define a table with vector search
DEFINE TABLE entity SCHEMAFULL;
DEFINE FIELD name ON entity TYPE string;
DEFINE FIELD entity_type ON entity TYPE string;
DEFINE FIELD embedding ON entity TYPE array<float>;
DEFINE INDEX embedding_idx ON entity FIELDS embedding MTREE DIMENSION 1536;

-- Define graph relationships
DEFINE TABLE relationship SCHEMAFULL;
DEFINE FIELD in ON relationship TYPE record<entity>;
DEFINE FIELD out ON relationship TYPE record<entity>;
DEFINE FIELD description ON relationship TYPE string;

-- Vector similarity search + graph traversal
SELECT * FROM entity 
WHERE embedding <|1536|> $query_vector
FETCH ->relationship->entity;
```

**Native Rust SDK**: Async/await, type-safe queries
```rust
use surrealdb::{Surreal, engine::remote::ws::Ws};

let db = Surreal::new::<Ws>("localhost:8000").await?;
db.use_ns("lightrag").use_db("prod").await?;

// Type-safe queries
let entities: Vec<Entity> = db
    .query("SELECT * FROM entity WHERE name = $name")
    .bind(("name", "Rust"))
    .await?
    .take(0)?;
```

#### Comparison with Alternatives

| Database | Graph | Vector | Document | KV | Verdict |
|----------|-------|--------|----------|-----|---------|
| **Neo4j** | ✅ Excellent | ❌ Plugin only | ❌ No | ❌ No | ❌ Need 3+ DBs |
| **Qdrant** | ❌ No | ✅ Excellent | ❌ Limited | ❌ No | ❌ Need graph DB |
| **PostgreSQL + pgvector + AGE** | ⚠️ Via AGE | ⚠️ Via pgvector | ✅ Yes | ✅ Yes | ⚠️ Complex setup |
| **SurrealDB** | ✅ Native | ✅ Native | ✅ Native | ✅ Native | ✅ **Selected** |

### Alignment with Requirements

✅ **Knowledge Graph**: Native graph relations  
✅ **Vector Search**: Built-in MTREE/HNSW indices  
✅ **Document Storage**: Flexible schema  
✅ **Multi-Tenancy**: Database namespaces  
✅ **Async Operations**: Native async Rust SDK  
✅ **Performance**: Written in Rust, optimized  

### Migration Path

Python → Rust storage mapping:
```rust
// Python: self.full_docs[doc_id] = content
// Rust:
db.create(("document", doc_id))
    .content(Document { content, status: Status::Pending })
    .await?;

// Python: self.chunk_entity_relation_graph.add_node(entity_name)
// Rust:
db.create(("entity", entity_name))
    .content(Entity { name, entity_type, embedding })
    .await?;
```

---

## 5. Supplementary Vector Database: Qdrant

### Decision
Use **Qdrant** as an optional high-performance vector database for specialized workloads.

### Rationale

While SurrealDB handles most needs, **Qdrant** excels at:
- **Billion-scale vector search** (if required)
- **Advanced filtering** (metadata + vector similarity)
- **Distributed deployment** (sharding, replication)

#### When to Use Qdrant

| Scenario | Use SurrealDB | Use Qdrant |
|----------|---------------|------------|
| <10M vectors, graph queries needed | ✅ | ❌ |
| >100M vectors, vector search primary | ❌ | ✅ |
| Hybrid (graph + vector frequently) | ✅ | ❌ |
| Pure vector similarity | Either | ✅ Optimal |

#### Qdrant Integration Example

```rust
use qdrant_client::prelude::*;

let client = QdrantClient::from_url("http://localhost:6334").build()?;

// Create collection
client.create_collection(&CreateCollection {
    collection_name: "entities".to_string(),
    vectors_config: Some(VectorParams {
        size: 1536,
        distance: Distance::Cosine,
    }),
    ..Default::default()
}).await?;

// Search
let results = client.search_points(&SearchPoints {
    collection_name: "entities".to_string(),
    vector: query_embedding,
    limit: 40,
    with_payload: Some(WithPayloadSelector::from(true)),
    ..Default::default()
}).await?;
```

### Deployment Architecture

**Option A: SurrealDB Only** (default, simpler)
```
[LightRAG Core] → [SurrealDB]
                   ├─ Graph
                   ├─ Vectors
                   └─ Documents
```

**Option B: Hybrid** (scale-out vector workloads)
```
[LightRAG Core] → [SurrealDB] (graph + metadata)
                 ↘ [Qdrant] (vectors only)
```

---

## 6. LLM Integration: async-openai + Trait Abstraction

### Decision
Use **async-openai** for OpenAI API access and define an `LLMProvider` trait for multi-provider support.

### Rationale

#### LLM Client Architecture

**Primary Client: async-openai**
- **Version**: 0.32.0 (as of Dec 2025)
- **Coverage**: Complete OpenAI API (chat, embeddings, completions)
- **Compatibility**: Azure OpenAI, OpenAI-compatible endpoints
- **Async**: Built on Tokio + reqwest

#### Multi-Provider Design

```rust
#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn chat_completion(
        &self,
        messages: Vec<ChatMessage>,
        model: &str,
    ) -> Result<String, LLMError>;
    
    async fn generate_embedding(
        &self,
        text: &str,
        model: &str,
    ) -> Result<Vec<f32>, LLMError>;
}

// OpenAI implementation
pub struct OpenAIProvider {
    client: async_openai::Client<OpenAIConfig>,
}

// Anthropic implementation
pub struct AnthropicProvider {
    client: reqwest::Client,
    api_key: String,
}

// Ollama implementation
pub struct OllamaProvider {
    base_url: String,
    client: reqwest::Client,
}
```

#### Provider Comparison

| Provider | Client Library | Status (2025) |
|----------|----------------|---------------|
| **OpenAI** | async-openai | ✅ Mature |
| **Anthropic** | Custom (reqwest) | ⚠️ Community crates available |
| **Google Gemini** | Custom (reqwest) | ⚠️ Use google-generative-ai-rs |
| **Ollama** | Custom (reqwest) | ✅ Simple HTTP API |
| **Azure OpenAI** | async-openai | ✅ Built-in support |

### Example: Entity Extraction

```rust
async fn extract_entities(
    llm: &dyn LLMProvider,
    chunk: &str,
) -> Result<Vec<Entity>, LLMError> {
    let prompt = format!(
        "Extract entities from: {}\nFormat: entity_name|type|description",
        chunk
    );
    
    let response = llm.chat_completion(
        vec![ChatMessage::user(prompt)],
        "gpt-4o-mini"
    ).await?;
    
    parse_entity_response(&response)
}
```

### Alignment with Requirements

✅ **LLM-Agnostic**: Trait-based abstraction  
✅ **OpenAI**: Primary provider via async-openai  
✅ **Anthropic**: Custom implementation  
✅ **Local Models**: Ollama support  
✅ **Caching**: Response cache in SurrealDB  

---

## 7. Text Processing: tiktoken-rs + text-splitter

### Decision
Use **tiktoken-rs** for tokenization and **text-splitter** for semantic chunking.

### Rationale

#### Tokenization: tiktoken-rs

**Why tiktoken-rs?**
- Direct port of OpenAI's tiktoken (Python)
- Supports all OpenAI encoding schemes (o200k, cl100k, etc.)
- Accurate token counting for context window management

```rust
use tiktoken_rs::o200k_base;

let tokenizer = o200k_base()?;
let tokens = tokenizer.encode_with_special_tokens("Hello, world!");
println!("Token count: {}", tokens.len());
```

#### Chunking: text-splitter

**Why text-splitter?**
- **Semantic Awareness**: Unicode boundaries (graphemes, words, sentences)
- **Overlap Support**: Configurable token overlap
- **Markdown-Aware**: Preserves structure
- **tiktoken Integration**: Native support

```rust
use text_splitter::{ChunkConfig, TextSplitter};
use tiktoken_rs::cl100k_base;

let tokenizer = cl100k_base()?;
let config = ChunkConfig::new(1200)
    .with_sizer(tokenizer)
    .with_overlap(100);

let splitter = TextSplitter::new(config);
let chunks = splitter.chunks("your document...");
```

#### Algorithm Mapping

| Python LightRAG | Rust Equivalent |
|-----------------|-----------------|
| `chunking_by_token_size()` | `TextSplitter::chunks()` |
| `tiktoken.encode()` | `tiktoken_rs::CoreBPE::encode()` |
| Custom overlap logic | Built-in `with_overlap()` |

### Alignment with Requirements

✅ **Token Counting**: tiktoken-rs matches OpenAI exactly  
✅ **Chunking**: Configurable size + overlap  
✅ **Semantic Boundaries**: Preserves meaning  
✅ **Performance**: Native Rust, no Python overhead  

---

## 8. Frontend: Open WebUI

### Decision
Use **Open WebUI** as the production-ready web interface for LightRAG.

### Rationale

#### Why Open WebUI?

Open WebUI is a **mature, battle-tested LLM interface** (118k+ GitHub stars) specifically designed for RAG applications:

**Key Features**:
- **Production-Ready**: Used by thousands of organizations worldwide
- **Built-in RAG Support**: Native vector database integration (9+ options including pgvector)
- **Document Management**: Upload, chunk, and manage documents out-of-the-box
- **Multi-Model Support**: OpenAI, Anthropic, Ollama, and custom LLM providers
- **Rich UI**: Document library, chat interface, query modes
- **Authentication**: LDAP, OAuth, SSO support
- **Real-Time Features**: WebSocket support, streaming responses
- **Extensible**: Plugin system (Pipelines framework)
- **Docker-Ready**: Production deployment configurations included

#### Architecture

Open WebUI consists of:
- **Frontend**: Svelte + TypeScript + Tailwind CSS
- **Backend**: Python FastAPI (can integrate with Rust backend via API)
- **Database**: PostgreSQL (primary), SQLite (development)
- **Vector DB**: Supports pgvector, ChromaDB, Qdrant, Milvus, etc.

#### Integration Strategy with Rust Backend

Two deployment options:

**Option 1: Open WebUI as Frontend Layer** (Recommended)
```
┌─────────────────────────┐
│   Open WebUI            │
│   (Svelte + FastAPI)    │
└────────┬────────────────┘
         │ HTTP API
         ▼
┌─────────────────────────┐
│   LightRAG Rust Core    │
│   (Axum + OpenAPI)      │
└────────┬────────────────┘
         │
         ▼
┌─────────────────────────┐
│   PostgreSQL AGE        │
│   + pgvector            │
└─────────────────────────┘
```

**Option 2: Open WebUI Standalone** (Rapid Deployment)
```
┌──────────────────────────────────┐
│   Open WebUI (Full Stack)        │
│   Frontend + Backend + RAG       │
└────────┬─────────────────────────┘
         │
         ▼
┌─────────────────────────┐
│   PostgreSQL + pgvector │
│   FalkorDB (optional)   │
└─────────────────────────┘
```

#### Comparison with Custom Rust Frontends

| Approach | Time to Production | Features | Maintenance | Verdict |
|----------|-------------------|----------|-------------|---------|
| **Leptos/Dioxus** | 6-12 months | Custom | High | ❌ Reinventing wheel |
| **Open WebUI** | 1-2 weeks | Battle-tested | Low | ✅ **Selected** |

**Why Open WebUI Over Custom Rust Frontend?**

1. **Proven**: 118k stars, production-ready, battle-tested
2. **Feature-Complete**: Document upload, RAG, multi-model support, authentication
3. **Zero Development Time**: Ready to deploy
4. **Active Community**: 690+ contributors, continuous updates
5. **Extensible**: Plugin system for custom logic
6. **Professional UI**: Designed by UX experts, not engineers

### Configuration Example

```yaml
# docker-compose.yml
version: '3.8'

services:
  lightrag-backend:
    build: ./lightrag-rust
    ports:
      - "8000:8000"
    environment:
      - DATABASE_URL=postgresql://user:pass@postgres:5432/lightrag
      - OPENAI_API_KEY=${OPENAI_API_KEY}
    depends_on:
      - postgres

  open-webui:
    image: ghcr.io/open-webui/open-webui:main
    ports:
      - "3000:8080"
    environment:
      - OPENAI_API_BASE_URL=http://lightrag-backend:8000/v1
      - ENABLE_RAG_WEB_SEARCH=true
      - RAG_EMBEDDING_ENGINE=openai
      - VECTOR_DB=pgvector
      - DATABASE_URL=postgresql://user:pass@postgres:5432/webui
    volumes:
      - open-webui-data:/app/backend/data
    depends_on:
      - lightrag-backend
      - postgres

  postgres:
    image: pgvector/pgvector:pg17
    environment:
      - POSTGRES_PASSWORD=password
      - POSTGRES_DB=lightrag
    volumes:
      - postgres-data:/var/lib/postgresql/data
    ports:
      - "5432:5432"

volumes:
  open-webui-data:
  postgres-data:
```

### Integration with LightRAG Rust Backend

Open WebUI can call LightRAG Rust backend via OpenAI-compatible API:

```rust
// LightRAG Rust backend exposes OpenAI-compatible endpoints
use axum::{Router, routing::{post, get}};

async fn chat_completions(
    State(rag): State<Arc<LightRAG>>,
    Json(req): Json<ChatCompletionRequest>,
) -> Result<Json<ChatCompletionResponse>, ApiError> {
    // Query LightRAG with context from knowledge graph
    let context = rag.query(&req.messages.last().unwrap().content).await?;
    
    // Call LLM with augmented context
    let response = rag.llm_client
        .chat_completion(build_messages_with_context(&req.messages, &context))
        .await?;
    
    Ok(Json(ChatCompletionResponse { ... }))
}

let app = Router::new()
    .route("/v1/chat/completions", post(chat_completions))
    .route("/v1/models", get(list_models))
    .with_state(Arc::new(rag));
```

### Advantages

✅ **Production-Ready**: Deploy in hours, not months  
✅ **Feature-Complete**: Document management, RAG, multi-model support  
✅ **Battle-Tested**: 118k stars, proven at scale  
✅ **Zero Frontend Development**: Focus Rust efforts on core RAG engine  
✅ **Extensible**: Plugin system for custom business logic  
✅ **Professional UI**: Better than 99% of custom SPAs  

### When to Use Custom Rust Frontend

Only if:
- Open WebUI features are insufficient (rare)
- Need embedding in larger Rust application
- Regulatory requirements prevent external dependencies

Otherwise, Open WebUI is the pragmatic choice.

---

## 9. Graph Visualization: Cytoscape.js

### Decision
Use **Cytoscape.js** for interactive knowledge graph visualization in the frontend.

### Rationale

#### Why Cytoscape.js?

Cytoscape.js is the **industry-standard graph visualization library** with proven track record in bioinformatics and network analysis:

**Key Features**:
- **Performance**: Handles 1000+ nodes/edges smoothly
- **Layouts**: Force-directed, hierarchical, circular, grid, etc.
- **Interactive**: Pan, zoom, drag nodes, select elements
- **Styling**: CSS-like syntax for node/edge styling
- **Extensible**: 50+ official extensions
- **Well-Maintained**: 35+ years (Cytoscape desktop), active development
- **Proven**: Used by NIST, NIH, pharmaceutical companies

#### Comparison with Alternatives

| Library | Performance | Layout Options | Interactivity | Community | Verdict |
|---------|-------------|----------------|---------------|-----------|---------|
| **D3.js** | Good | Custom (complex) | Full | Huge | ❌ Too low-level |
| **vis.js** | Good | Limited | Good | Medium | ✅ Viable |
| **Cytoscape.js** | Excellent | Extensive | Excellent | Large | ✅ **Selected** |
| **Sigma.js** | Excellent | Limited | Good | Small | ❌ Less flexible |

**Why Cytoscape.js Over Others?**

1. **Purpose-Built**: Designed for graph visualization (not general dataviz)
2. **Layouts**: 10+ built-in algorithms (force, hierarchical, etc.)
3. **Extensions**: COSELayout (compound spring embedder) for large graphs
4. **Proven**: Battle-tested in scientific/pharmaceutical domains
5. **Documentation**: Excellent docs and examples

#### Integration with Open WebUI

Cytoscape.js can be embedded in Open WebUI via:

**Option 1: Custom Plugin** (Recommended)
```typescript
// open-webui-plugin/graph-viewer.ts
import cytoscape from 'cytoscape';
import coseBilkent from 'cytoscape-cose-bilkent';

cytoscape.use(coseBilkent);

export async function renderKnowledgeGraph(containerId: string, trackId: string) {
  // Fetch graph data from LightRAG backend
  const response = await fetch(`/api/graph/${trackId}`);
  const { nodes, edges } = await response.json();

  // Initialize Cytoscape
  const cy = cytoscape({
    container: document.getElementById(containerId),
    
    elements: {
      nodes: nodes.map(n => ({
        data: { id: n.id, label: n.name, type: n.entity_type }
      })),
      edges: edges.map(e => ({
        data: { source: e.source, target: e.target, label: e.relationship }
      }))
    },

    style: [
      {
        selector: 'node',
        style: {
          'background-color': '#3498db',
          'label': 'data(label)',
          'width': 40,
          'height': 40,
          'font-size': '12px',
          'text-valign': 'center',
          'text-halign': 'center'
        }
      },
      {
        selector: 'edge',
        style: {
          'width': 2,
          'line-color': '#95a5a6',
          'target-arrow-color': '#95a5a6',
          'target-arrow-shape': 'triangle',
          'curve-style': 'bezier',
          'label': 'data(label)',
          'font-size': '10px'
        }
      }
    ],

    layout: {
      name: 'cose-bilkent',
      idealEdgeLength: 100,
      nodeRepulsion: 4500,
      animate: true,
      animationDuration: 1000
    }
  });

  // Add interactivity
  cy.on('tap', 'node', (evt) => {
    const node = evt.target;
    console.log('Tapped node:', node.data());
    // Show node details in sidebar
  });
}
```

**Option 2: Standalone Graph Viewer Page**
Add new route in Open WebUI for dedicated graph visualization page.

#### Example: LightRAG Backend Graph API

```rust
// Expose graph data via REST API
use axum::{Router, routing::get, extract::{State, Path}};

#[derive(Serialize)]
struct GraphData {
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
}

#[derive(Serialize)]
struct GraphNode {
    id: String,
    name: String,
    entity_type: String,
    description: Option<String>,
}

#[derive(Serialize)]
struct GraphEdge {
    source: String,
    target: String,
    relationship: String,
    weight: f32,
}

async fn get_knowledge_graph(
    State(rag): State<Arc<LightRAG>>,
    Path(track_id): Path<String>,
) -> Result<Json<GraphData>, ApiError> {
    let entities = rag.storage.get_entities_by_document(&track_id).await?;
    let relationships = rag.storage.get_relationships_by_document(&track_id).await?;
    
    let nodes = entities.into_iter()
        .map(|e| GraphNode {
            id: e.id,
            name: e.name,
            entity_type: e.entity_type,
            description: e.description,
        })
        .collect();
    
    let edges = relationships.into_iter()
        .map(|r| GraphEdge {
            source: r.source_id,
            target: r.target_id,
            relationship: r.relationship_type,
            weight: r.weight,
        })
        .collect();
    
    Ok(Json(GraphData { nodes, edges }))
}

let app = Router::new()
    .route("/api/graph/:track_id", get(get_knowledge_graph))
    .with_state(Arc::new(rag));
```

#### Advanced Features

**1. Real-Time Updates** (WebSocket)
```typescript
const ws = new WebSocket('ws://localhost:8000/api/graph/stream');
ws.onmessage = (event) => {
  const { action, node, edge } = JSON.parse(event.data);
  if (action === 'add_node') cy.add({ group: 'nodes', data: node });
  if (action === 'add_edge') cy.add({ group: 'edges', data: edge });
};
```

**2. Search/Filter**
```typescript
// Highlight nodes matching search query
function searchGraph(query: string) {
  cy.elements().removeClass('highlighted');
  cy.nodes().filter(n => 
    n.data('label').toLowerCase().includes(query.toLowerCase())
  ).addClass('highlighted');
}
```

**3. Export**
```typescript
// Export graph as PNG
function exportGraph() {
  const png = cy.png({ full: true, scale: 2 });
  const link = document.createElement('a');
  link.href = png;
  link.download = 'knowledge-graph.png';
  link.click();
}
```

### Alignment with Requirements

✅ **Knowledge Graph Visualization**: Display entity-relationship graph  
✅ **Interactive**: Pan, zoom, drag, select nodes  
✅ **Real-Time**: WebSocket support for live updates  
✅ **Export**: PNG, JSON for analysis  
✅ **Production-Ready**: Used by NIST, NIH, major pharma companies  

### Installation

```bash
npm install cytoscape cytoscape-cose-bilkent
```

```html
<!-- In Open WebUI plugin -->
<div id="cy" style="width: 100%; height: 600px;"></div>
<script src="/plugins/graph-viewer.js"></script>
```

---

## 10. Error Handling: thiserror + anyhow

### Decision
Use **thiserror** for library errors and **anyhow** for application errors.

### Rationale

#### Error Handling Strategy

**thiserror** (for libraries/crates):
- Define custom error types with derive macros
- Preserve error context with `#[from]`
- Used in `lightrag-storage`, `lightrag-llm`, etc.

**anyhow** (for applications):
- Dynamic error type (`anyhow::Error`)
- Easy error propagation with `?`
- Context addition with `.context()`
- Used in `lightrag-api`, examples

#### Example: Library Error (thiserror)

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Connection failed: {0}")]
    ConnectionError(String),
    
    #[error("Query failed: {0}")]
    QueryError(#[from] surrealdb::Error),
    
    #[error("Serialization error: {0}")]
    SerdeError(#[from] serde_json::Error),
}
```

#### Example: Application Error (anyhow)

```rust
use anyhow::{Context, Result};

async fn insert_document(content: &str) -> Result<String> {
    let rag = get_rag()
        .context("Failed to initialize LightRAG")?;
    
    rag.ainsert(content).await
        .context("Document insertion failed")?;
    
    Ok("success".to_string())
}
```

### Alignment with Requirements

✅ **Type Safety**: Compile-time error checking  
✅ **Context**: Rich error messages  
✅ **Ergonomics**: `?` operator for propagation  
✅ **Best Practices**: Library vs application separation  

---

## 11. Observability: tracing + tracing-subscriber

### Decision
Use **tracing** for structured logging and observability.

### Rationale

#### Why tracing Over log?

| Aspect | log crate | tracing | Winner |
|--------|-----------|---------|--------|
| **Structured** | No | Yes | tracing |
| **Spans** | No | Yes | tracing |
| **Async-Aware** | No | Yes | tracing |
| **Context Propagation** | No | Yes | tracing |
| **OpenTelemetry** | Limited | Native | tracing |

#### tracing Features

**Structured Logging**:
```rust
use tracing::{info, warn, error, instrument};

#[instrument(skip(content), fields(doc_id = %doc_id))]
async fn insert_document(doc_id: &str, content: &str) -> Result<()> {
    info!(content_len = content.len(), "Inserting document");
    
    let chunks = chunk_text(content).await?;
    info!(chunk_count = chunks.len(), "Document chunked");
    
    Ok(())
}
```

**Span Hierarchy**:
```
document_insert [doc_id="abc123"]
  ├─ chunking [content_len=15000]
  ├─ entity_extraction [chunk_count=12]
  │   ├─ llm_call [model="gpt-4o-mini", tokens=1200]
  │   └─ llm_call [model="gpt-4o-mini", tokens=1150]
  ├─ graph_merge [entities=45, relations=67]
  └─ vector_indexing [vectors=45]
```

**Integration with OpenTelemetry**:
```rust
use tracing_subscriber::layer::SubscriberExt;
use opentelemetry::global;

let tracer = global::tracer("lightrag");
let telemetry = tracing_opentelemetry::layer()
    .with_tracer(tracer);

tracing_subscriber::registry()
    .with(telemetry)
    .with(tracing_subscriber::fmt::layer())
    .init();
```

### Alignment with Requirements

✅ **Debugging**: Rich context for troubleshooting  
✅ **Performance**: Span timing data  
✅ **Distributed Tracing**: OpenTelemetry support  
✅ **Async-Aware**: No lost context in async tasks  

---

## 12. Testing: cargo-nextest

### Decision
Use **cargo-nextest** as the primary test runner.

### Rationale

#### Why cargo-nextest?

**Performance Improvements** (vs `cargo test`):
- **Parallel Execution**: Tests run in separate processes
- **Faster Startup**: Shared build artifacts
- **Better Output**: Cleaner progress reporting

**Benchmarks** (real-world projects):
- 2x faster for small test suites
- 10x faster for large test suites (1000+ tests)

#### Example: Test Configuration

```toml
# .config/nextest.toml
[profile.default]
retries = 1
test-threads = 8
slow-timeout = { period = "60s", terminate-after = 3 }

[profile.ci]
retries = 3
fail-fast = true
```

#### Testing Strategy

**Unit Tests**:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_chunk_document() {
        let config = ChunkConfig::new(1200);
        let chunks = chunk_document("test content", config).await.unwrap();
        assert_eq!(chunks.len(), 1);
    }
}
```

**Integration Tests**:
```rust
#[tokio::test]
async fn test_full_insert_pipeline() {
    let rag = LightRAG::new_test_instance().await.unwrap();
    let track_id = rag.ainsert("test document").await.unwrap();
    
    // Verify entities extracted
    let entities = rag.query_entities().await.unwrap();
    assert!(!entities.is_empty());
}
```

**Property Tests** (with proptest):
```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_chunking_preserves_content(content in "\\PC{100,10000}") {
        let chunks = chunk_document(&content, default_config());
        let reconstructed = chunks.join("");
        assert!(reconstructed.contains(&content[..100]));
    }
}
```

### Alignment with Requirements

✅ **Speed**: Fast CI/CD pipelines  
✅ **Reliability**: Retries for flaky tests  
✅ **Coverage**: Integration + unit + property tests  

---

## 13. Database Technologies

### PostgreSQL AGE + pgvector (Primary Database)

#### Decision
Use **PostgreSQL with AGE extension and pgvector** as the primary database solution.

#### Rationale

**Why PostgreSQL AGE?**

Apache AGE (A Graph Extension) transforms PostgreSQL into a powerful graph database:

**Key Features**:
- **OpenCypher Support**: Industry-standard graph query language
- **SQL + Graph**: Combine relational and graph queries in one database
- **ACID Compliance**: Full transactional guarantees
- **Mature Ecosystem**: PostgreSQL's 35+ years of development
- **Multi-Model**: Graph + Document (JSONB) + Vector (pgvector) in one database

**Architecture**:
```
PostgreSQL 17+
├── AGE Extension (Graph database with OpenCypher)
├── pgvector Extension (Vector similarity search)
└── JSONB (Document storage)
```

**Why AGE Over Neo4j?**

| Feature | Neo4j | PostgreSQL AGE | Winner |
|---------|-------|----------------|--------|
| **Query Language** | Cypher | OpenCypher (compatible) | ✅ Tie |
| **ACID** | Yes | Yes | ✅ Tie |
| **Ecosystem** | Medium | Massive (PostgreSQL) | ✅ AGE |
| **Licensing** | Commercial (Enterprise) | Apache 2.0 | ✅ AGE |
| **Vector Search** | Separate service | pgvector (native) | ✅ AGE |
| **Operational Cost** | High | Low (single database) | ✅ AGE |
| **Learning Curve** | New stack | Familiar (PostgreSQL) | ✅ AGE |

#### pgvector Integration

**Why pgvector?**

- **Native PostgreSQL**: No separate vector database needed
- **Performance**: HNSW indexing for billion-scale vectors
- **SQL Integration**: Standard SQL queries for vector search
- **Production-Ready**: Used by Supabase, Timescale, Neon

**Vector Operations**:
```sql
-- Create vector column with HNSW index
CREATE TABLE embeddings (
    id UUID PRIMARY KEY,
    content TEXT,
    embedding VECTOR(1536)
);

CREATE INDEX ON embeddings USING hnsw (embedding vector_cosine_ops);

-- Vector similarity search
SELECT id, content, 
       1 - (embedding <=> query_vector) AS similarity
FROM embeddings
ORDER BY embedding <=> query_vector
LIMIT 10;
```

#### Example: Unified Graph + Vector Query

```sql
-- Find related entities using graph traversal + vector similarity
WITH similar_chunks AS (
    SELECT chunk_id, content, embedding <=> $1 AS distance
    FROM text_chunks
    ORDER BY embedding <=> $1
    LIMIT 5
)
SELECT 
    e.entity_name,
    e.entity_type,
    r.relationship_type,
    e2.entity_name AS related_entity,
    sc.content AS relevant_chunk
FROM similar_chunks sc
JOIN ag_catalog.entity e ON e.chunk_id = sc.chunk_id
JOIN ag_catalog.relationship r ON r.source_id = e.id
JOIN ag_catalog.entity e2 ON e2.id = r.target_id
WHERE ag_catalog.cypher('knowledge_graph', $$
    MATCH (a:Entity {id: $source_id})-[r:RELATES_TO]->(b:Entity)
    RETURN a, r, b
$$, json_build_object('source_id', e.id));
```

#### Rust Integration

```rust
use sqlx::{PgPool, postgres::PgPoolOptions};
use pgvector::Vector;

#[derive(sqlx::FromRow)]
struct Entity {
    id: String,
    name: String,
    entity_type: String,
}

pub struct AGEStorage {
    pool: PgPool,
}

impl AGEStorage {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(50)
            .connect(database_url)
            .await?;
        
        // Load AGE extension
        sqlx::query("CREATE EXTENSION IF NOT EXISTS age;")
            .execute(&pool)
            .await?;
        sqlx::query("CREATE EXTENSION IF NOT EXISTS vector;")
            .execute(&pool)
            .await?;
        
        Ok(Self { pool })
    }

    pub async fn search_similar_entities(
        &self,
        query_embedding: &[f32],
        top_k: usize,
    ) -> Result<Vec<Entity>> {
        let vector = Vector::from(query_embedding.to_vec());
        
        sqlx::query_as::<_, Entity>(
            "SELECT id, name, entity_type
             FROM entities
             ORDER BY embedding <=> $1
             LIMIT $2"
        )
        .bind(vector)
        .bind(top_k as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(Into::into)
    }

    pub async fn insert_entity_with_graph(
        &self,
        name: &str,
        entity_type: &str,
        embedding: &[f32],
    ) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let vector = Vector::from(embedding.to_vec());
        
        // Insert entity
        sqlx::query(
            "INSERT INTO entities (id, name, entity_type, embedding)
             VALUES ($1, $2, $3, $4)"
        )
        .bind(&id)
        .bind(name)
        .bind(entity_type)
        .bind(vector)
        .execute(&self.pool)
        .await?;
        
        // Add to graph (AGE Cypher)
        sqlx::query(
            "SELECT * FROM ag_catalog.cypher('knowledge_graph', $$
                CREATE (e:Entity {id: $id, name: $name, type: $type})
                RETURN e
            $$) AS (entity ag_catalog.agtype);"
        )
        .bind(&id)
        .bind(name)
        .bind(entity_type)
        .execute(&self.pool)
        .await?;
        
        Ok(id)
    }
}
```

### FalkorDB (Alternative Graph Database)

#### Decision
Offer **FalkorDB** as an alternative Redis-based graph database option.

#### Rationale

**Why FalkorDB?**

FalkorDB is a **Redis module** that brings graph database capabilities to Redis:

**Key Features**:
- **Ultra-Fast**: Built on Redis, sub-millisecond query latency
- **OpenCypher**: Same query language as AGE
- **GraphBLAS**: Uses sparse matrix operations for graph queries
- **Multi-Tenant**: Built-in isolation for multiple knowledge graphs
- **Redis Ecosystem**: Leverages Redis clustering, replication
- **LLM-Optimized**: Designed for Knowledge Graphs in RAG applications

**When to Use FalkorDB vs PostgreSQL AGE?**

| Use Case | Recommended | Reason |
|----------|-------------|---------|
| **Persistent Storage** | PostgreSQL AGE | ACID, durability |
| **Ultra-Low Latency** | FalkorDB | Sub-ms queries |
| **Large Knowledge Graphs** | PostgreSQL AGE | Better for >100M entities |
| **Multi-Tenancy** | FalkorDB | Native support |
| **Cost** | PostgreSQL AGE | No Redis Enterprise needed |

#### Example: FalkorDB Rust Integration

```rust
use redis::{Commands, Connection};

pub struct FalkorDBStorage {
    conn: Connection,
    graph_name: String,
}

impl FalkorDBStorage {
    pub fn new(redis_url: &str, graph_name: &str) -> Result<Self> {
        let client = redis::Client::open(redis_url)?;
        let conn = client.get_connection()?;
        Ok(Self {
            conn,
            graph_name: graph_name.to_string(),
        })
    }

    pub fn query_graph(&mut self, cypher: &str) -> Result<Vec<serde_json::Value>> {
        let result: Vec<String> = self.conn.graph_query(
            &self.graph_name,
            cypher
        )?;
        
        result.into_iter()
            .map(|r| serde_json::from_str(&r).map_err(Into::into))
            .collect()
    }

    pub fn add_entity(&mut self, name: &str, entity_type: &str) -> Result<()> {
        let cypher = format!(
            "CREATE (e:Entity {{name: '{}', type: '{}'}})",
            name, entity_type
        );
        self.conn.graph_query(&self.graph_name, &cypher)?;
        Ok(())
    }

    pub fn find_related(&mut self, entity_name: &str) -> Result<Vec<String>> {
        let cypher = format!(
            "MATCH (a:Entity {{name: '{}'}})-[r]->(b) RETURN b.name",
            entity_name
        );
        self.query_graph(&cypher)?
            .into_iter()
            .map(|v| Ok(v["b.name"].as_str().unwrap_or("").to_string()))
            .collect()
    }
}
```

### SurrealDB (Optional Multi-Model Database)

#### Decision
Keep **SurrealDB** as an optional all-in-one database for simpler deployments.

#### Rationale

SurrealDB remains a viable option for:
- **Simplified Deployment**: Single database for everything
- **Greenfield Projects**: No existing PostgreSQL infrastructure
- **Rapid Prototyping**: Quick setup without extensions

However, PostgreSQL AGE + pgvector is **recommended for production** due to:
- Mature ecosystem
- Better tooling (pgAdmin, Grafana, etc.)
- Proven at scale (Supabase, Neon, AWS RDS)

---

## 14. API Documentation: OpenAPI/Swagger

### Serialization: serde

**Why**: Universal standard for Rust serialization
- JSON via `serde_json`
- TOML via `toml`
- Derives for structs

```rust
#[derive(Serialize, Deserialize)]
struct Document {
    id: String,
    content: String,
    status: DocumentStatus,
}
```

### HTTP Client: reqwest

**Why**: Industry standard, Tokio-based
- Used for LLM API calls
- Connection pooling
- Timeout support

### Configuration: config + dotenvy

**Why**: Layered configuration
- `.env` files via dotenvy
- TOML/YAML via config crate
- Environment variable overrides

```rust
use config::{Config, Environment, File};

let settings = Config::builder()
    .add_source(File::with_name("config"))
    .add_source(Environment::with_prefix("LIGHTRAG"))
    .build()?;
```

### Build Tools

- **rustfmt**: Code formatting
- **clippy**: Linting (default + pedantic)
- **cargo-audit**: Security audits
- **cargo-watch**: Development hot-reload

---

## 14. API Documentation: OpenAPI/Swagger with utoipa

### Decision
Use **utoipa** for automatic OpenAPI 3.0 specification generation and Swagger UI.

### Rationale

#### Why utoipa?

**utoipa** is the leading OpenAPI generator for Rust:

**Key Features**:
- **Automatic Generation**: Derive macros for types and routes
- **OpenAPI 3.0**: Industry-standard specification
- **Swagger UI**: Interactive API documentation
- **Axum Integration**: First-class support via `utoipa-axum`
- **Type Safety**: Compile-time validation of API specs
- **Zero Runtime Cost**: All generation happens at compile time

#### Comparison with Alternatives

| Approach | Type Safety | Automation | Integration | Verdict |
|----------|-------------|------------|-------------|---------|
| **Manual OpenAPI YAML** | ❌ No | ❌ No | Any | ❌ Error-prone |
| **paperclip** | ✅ Yes | ✅ Yes | Limited | ❌ Less mature |
| **utoipa** | ✅ Yes | ✅ Yes | Excellent | ✅ **Selected** |

**Why utoipa Over Manual Specs?**

1. **Single Source of Truth**: Code is the spec
2. **Type Safety**: Compiler catches API mismatches
3. **Always Up-to-Date**: Spec auto-updates with code changes
4. **Developer Experience**: Swagger UI for testing
5. **Client Generation**: OpenAPI spec enables codegen for clients

#### Example: Axum Integration

```rust
use utoipa::{OpenApi, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};
use axum::{Json, extract::{State, Path}};

// Define API schemas
#[derive(ToSchema, Serialize, Deserialize)]
struct InsertRequest {
    /// Document content to insert into knowledge graph
    #[schema(example = "This is a sample document")]
    content: String,
    
    /// Optional metadata
    metadata: Option<serde_json::Value>,
}

#[derive(ToSchema, Serialize)]
struct InsertResponse {
    /// Unique tracking ID for the inserted document
    track_id: String,
    
    /// Number of chunks generated
    chunks_count: usize,
}

#[derive(ToSchema, Serialize)]
struct ApiError {
    /// Error message
    message: String,
    
    /// Error code
    code: String,
}

// Define API endpoints with documentation
#[utoipa::path(
    post,
    path = "/documents",
    tag = "documents",
    request_body = InsertRequest,
    responses(
        (status = 200, description = "Document inserted successfully", body = InsertResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    )
)]
async fn insert_document(
    State(rag): State<Arc<LightRAG>>,
    Json(req): Json<InsertRequest>,
) -> Result<Json<InsertResponse>, ApiError> {
    let track_id = rag.ainsert(&req.content).await?;
    let chunks_count = rag.get_chunks_count(&track_id).await?;
    
    Ok(Json(InsertResponse {
        track_id,
        chunks_count,
    }))
}

#[utoipa::path(
    post,
    path = "/query",
    tag = "query",
    request_body = QueryRequest,
    responses(
        (status = 200, description = "Query executed successfully", body = QueryResponse),
        (status = 400, description = "Invalid query", body = ApiError)
    )
)]
async fn query(
    State(rag): State<Arc<LightRAG>>,
    Json(req): Json<QueryRequest>,
) -> Result<Json<QueryResponse>, ApiError> {
    let result = rag.aquery(&req.query, req.mode).await?;
    Ok(Json(QueryResponse { result }))
}

// Generate OpenAPI spec
#[derive(OpenApi)]
#[openapi(
    info(
        title = "LightRAG API",
        version = "1.0.0",
        description = "Advanced Retrieval-Augmented Generation with Knowledge Graphs",
        contact(
            name = "API Support",
            email = "support@lightrag.io"
        ),
        license(
            name = "MIT",
            url = "https://opensource.org/licenses/MIT"
        )
    ),
    servers(
        (url = "http://localhost:8000", description = "Local development"),
        (url = "https://api.lightrag.io", description = "Production")
    ),
    tags(
        (name = "documents", description = "Document management"),
        (name = "query", description = "Query operations"),
        (name = "graph", description = "Knowledge graph operations")
    ),
    paths(insert_document, query),
    components(schemas(InsertRequest, InsertResponse, QueryRequest, QueryResponse, ApiError))
)]
struct ApiDoc;

#[tokio::main]
async fn main() {
    // Create router with OpenAPI integration
    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(routes!(insert_document))
        .routes(routes!(query))
        .split_for_parts();

    // Add Swagger UI
    let app = router
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api.clone()))
        .merge(Redoc::with_url("/redoc", api))
        .with_state(Arc::new(rag));

    // Serve
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

#### Generated OpenAPI Spec

The above code auto-generates:

```yaml
openapi: 3.0.0
info:
  title: LightRAG API
  version: 1.0.0
  description: Advanced Retrieval-Augmented Generation with Knowledge Graphs
  contact:
    name: API Support
    email: support@lightrag.io
  license:
    name: MIT
    url: https://opensource.org/licenses/MIT
servers:
  - url: http://localhost:8000
    description: Local development
  - url: https://api.lightrag.io
    description: Production
paths:
  /documents:
    post:
      tags:
        - documents
      summary: Insert document
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/InsertRequest'
      responses:
        '200':
          description: Document inserted successfully
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/InsertResponse'
        '400':
          description: Invalid request
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/ApiError'
components:
  schemas:
    InsertRequest:
      type: object
      required:
        - content
      properties:
        content:
          type: string
          example: "This is a sample document"
        metadata:
          type: object
          nullable: true
    InsertResponse:
      type: object
      properties:
        track_id:
          type: string
        chunks_count:
          type: integer
```

#### Swagger UI Access

Once running, access interactive API documentation at:
- **Swagger UI**: `http://localhost:8000/swagger-ui`
- **ReDoc**: `http://localhost:8000/redoc`
- **OpenAPI JSON**: `http://localhost:8000/api-docs/openapi.json`

#### Client Generation

Use OpenAPI spec to generate clients:

```bash
# TypeScript client
openapi-generator-cli generate \
  -i http://localhost:8000/api-docs/openapi.json \
  -g typescript-axios \
  -o ./clients/typescript

# Python client
openapi-generator-cli generate \
  -i http://localhost:8000/api-docs/openapi.json \
  -g python \
  -o ./clients/python

# Rust client
openapi-generator-cli generate \
  -i http://localhost:8000/api-docs/openapi.json \
  -g rust \
  -o ./clients/rust
```

### Dependencies

```toml
[dependencies]
utoipa = { version = "5", features = ["axum_extras", "chrono", "uuid"] }
utoipa-axum = "0.1"
utoipa-swagger-ui = { version = "8", features = ["axum"] }
utoipa-redoc = { version = "5", features = ["axum"] }
```

### Advantages

✅ **Type Safety**: Compile-time API validation  
✅ **Single Source of Truth**: Code is the spec  
✅ **Interactive Docs**: Swagger UI for testing  
✅ **Client Generation**: Auto-generate client libraries  
✅ **Always Up-to-Date**: Spec updates with code changes  
✅ **Zero Runtime Cost**: Compile-time generation  

### Integration with Open WebUI

Open WebUI can consume the OpenAPI spec to:
1. Auto-discover LightRAG endpoints
2. Generate TypeScript client code
3. Validate requests/responses
4. Display API documentation in UI

---

## Project Structure

### Cargo Workspace Layout

```
lightrag-rust/
├── Cargo.toml                    # Workspace root
├── .cargo/config.toml            # Build configuration
├── .env.example
├── config.toml.example
├── Dockerfile
├── docker-compose.yml
│
├── crates/
│   ├── lightrag-core/            # Core orchestrator
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── lightrag.rs       # Main LightRAG struct
│   │       ├── config.rs
│   │       └── types.rs
│   │
│   ├── lightrag-storage/         # Storage abstractions
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── traits.rs         # Storage traits
│   │       ├── surrealdb.rs      # SurrealDB impl
│   │       ├── qdrant.rs         # Qdrant impl
│   │       └── error.rs
│   │
│   ├── lightrag-llm/             # LLM abstractions
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── provider.rs       # LLMProvider trait
│   │       ├── openai.rs         # OpenAI impl
│   │       ├── anthropic.rs      # Anthropic impl
│   │       ├── ollama.rs         # Ollama impl
│   │       └── error.rs
│   │
│   ├── lightrag-pipeline/        # Document processing
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── chunking.rs
│   │       ├── extraction.rs
│   │       ├── merging.rs
│   │       └── embedding.rs
│   │
│   ├── lightrag-query/           # Query engine
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── modes.rs          # Query modes
│   │       ├── retrieval.rs
│   │       └── generation.rs
│   │
│   └── lightrag-api/             # REST API
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           ├── routes/
│           │   ├── documents.rs
│           │   └── query.rs
│           ├── middleware/
│           └── error.rs
│
├── lightrag-ui/                  # Leptos frontend
│   ├── Cargo.toml
│   ├── index.html
│   └── src/
│       ├── main.rs
│       ├── components/
│       └── pages/
│
├── examples/
│   ├── simple_insert.rs
│   ├── query_modes.rs
│   └── custom_provider.rs
│
└── tests/
    ├── integration/
    └── fixtures/
```

---

## Migration Strategy

### Phase 1: Core Foundation (Weeks 1-3)

**Goal**: Implement core types and storage layer

1. Define core types (`Document`, `Chunk`, `Entity`, `Relationship`)
2. Implement SurrealDB storage adapter
3. Create storage trait abstractions
4. Write comprehensive unit tests

**Success Criteria**:
- ✅ Documents can be stored and retrieved
- ✅ All tests passing
- ✅ Code coverage >80%

### Phase 2: Pipeline Implementation (Weeks 4-6)

**Goal**: Document processing pipeline

1. Integrate tiktoken-rs + text-splitter
2. Implement entity extraction with async-openai
3. Build graph merging logic
4. Add embedding generation

**Success Criteria**:
- ✅ End-to-end document insertion works
- ✅ Entities extracted correctly
- ✅ Graph populated accurately

### Phase 3: Query Engine (Weeks 7-8)

**Goal**: Multi-mode querying

1. Implement naive mode (vector search only)
2. Implement local mode (entity-centric)
3. Implement global mode (graph-centric)
4. Implement hybrid mode

**Success Criteria**:
- ✅ All query modes functional
- ✅ Results match Python implementation
- ✅ Performance benchmarks met

### Phase 4: API Layer (Weeks 9-10)

**Goal**: REST API with Axum

1. Define API routes
2. Implement request/response types
3. Add OpenAPI documentation
4. Integration tests

**Success Criteria**:
- ✅ API feature parity with Python
- ✅ OpenAPI spec generated
- ✅ All endpoints tested

### Phase 5: Frontend (Weeks 11-12)

**Goal**: Management UI with Leptos

1. Document upload interface
2. Query interface
3. Graph visualization
4. Status monitoring

**Success Criteria**:
- ✅ Functional UI
- ✅ Real-time status updates
- ✅ Responsive design

### Phase 6: Production Readiness (Weeks 13-14)

**Goal**: Deployment and optimization

1. Docker multi-stage builds
2. Kubernetes manifests
3. Performance tuning
4. Security audit
5. Documentation

**Success Criteria**:
- ✅ Single binary deployment
- ✅ Performance 10x Python
- ✅ Security audit passed
- ✅ Complete documentation

---

## Performance Targets

### Benchmarks vs Python Implementation

| Operation | Python (baseline) | Rust (target) | Improvement |
|-----------|------------------|---------------|-------------|
| **Document Chunking** | 100ms | <10ms | 10x |
| **Entity Extraction** | 5s (LLM-bound) | 5s (LLM-bound) | ~1x |
| **Graph Insertion** | 200ms | <20ms | 10x |
| **Vector Search** | 50ms | <10ms | 5x |
| **Query (Hybrid)** | 2s | <500ms | 4x |
| **Memory Usage** | 500MB | <100MB | 5x |

### Scalability Targets

- **Documents**: 10M+ documents
- **Entities**: 100M+ entities
- **Concurrent Requests**: 1000+ req/s
- **Query Latency (p99)**: <1s

---

## Security Considerations

### Best Practices

1. **Input Validation**: serde + validator crate
2. **SQL Injection**: Parameterized SurrealDB queries
3. **API Auth**: JWT via jsonwebtoken crate
4. **Rate Limiting**: Tower middleware
5. **HTTPS**: TLS via rustls
6. **Secrets Management**: Environment variables
7. **Dependency Auditing**: cargo-audit in CI

### Example: API Authentication

```rust
use axum::middleware;
use jsonwebtoken::{decode, DecodingKey, Validation};

async fn auth_middleware(
    headers: HeaderMap,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;
    
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(SECRET.as_ref()),
        &Validation::default()
    ).map_err(|_| StatusCode::UNAUTHORIZED)?;
    
    Ok(next.run(req).await)
}

let app = Router::new()
    .route("/query", post(query_handler))
    .layer(middleware::from_fn(auth_middleware));
```

---

## Risks and Mitigation

### Technical Risks

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| **SurrealDB Production Readiness** | High | Medium | Monitor community, have Qdrant+Neo4j fallback |
| **Leptos Breaking Changes** | Medium | Low | Pin versions, follow upgrade guides |
| **Rust Learning Curve** | Medium | Medium | Training, pair programming, documentation |
| **Compilation Time** | Low | High | Use cargo-nextest, incremental builds |

### Operational Risks

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| **Binary Size** | Low | High | Strip symbols, use release profile optimizations |
| **Deployment Complexity** | Medium | Low | Single binary simplifies vs Python |
| **Debugging Difficulty** | Medium | Medium | Extensive logging, error context |

---

## Conclusion

This technology stack represents the **best-in-class** choices for building a high-performance, type-safe, and maintainable RAG system in Rust as of December 2025. Key advantages:

1. **Performance**: 5-100x improvements over Python
2. **Type Safety**: Compile-time guarantees eliminate runtime errors
3. **Simplicity**: SurrealDB consolidates 12 storage instances into 1
4. **Modern**: Leverages 2025 ecosystem maturity (Axum, Leptos, etc.)
5. **Scalable**: Native async, true parallelism, efficient resource usage

The stack is designed for **long-term maintainability** with well-established technologies backed by strong communities and organizations (Tokio project, SurrealDB team, etc.).

### Next Steps

1. ✅ ADR Approved
2. → Begin Phase 1: Core Foundation
3. → Create individual technology guides
4. → Set up project repository structure

---

**Approved By**: Senior Principal Rust Architect  
**Date**: December 20, 2025  
**Revision**: 1.0
