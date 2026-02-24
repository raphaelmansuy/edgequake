//! Query operations for EdgeQuake.
//!
//! Contains `query()`, `search_entities()`, `get_entity_graph()`,
//! `get_graph_stats()`, `get_document()`, `list_documents()`.

use crate::error::{Error, Result};
use crate::types::{
    ContextEntity, DocumentInfo, GraphStats, QueryContext, QueryParams, QueryResult,
};

use super::EdgeQuake;

impl EdgeQuake {
    /// - **FEAT0007**: Multi-Mode Query Execution
    /// - **FEAT0101-0106**: Query mode strategies (naive/local/global/hybrid/mix/bypass)
    /// - **FEAT0107**: LLM-Based Keyword Extraction
    /// - **FEAT0108**: Smart Context Truncation
    ///
    /// # Enforces
    ///
    /// - **BR0101**: Token budget must not exceed LLM context window
    /// - **BR0102**: Graph context takes priority over naive chunks
    /// - **BR0103**: Query mode must be valid enum value
    /// - **BR0105**: Empty queries are rejected
    /// - **BR0201**: Tenant isolation (queries scoped to tenant/workspace)
    ///
    /// # WHY: Multi-Stage Retrieval Pipeline
    ///
    /// Query execution follows a multi-stage retrieval pipeline:
    ///
    /// ```text
    /// Query → Keywords → Vector Search → Graph Traversal → Context → LLM → Response
    ///                         ↓                ↓
    ///                    [chunks]        [entities, rels]
    /// ```
    ///
    /// 1. **Keyword Extraction** - Extract search terms from natural language query
    /// 2. **Candidate Retrieval** - Vector similarity + graph traversal
    /// 3. **Context Aggregation** - Merge and rank retrieved context
    /// 4. **Token Budget** - Truncate to fit LLM context window
    /// 5. **LLM Generation** - Generate final response
    ///
    /// # Arguments
    ///
    /// * `query` - Natural language query string
    /// * `params` - Optional query parameters (mode, filters, limits)
    ///
    /// # Returns
    ///
    /// [`QueryResult`] with response, sources, and retrieval statistics
    ///
    /// # Errors
    ///
    /// - `Error::not_initialized` if EdgeQuake not initialized
    /// - Query engine errors propagated from retrieval/generation
    ///
    /// # See Also
    ///
    /// - [`QueryParams`] for configuration options
    /// - [`QueryMode`] for available modes
    /// - [docs/features.md#FEAT0101](../../../../../../docs/features.md) for mode details
    pub async fn query(&self, query: &str, params: Option<QueryParams>) -> Result<QueryResult> {
        if !self.initialized {
            return Err(Error::not_initialized("EdgeQuake not initialized"));
        }

        let mut params = params.unwrap_or_default();

        // Set tenant and workspace IDs from config if not provided in params
        // WHY: Ensures tenant isolation (BR0201) even if caller forgets to set
        if params.tenant_id.is_none() {
            params.tenant_id = self.config.tenant_id.clone();
        }
        if params.workspace_id.is_none() {
            params.workspace_id = self.config.workspace_id.clone();
        }

        let query_engine = self
            .query_engine
            .as_ref()
            .ok_or_else(|| Error::not_initialized("Query engine not initialized"))?;

        // Delegate to SOTA query engine (FEAT0109)
        // WHY delegation: Query logic is complex; separating into edgequake-query crate
        // enables independent testing and evolution of retrieval strategies
        query_engine.query(query, params).await
    }

    /// Delete a document and cascade delete associated graph data.
    ///
    /// # Implements
    ///
    /// - **UC0005**: Delete Document
    /// - **FEAT0011**: Document-Chunk-Entity Lineage
    ///
    /// # Enforces
    ///
    /// - **BR0007**: Lineage records are append-only (deletion removes, not modifies)
    /// - **BR0201**: Tenant isolation (only deletes within tenant scope)
    ///
    /// # WHY: Source-Tracking Cascade Delete
    ///
    /// This implements document suppression with cascade semantics:
    ///
    /// 1. **Source Tracking** - Every entity/relationship stores `source_id` listing all
    ///    contributing chunks. WHY: A single entity (e.g., "Apple") may be mentioned
    ///    in 100 documents. We can't delete the entity unless ALL sources are gone.
    ///
    /// 2. **Cascade Logic**:
    ///    - If entity has ONLY sources from this document → DELETE entity
    ///    - If entity has MIXED sources → UPDATE to remove this document's sources
    ///
    /// 3. **Edge Cleanup** - Edges connected to deleted nodes are also deleted.
    ///    WHY: Orphan edges would corrupt graph queries.
    ///
    /// This matches LightRAG's P4-04 (Document Suppression) and P4-05 (Cascade Delete).
    pub async fn get_graph_stats(&self) -> Result<GraphStats> {
        if !self.initialized {
            return Err(Error::not_initialized("EdgeQuake not initialized"));
        }

        let graph_storage = self
            .graph_storage
            .as_ref()
            .ok_or_else(|| Error::not_initialized("Graph storage not initialized"))?;

        let node_count = graph_storage.node_count().await?;
        let edge_count = graph_storage.edge_count().await?;

        Ok(GraphStats {
            node_count,
            edge_count,
            ..Default::default()
        })
    }

    /// Get document information by ID.
    ///
    /// # Implements
    ///
    /// - **UC0003**: View Document Details
    /// - **FEAT0010**: Document Metadata Storage
    ///
    /// # TODO
    ///
    /// Implementation pending - needs to retrieve from KV store.
    pub async fn get_document(&self, _document_id: &str) -> Result<Option<DocumentInfo>> {
        if !self.initialized {
            return Err(Error::not_initialized("EdgeQuake not initialized"));
        }

        // TODO: Retrieve from KV store
        Ok(None)
    }

    /// List all documents in the knowledge base.
    ///
    /// # Implements
    ///
    /// - **UC0002**: List Documents
    /// - **FEAT0010**: Document Metadata Storage
    ///
    /// # TODO
    ///
    /// Implementation pending - needs to enumerate KV store entries.
    pub async fn list_documents(&self) -> Result<Vec<DocumentInfo>> {
        if !self.initialized {
            return Err(Error::not_initialized("EdgeQuake not initialized"));
        }

        Ok(Vec::new())
    }

    /// Search entities by name using vector similarity.
    ///
    /// # Implements
    ///
    /// - **UC0102**: Search Entities by Name
    /// - **FEAT0201**: Vector Similarity Search
    ///
    /// # WHY: Fuzzy Entity Discovery
    ///
    /// Users often don't know exact entity names. Vector similarity enables:
    /// - Typo tolerance (finding "Apple Inc" when searching "apple company")
    /// - Semantic matching (finding "Microsoft" when searching "software giant")
    pub async fn search_entities(&self, query: &str, limit: usize) -> Result<Vec<ContextEntity>> {
        if !self.initialized {
            return Err(Error::not_initialized("EdgeQuake not initialized"));
        }

        let embedding_provider = self
            .embedding_provider
            .as_ref()
            .ok_or_else(|| Error::not_initialized("Embedding provider not initialized"))?;

        let vector_storage = self
            .vector_storage
            .as_ref()
            .ok_or_else(|| Error::not_initialized("Vector storage not initialized"))?;

        let graph_storage = self
            .graph_storage
            .as_ref()
            .ok_or_else(|| Error::not_initialized("Graph storage not initialized"))?;

        // 1. Embed query
        let embeddings = embedding_provider.embed(&[query.to_string()]).await?;
        let query_embedding = embeddings
            .first()
            .ok_or_else(|| Error::internal("No embedding generated"))?;

        // 2. Search vector store
        let results = vector_storage.query(query_embedding, limit, None).await?;

        // 3. Map to ContextEntity
        let mut entities = Vec::new();
        for result in results {
            if let Some(node) = graph_storage.get_node(&result.id).await? {
                entities.push(ContextEntity {
                    name: node
                        .properties
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&result.id)
                        .to_string(),
                    entity_type: node
                        .properties
                        .get("entity_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("UNKNOWN")
                        .to_string(),
                    description: node
                        .properties
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    score: result.score,
                });
            }
        }

        Ok(entities)
    }

    /// Get knowledge graph subgraph centered on an entity.
    ///
    /// # Implements
    ///
    /// - **UC0101**: Explore Entity Neighborhood
    /// - **FEAT0202**: Graph Traversal
    /// - **FEAT0601**: Knowledge Graph Visualization
    ///
    /// # WHY: Visual Knowledge Exploration
    ///
    /// Subgraph extraction enables:
    /// - Interactive graph visualization in the WebUI
    /// - Understanding entity context and relationships
    /// - Debugging knowledge graph quality
    ///
    /// # Arguments
    ///
    /// * `entity_name` - Starting entity for traversal
    /// * `max_depth` - Maximum hops from starting entity (currently unused, always 1)
    /// * `max_nodes` - Maximum nodes to return (currently unused)
    ///
    /// # TODO
    ///
    /// - Implement multi-hop traversal with configurable depth
    /// - Add node limit enforcement
    /// - Optimize for large graphs with sampling
    pub async fn get_entity_graph(
        &self,
        entity_name: &str,
        _max_depth: usize,
        _max_nodes: usize,
    ) -> Result<QueryContext> {
        if !self.initialized {
            return Err(Error::not_initialized("EdgeQuake not initialized"));
        }

        let graph_storage = self
            .graph_storage
            .as_ref()
            .ok_or_else(|| Error::not_initialized("Graph storage not initialized"))?;

        // For now, just get the entity and its immediate neighbors
        let mut entities = Vec::new();
        let mut relationships = Vec::new();

        if let Some(node) = graph_storage.get_node(entity_name).await? {
            entities.push(ContextEntity {
                name: node
                    .properties
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or(entity_name)
                    .to_string(),
                entity_type: node
                    .properties
                    .get("entity_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("UNKNOWN")
                    .to_string(),
                description: node
                    .properties
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                score: 1.0,
            });

            let edges = graph_storage.get_node_edges(entity_name).await?;
            for edge in edges {
                relationships.push(crate::types::ContextRelationship {
                    source: edge.source.clone(),
                    target: edge.target.clone(),
                    relation_type: "RELATED".to_string(),
                    description: edge
                        .properties
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    score: 1.0,
                });

                // Also add the target entity if not already present
                if let Some(target_node) = graph_storage.get_node(&edge.target).await? {
                    entities.push(ContextEntity {
                        name: target_node
                            .properties
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or(&edge.target)
                            .to_string(),
                        entity_type: target_node
                            .properties
                            .get("entity_type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("UNKNOWN")
                            .to_string(),
                        description: target_node
                            .properties
                            .get("description")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        score: 1.0,
                    });
                }
            }
        }

        Ok(QueryContext {
            entities,
            relationships,
            ..Default::default()
        })
    }
}
