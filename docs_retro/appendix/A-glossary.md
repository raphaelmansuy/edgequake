# Appendix A: Glossary

## Core Concepts

### RAG (Retrieval-Augmented Generation)
A technique that enhances Large Language Model responses by retrieving relevant context from a knowledge base before generating an answer. LightRAG extends this with graph-based retrieval.

### Knowledge Graph (KG)
A structured representation of information where entities (nodes) are connected by relationships (edges). LightRAG automatically constructs knowledge graphs from unstructured documents.

### Vector Database (VDB)
A specialized database optimized for storing and querying high-dimensional vectors (embeddings). Used for semantic similarity search in LightRAG.

### Embedding
A numerical vector representation of text that captures semantic meaning. Similar texts have similar embeddings, enabling similarity search.

---

## Domain Entities

### Document
The primary input unit. A piece of text content (book, article, report) that LightRAG processes to extract knowledge. Documents are chunked, entities are extracted, and relationships are identified.

### Chunk
A segment of a document created by the chunking algorithm. Chunks are small enough to process efficiently but large enough to maintain context. Default size is 1200 tokens with 100-token overlap.

### Entity
A named object extracted from text, such as a person, organization, location, or concept. Entities become nodes in the knowledge graph with properties like name, type, and description.

### Relationship
A connection between two entities describing how they relate. Relationships become edges in the knowledge graph with properties like description, keywords, and weight.

### Source ID
A unique identifier linking extracted entities and relationships back to their source chunks. Uses MD5 hash of chunk content.

---

## Storage Types

### Key-Value Storage (KV)
Storage for document content and metadata. Maps document/chunk IDs to their content and properties.

**Namespaces:**
- `full_docs` - Complete document content
- `text_chunks` - Document chunks
- `llm_response_cache` - Cached LLM responses

### Vector Storage (VDB)
Storage for embeddings with similarity search capability. Stores entity descriptions and text chunks with their vector representations.

**Namespaces:**
- `entities_vdb` - Entity embeddings
- `relationships_vdb` - Relationship embeddings
- `chunks_vdb` - Chunk embeddings

### Graph Storage
Storage for the knowledge graph structure. Stores entities as nodes and relationships as edges with properties.

**Single namespace:** The knowledge graph

---

## Query Modes

### Naive Mode
Direct similarity search over text chunks without using the knowledge graph. Fastest but least contextual.

### Local Mode
Query processing that starts from query-relevant entities and expands to one-hop neighbors. Provides entity-level context from the knowledge graph.

### Global Mode
Query processing that considers high-level community structures and globally significant nodes. Provides broad context for complex questions.

### Hybrid Mode
Combination of local and global context for comprehensive retrieval. Default and recommended mode.

### Bypass Mode
Returns only the retrieved context without generating a response. Useful for custom post-processing pipelines.

---

## Processing Pipeline

### Chunking
The process of splitting documents into smaller segments. LightRAG uses token-based chunking with configurable overlap.

### Entity Extraction
Using an LLM to identify and extract named entities from text chunks. Output format: `entity<|#|>NAME<|#|>TYPE<|#|>DESCRIPTION`

### Relationship Extraction
Using an LLM to identify connections between entities. Output format: `relationship<|#|>SOURCE<|#|>TARGET<|#|>DESCRIPTION<|#|>KEYWORDS<|#|>WEIGHT`

### Merging
Combining duplicate entities and relationships from different chunks. Aggregates descriptions, keywords, and source references.

### Summarization
Condensing long descriptions using map-reduce pattern when they exceed token limits.

---

## Configuration Terms

### Working Directory
The base path for file-based storage. Each LightRAG instance uses its own working directory.

### Namespace
A prefix that isolates storage across different instances or tenants. Prevents data collision in shared backends.

### Batch Size
Number of items processed together. Controls parallelism and memory usage.

### Token Size
Measurement of text length using the tokenizer. Different from character or word count.

### Embedding Dimension
The size of the vector produced by the embedding model. Common values: 1536 (OpenAI), 768 (many open models).

---

## Multi-Tenancy Terms

### Tenant
An isolated organizational unit. Each tenant has completely separate data and configuration.

### Knowledge Base
A logical grouping of documents within a tenant. Tenants can have multiple knowledge bases for different purposes.

### Workspace
The combination of tenant and knowledge base identifiers. Format: `{tenant_id}:{kb_id}`

---

## LLM Integration

### LLM Function
An async callable that takes a prompt and returns generated text. LightRAG is model-agnostic through this abstraction.

### Embedding Function
An async callable that converts text to vectors. Returns numpy arrays of shape `(batch_size, embedding_dim)`.

### Model Max Tokens
The context window size of the LLM. Determines maximum prompt + response length.

### Max Async
Maximum concurrent LLM calls. Controls parallelism and rate limiting.

---

## Algorithms

### Map-Reduce Summarization
A technique for summarizing long texts by dividing into chunks (map), summarizing each, then combining summaries (reduce).

### Graph Traversal
Navigating the knowledge graph by following edges from starting nodes. Used in local and global query modes.

### Similarity Search
Finding the most similar vectors to a query vector. Uses cosine similarity or other distance metrics.

### Entity Resolution
Determining when different text mentions refer to the same real-world entity. LightRAG uses uppercase normalization.

---

## Technical Terms

### Async/Await
Python's asynchronous programming model. LightRAG uses async for non-blocking I/O operations.

### Tokenizer
A tool that converts text to tokens (subword units). LightRAG uses tiktoken (OpenAI's tokenizer) by default.

### MD5 Hash
A cryptographic hash function producing 128-bit fingerprints. Used for generating deterministic IDs.

### UPSERT
An operation that inserts new records or updates existing ones based on a key.

---

## Storage Backends

### JsonKVStorage
File-based key-value storage using JSON files. Default for development.

### NanoVectorDB
Lightweight in-memory vector database. Good for small datasets.

### NetworkX
Python library for graph operations. Default graph storage for development.

### Neo4j
Enterprise graph database. Recommended for production knowledge graph storage.

### Milvus
Open-source vector database. Recommended for production vector storage.

### PostgreSQL
Relational database with vector extensions (pgvector). Unified storage option.

---

## Abbreviations

| Abbrev | Meaning |
|--------|---------|
| RAG | Retrieval-Augmented Generation |
| KG | Knowledge Graph |
| VDB | Vector Database |
| LLM | Large Language Model |
| KV | Key-Value |
| API | Application Programming Interface |
| SDK | Software Development Kit |
| E2E | End-to-End |
| OOM | Out of Memory |
| TTL | Time To Live |
| CRUD | Create, Read, Update, Delete |

---

## Cross-References

- [Domain Model](../03-domain-model.md) - Detailed entity definitions
- [Storage Contracts](../06-storage-contracts.md) - Storage interface details
- [Configuration](../08-configuration.md) - Configuration parameters
