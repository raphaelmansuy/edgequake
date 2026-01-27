# Appendix C: References & Resources

## Primary Resources

### LightRAG Repository
- **GitHub:** https://github.com/HKUDS/LightRAG
- **License:** MIT
- **Organization:** Hong Kong University of Data Science

### Documentation
- **Official Docs:** https://lightrag.github.io (if available)
- **API Reference:** Generated from source docstrings
- **This Documentation:** Stack-agnostic implementation specification

---

## Research Papers

### LightRAG Paper
```bibtex
@article{lightrag2024,
  title={LightRAG: Simple and Fast Retrieval-Augmented Generation},
  author={HKUDS Team},
  year={2024},
  note={Original research paper describing LightRAG architecture}
}
```

### Related RAG Research
- **RAG Original:** "Retrieval-Augmented Generation for Knowledge-Intensive NLP Tasks" (Lewis et al., 2020)
- **GraphRAG:** Microsoft Research's graph-based retrieval approach
- **HyDE:** Hypothetical Document Embeddings for retrieval

---

## Technology References

### Large Language Models

| Provider | Documentation | Notes |
|----------|--------------|-------|
| OpenAI | https://platform.openai.com/docs | GPT-4, embeddings |
| Azure OpenAI | https://learn.microsoft.com/azure/cognitive-services/openai | Enterprise deployment |
| Anthropic | https://docs.anthropic.com | Claude models |
| Ollama | https://ollama.ai/docs | Local LLM hosting |
| HuggingFace | https://huggingface.co/docs | Open models |

### Embedding Models

| Model | Provider | Dimensions | Notes |
|-------|----------|------------|-------|
| text-embedding-3-small | OpenAI | 1536 | Cost-effective |
| text-embedding-3-large | OpenAI | 3072 | Highest quality |
| text-embedding-ada-002 | OpenAI | 1536 | Legacy |
| all-MiniLM-L6-v2 | HuggingFace | 384 | Open source |
| BGE-large | BAAI | 1024 | Multilingual |

### Vector Databases

| Database | Documentation | Notes |
|----------|--------------|-------|
| Milvus | https://milvus.io/docs | Scalable, open source |
| Qdrant | https://qdrant.tech/documentation | Rust-based, fast |
| ChromaDB | https://docs.trychroma.com | Python-native |
| Pinecone | https://docs.pinecone.io | Managed service |
| Weaviate | https://weaviate.io/developers/weaviate | GraphQL API |

### Graph Databases

| Database | Documentation | Notes |
|----------|--------------|-------|
| Neo4j | https://neo4j.com/docs | Industry standard |
| ArangoDB | https://www.arangodb.com/docs | Multi-model |
| Amazon Neptune | https://docs.aws.amazon.com/neptune | AWS managed |
| TigerGraph | https://docs.tigergraph.com | High performance |

### Key-Value Stores

| Store | Documentation | Notes |
|-------|--------------|-------|
| Redis | https://redis.io/docs | In-memory, fast |
| MongoDB | https://docs.mongodb.com | Document store |
| PostgreSQL | https://www.postgresql.org/docs | With JSON support |

---

## Python Libraries

### Core Dependencies

```yaml
core:
  - tiktoken: Token counting (OpenAI tokenizer)
  - numpy: Numerical operations
  - aiofiles: Async file operations
  - tenacity: Retry logic
  
llm_providers:
  - openai: OpenAI API client
  - anthropic: Anthropic API client
  - ollama: Ollama client
  
storage:
  - networkx: Graph operations
  - pymongo: MongoDB driver
  - neo4j: Neo4j driver
  - psycopg2/asyncpg: PostgreSQL drivers
  - pymilvus: Milvus client
  - qdrant-client: Qdrant client
  
api:
  - fastapi: Web framework
  - uvicorn: ASGI server
  - gunicorn: Production server
  - pydantic: Data validation
```

### Development Dependencies

```yaml
development:
  - pytest: Testing framework
  - pytest-asyncio: Async test support
  - ruff: Linting and formatting
  - mypy: Type checking
```

---

## Implementation Patterns

### Async Programming
- Python AsyncIO Documentation: https://docs.python.org/3/library/asyncio.html
- Real Python Async Guide: https://realpython.com/async-io-python/

### Dependency Injection
- FastAPI Depends: https://fastapi.tiangolo.com/tutorial/dependencies/

### Retry Patterns
- Tenacity Library: https://tenacity.readthedocs.io/

### Batch Processing
- Asyncio Gather: https://docs.python.org/3/library/asyncio-task.html#asyncio.gather

---

## Similar Projects

### RAG Frameworks

| Project | URL | Focus |
|---------|-----|-------|
| LangChain | https://langchain.com | General LLM framework |
| LlamaIndex | https://www.llamaindex.ai | Data framework for LLMs |
| Haystack | https://haystack.deepset.ai | NLP pipeline framework |
| Microsoft GraphRAG | https://github.com/microsoft/graphrag | Graph-based RAG |

### Knowledge Graph Tools

| Project | URL | Focus |
|---------|-----|-------|
| NetworkX | https://networkx.org | Python graph library |
| Neo4j GDS | https://neo4j.com/docs/graph-data-science | Graph algorithms |
| Apache TinkerPop | https://tinkerpop.apache.org | Graph computing |

---

## Standards & Specifications

### API Standards
- OpenAPI 3.0: https://swagger.io/specification/
- JSON:API: https://jsonapi.org/

### Data Formats
- JSON: https://www.json.org/
- JSON-LD: https://json-ld.org/ (for knowledge graphs)

### Security
- OWASP API Security: https://owasp.org/www-project-api-security/
- JWT: https://jwt.io/

---

## Learning Resources

### RAG Concepts
- Pinecone RAG Guide: https://www.pinecone.io/learn/retrieval-augmented-generation/
- LangChain RAG Tutorial: https://python.langchain.com/docs/tutorials/rag/

### Knowledge Graphs
- Neo4j Graph Academy: https://graphacademy.neo4j.com/
- Stanford CS520: https://web.stanford.edu/class/cs520/

### Vector Search
- Faiss Tutorial: https://github.com/facebookresearch/faiss/wiki
- Approximate Nearest Neighbor: https://ann-benchmarks.com/

---

## Community & Support

### Forums
- LightRAG GitHub Issues: For bug reports and feature requests
- LightRAG Discussions: For questions and ideas

### Related Communities
- r/MachineLearning: Reddit ML community
- Hugging Face Forums: Open-source AI discussion
- LangChain Discord: LLM framework community

---

## Version History

This documentation was generated from LightRAG source code analysis.

### Covered Versions
- LightRAG: Current main branch
- API Version: v1
- Storage Schema: v2

### Documentation Version
- Generated: 2024
- Format: Markdown with Mermaid diagrams
- Specification: 002-reverse-documentation

---

## Acknowledgments

This documentation follows the specification defined in:
- `specs/002-reverse-documentation.md`

The goal is to produce stack-agnostic documentation enabling LightRAG reimplementation in any technology stack.

---

## Cross-References

- [Index](../00-index.md) - Documentation navigation
- [Executive Summary](../01-executive-summary.md) - Project overview
- [Glossary](A-glossary.md) - Term definitions
