# LightRAG Architecture Overview

## System Purpose & Boundaries

### What This System Does
LightRAG is a Retrieval-Augmented Generation framework that:
1. **Ingests documents** by chunking text and extracting entities/relationships using LLMs
2. **Builds a knowledge graph** connecting entities with typed relationships
3. **Answers queries** by combining graph traversal with vector similarity search
4. **Generates responses** using LLM with retrieved context

### What It Explicitly Does NOT Do
- Real-time streaming ingestion
- Image/audio/video processing
- Model training or fine-tuning
- Document format conversion (expects plain text)
- Authentication/authorization (relies on external auth)

---

## Level 1: 10,000-Foot View

```mermaid
graph TB
    classDef external fill:#fff3e0,stroke:#e65100
    classDef core fill:#e3f2fd,stroke:#1565c0
    classDef storage fill:#e8f5e9,stroke:#2e7d32
    
    USER[User/Application]:::external
    
    subgraph "LightRAG System"
        API[REST API Layer]:::core
        ORCH[Orchestrator]:::core
        PIPE[Pipeline Engine]:::core
        QUERY[Query Engine]:::core
        
        subgraph "Storage Adapters"
            VS[Vector Storage]:::storage
            GS[Graph Storage]:::storage
            KV[KV Storage]:::storage
        end
    end
    
    LLM[LLM Provider]:::external
    EMB[Embedding Provider]:::external
    DB[(Databases)]:::external
    
    USER --> API
    API --> ORCH
    ORCH --> PIPE
    ORCH --> QUERY
    PIPE --> VS
    PIPE --> GS
    PIPE --> KV
    QUERY --> VS
    QUERY --> GS
    PIPE --> LLM
    PIPE --> EMB
    QUERY --> LLM
    VS --> DB
    GS --> DB
    KV --> DB
```

### Component Summary
| Component | Responsibility |
|-----------|---------------|
| **REST API Layer** | HTTP endpoints for document and query operations |
| **Orchestrator** | Coordinates all operations, manages storage lifecycle |
| **Pipeline Engine** | Document ingestion, chunking, entity extraction |
| **Query Engine** | Multi-mode query processing, context retrieval |
| **Vector Storage** | Embedding storage and similarity search |
| **Graph Storage** | Knowledge graph with entities and relationships |
| **KV Storage** | Caching, document status, LLM response cache |

---

## Level 2: Component Interaction

### Document Ingestion Flow

```mermaid
sequenceDiagram
    participant Client
    participant API
    participant Orchestrator
    participant Pipeline
    participant Chunker
    participant LLM
    participant VectorDB
    participant GraphDB
    participant KVStore
    
    Client->>API: POST /documents/text
    API->>Orchestrator: ainsert(text)
    Orchestrator->>KVStore: Store full document
    Orchestrator->>Pipeline: Process document
    
    Pipeline->>Chunker: Split into chunks
    Chunker-->>Pipeline: Chunks[]
    
    loop For each chunk
        Pipeline->>LLM: Extract entities/relations
        LLM-->>Pipeline: Entities[], Relations[]
    end
    
    Pipeline->>VectorDB: Store chunk embeddings
    Pipeline->>VectorDB: Store entity embeddings
    Pipeline->>VectorDB: Store relation embeddings
    Pipeline->>GraphDB: Create nodes (entities)
    Pipeline->>GraphDB: Create edges (relations)
    Pipeline->>KVStore: Update document status
    
    Orchestrator-->>API: Success response
    API-->>Client: 200 OK
```

### Query Processing Flow

```mermaid
sequenceDiagram
    participant Client
    participant API
    participant Orchestrator
    participant QueryEngine
    participant VectorDB
    participant GraphDB
    participant LLM
    
    Client->>API: POST /query
    API->>Orchestrator: aquery(question, mode)
    Orchestrator->>QueryEngine: Process query
    
    alt mode == "naive"
        QueryEngine->>VectorDB: Search chunks
        VectorDB-->>QueryEngine: Relevant chunks
    else mode == "local"
        QueryEngine->>VectorDB: Search entities
        VectorDB-->>QueryEngine: Similar entities
        QueryEngine->>GraphDB: Get entity context
        GraphDB-->>QueryEngine: Connected nodes/edges
    else mode == "global"
        QueryEngine->>GraphDB: Get high-degree entities
        GraphDB-->>QueryEngine: Central entities
        QueryEngine->>VectorDB: Get relation context
    else mode == "hybrid"
        QueryEngine->>VectorDB: Search all (chunks, entities, relations)
        QueryEngine->>GraphDB: Traverse connections
    end
    
    QueryEngine->>LLM: Generate response with context
    LLM-->>QueryEngine: Generated answer
    QueryEngine-->>Orchestrator: QueryResult
    Orchestrator-->>API: Response
    API-->>Client: JSON response
```

---

## Level 3: Data Flow & Storage

### Entity Relationship Diagram

```mermaid
erDiagram
    DOCUMENT ||--o{ CHUNK : contains
    CHUNK ||--o{ ENTITY : mentions
    CHUNK ||--o{ RELATIONSHIP : mentions
    ENTITY }o--o{ RELATIONSHIP : connects
    ENTITY ||--o{ ENTITY_EMBEDDING : has
    RELATIONSHIP ||--o{ RELATION_EMBEDDING : has
    CHUNK ||--o{ CHUNK_EMBEDDING : has
    
    DOCUMENT {
        string id PK
        string content
        string file_path
        enum status
        string track_id
        datetime created_at
        datetime updated_at
    }
    
    CHUNK {
        string id PK
        string content
        string document_id FK
        string file_path
        int token_count
    }
    
    ENTITY {
        string id PK
        string entity_name
        string entity_type
        string description
        string source_id
        string file_path
    }
    
    RELATIONSHIP {
        string id PK
        string source_entity FK
        string target_entity FK
        string description
        string keywords
        float weight
        string source_id
    }
    
    ENTITY_EMBEDDING {
        string id PK
        string entity_name
        float[] vector
    }
    
    RELATION_EMBEDDING {
        string id PK
        string src_entity
        string tgt_entity  
        float[] vector
    }
    
    CHUNK_EMBEDDING {
        string id PK
        float[] vector
        string content
    }
```

---

## Storage Architecture

### Storage Type Mapping

| Namespace | Storage Type | Purpose |
|-----------|-------------|---------|
| `full_docs` | KV Storage | Complete document content |
| `doc_status` | KV Storage | Document processing status |
| `text_chunks` | KV Storage | Chunked document segments |
| `llm_response_cache` | KV Storage | Cached LLM responses |
| `chunk_entity_relation_graph` | Graph Storage | Knowledge graph |
| `entities_vdb` | Vector Storage | Entity embeddings |
| `relationships_vdb` | Vector Storage | Relationship embeddings |
| `chunks_vdb` | Vector Storage | Chunk embeddings |

### Storage Backend Options

```mermaid
graph TD
    subgraph "Storage Abstractions"
        BKV[BaseKVStorage]
        BVS[BaseVectorStorage]
        BGS[BaseGraphStorage]
    end
    
    subgraph "KV Implementations"
        JSON[JsonKVStorage]
        REDIS[RedisKVStorage]
        PG_KV[PostgresKVStorage]
        MONGO_KV[MongoKVStorage]
    end
    
    subgraph "Vector Implementations"
        NANO[NanoVectorDBStorage]
        FAISS[FAISSVectorDBStorage]
        QDRANT[QdrantVectorDBStorage]
        MILVUS[MilvusVectorDBStorage]
        MONGO_V[MongoVectorDBStorage]
    end
    
    subgraph "Graph Implementations"
        NX[NetworkXStorage]
        NEO4J[Neo4JStorage]
        MEM[MemgraphStorage]
        MONGO_G[MongoGraphStorage]
    end
    
    BKV --> JSON
    BKV --> REDIS
    BKV --> PG_KV
    BKV --> MONGO_KV
    
    BVS --> NANO
    BVS --> FAISS
    BVS --> QDRANT
    BVS --> MILVUS
    BVS --> MONGO_V
    
    BGS --> NX
    BGS --> NEO4J
    BGS --> MEM
    BGS --> MONGO_G
```

---

## Multi-Tenancy Architecture

```mermaid
graph TB
    subgraph "API Layer"
        MW[Tenant Middleware]
        AUTH[Auth Handler]
    end
    
    subgraph "Service Layer"
        TS[Tenant Service]
        TRM[Tenant RAG Manager]
    end
    
    subgraph "Data Layer"
        T1[Tenant 1 Data]
        T2[Tenant 2 Data]
        TN[Tenant N Data]
    end
    
    MW --> AUTH
    AUTH --> TS
    TS --> TRM
    TRM --> T1
    TRM --> T2
    TRM --> TN
```

### Tenant Isolation
- Each tenant has isolated storage namespaces
- Knowledge bases within tenants provide further isolation
- Workspace-based directory separation for file storage
- Context variables track current tenant through request lifecycle

---

## Concurrency Model

### Pipeline Concurrency

```mermaid
graph LR
    subgraph "Concurrency Controls"
        SEM[Semaphore<br/>max_parallel_insert]
        LOCK[Keyed Locks<br/>per entity/relation]
        PIPE_LOCK[Pipeline Lock<br/>per workspace]
    end
    
    subgraph "Processing"
        P1[Process Doc 1]
        P2[Process Doc 2]
        PN[Process Doc N]
    end
    
    SEM --> P1
    SEM --> P2
    SEM --> PN
    
    P1 --> LOCK
    P2 --> LOCK
    PN --> LOCK
```

### Key Concurrency Mechanisms
1. **Semaphore**: Limits parallel document processing (`max_parallel_insert`)
2. **Keyed Locks**: Fine-grained locks for entity/relationship updates
3. **Pipeline Lock**: Ensures single worker processes queue per workspace
4. **Async/Await**: Non-blocking I/O throughout

---

## API Layer Architecture

```mermaid
graph TB
    subgraph "FastAPI Application"
        APP[Main App]
        
        subgraph "Routers"
            DOC[Document Routes]
            QRY[Query Routes]
            GRF[Graph Routes]
            OLL[Ollama API]
            TNT[Tenant Routes]
        end
        
        subgraph "Middleware"
            CORS[CORS]
            AUTH[Auth]
            TENANT[Tenant Context]
        end
    end
    
    APP --> CORS
    CORS --> AUTH
    AUTH --> TENANT
    TENANT --> DOC
    TENANT --> QRY
    TENANT --> GRF
    TENANT --> OLL
    TENANT --> TNT
```

---

## Deployment Topologies

### Single Process (Development)

```mermaid
graph TB
    subgraph "Single Process"
        UV[Uvicorn]
        APP[LightRAG App]
        FS[(File System)]
    end
    
    UV --> APP
    APP --> FS
```

### Multi-Worker (Production)

```mermaid
graph TB
    subgraph "Gunicorn Master"
        MASTER[Master Process]
        
        subgraph "Workers"
            W1[Worker 1]
            W2[Worker 2]
            WN[Worker N]
        end
    end
    
    subgraph "Shared State"
        MP[Multiprocessing<br/>Manager]
        LOCKS[Shared Locks]
    end
    
    subgraph "External Storage"
        DB[(Database)]
    end
    
    MASTER --> W1
    MASTER --> W2
    MASTER --> WN
    
    W1 --> MP
    W2 --> MP
    WN --> MP
    
    MP --> LOCKS
    
    W1 --> DB
    W2 --> DB
    WN --> DB
```

---

## Cross-References

- [Domain Model](03-domain-model.md) - Detailed entity definitions
- [API Contracts](04-api-contracts.md) - REST API specifications
- [Storage Contracts](06-storage-contracts.md) - Backend interface details
- [Configuration](08-configuration.md) - All configuration options
