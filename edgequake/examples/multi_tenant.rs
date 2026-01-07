//! Multi-tenant RAG example.
//!
//! This example demonstrates how to use EdgeQuake in a multi-tenant
//! environment where each tenant has isolated data.
//!
//! Run with: cargo run --example multi_tenant

use std::sync::Arc;

use edgequake_llm::{EmbeddingProvider, MockProvider};
use edgequake_pipeline::Chunker;
use edgequake_storage::{
    GraphStorage, KVStorage, MemoryGraphStorage, MemoryKVStorage, MemoryVectorStorage,
    VectorStorage,
};

/// A tenant-isolated RAG instance.
struct TenantRAG {
    tenant_id: String,
    kv_storage: Arc<MemoryKVStorage>,
    vector_storage: Arc<MemoryVectorStorage>,
    graph_storage: Arc<MemoryGraphStorage>,
    llm_provider: Arc<MockProvider>,
}

impl TenantRAG {
    /// Create a new tenant-isolated RAG instance.
    fn new(tenant_id: &str) -> Self {
        // Create namespaced storage for this tenant
        let namespace = format!("tenant_{}", tenant_id);

        Self {
            tenant_id: tenant_id.to_string(),
            kv_storage: Arc::new(MemoryKVStorage::new(&namespace)),
            vector_storage: Arc::new(MemoryVectorStorage::new(&namespace, 1536)),
            graph_storage: Arc::new(MemoryGraphStorage::new(&namespace)),
            llm_provider: Arc::new(MockProvider::new()),
        }
    }

    /// Initialize storage backends.
    async fn initialize(&self) -> anyhow::Result<()> {
        self.kv_storage.initialize().await?;
        self.vector_storage.initialize().await?;
        self.graph_storage.initialize().await?;
        println!("Tenant '{}' storage initialized", self.tenant_id);
        Ok(())
    }

    /// Ingest a document for this tenant.
    async fn ingest_document(&self, doc_id: &str, content: &str) -> anyhow::Result<()> {
        // Create chunker
        let chunker = Chunker::default_chunker();
        let chunks = chunker.chunk(content, doc_id)?;

        println!(
            "Tenant '{}': Ingesting document '{}' ({} chunks)",
            self.tenant_id,
            doc_id,
            chunks.len()
        );

        // Store chunks in KV storage
        for chunk in &chunks {
            let data = vec![(
                chunk.id.clone(),
                serde_json::json!({
                    "content": chunk.content,
                    "document_id": doc_id,
                    "index": chunk.index,
                    "tenant_id": self.tenant_id,
                }),
            )];
            self.kv_storage.upsert(&data).await?;

            // Generate and store embedding
            let embedding = self.llm_provider.embed_one(&chunk.content).await?;
            self.vector_storage
                .upsert(&[(
                    chunk.id.clone(),
                    embedding,
                    serde_json::json!({
                        "chunk_id": chunk.id,
                        "document_id": doc_id,
                    }),
                )])
                .await?;
        }

        // Store document metadata
        self.kv_storage
            .upsert(&[(
                format!("doc_{}", doc_id),
                serde_json::json!({
                    "id": doc_id,
                    "chunk_count": chunks.len(),
                    "tenant_id": self.tenant_id,
                }),
            )])
            .await?;

        println!(
            "Tenant '{}': Document '{}' ingested successfully",
            self.tenant_id, doc_id
        );

        Ok(())
    }

    /// Query this tenant's knowledge base.
    async fn query(&self, query: &str) -> anyhow::Result<Vec<String>> {
        println!("Tenant '{}': Processing query: {}", self.tenant_id, query);

        // Get query embedding
        let query_embedding = self.llm_provider.embed_one(query).await?;

        // Search for relevant chunks
        let results = self.vector_storage.query(&query_embedding, 5, None).await?;

        // Retrieve chunk contents
        let mut context = Vec::new();
        for result in results {
            if let Some(chunk_data) = self.kv_storage.get_by_id(&result.id).await? {
                if let Some(content) = chunk_data.get("content").and_then(|c| c.as_str()) {
                    context.push(content.to_string());
                }
            }
        }

        println!(
            "Tenant '{}': Found {} relevant chunks",
            self.tenant_id,
            context.len()
        );

        Ok(context)
    }

    /// Get statistics for this tenant.
    async fn get_stats(&self) -> anyhow::Result<TenantStats> {
        let document_count = self
            .kv_storage
            .keys()
            .await?
            .iter()
            .filter(|k| k.starts_with("doc_"))
            .count();

        let chunk_count = self
            .kv_storage
            .keys()
            .await?
            .iter()
            .filter(|k| k.contains("-chunk-"))
            .count();

        let node_count = self.graph_storage.node_count().await?;
        let edge_count = self.graph_storage.edge_count().await?;

        Ok(TenantStats {
            tenant_id: self.tenant_id.clone(),
            document_count,
            chunk_count,
            node_count,
            edge_count,
        })
    }
}

/// Statistics for a tenant.
#[derive(Debug)]
struct TenantStats {
    tenant_id: String,
    document_count: usize,
    chunk_count: usize,
    node_count: usize,
    edge_count: usize,
}

/// Multi-tenant RAG manager.
struct MultiTenantRAG {
    tenants: std::collections::HashMap<String, TenantRAG>,
}

impl MultiTenantRAG {
    fn new() -> Self {
        Self {
            tenants: std::collections::HashMap::new(),
        }
    }

    /// Create or get a tenant.
    fn get_or_create_tenant(&mut self, tenant_id: &str) -> &mut TenantRAG {
        self.tenants
            .entry(tenant_id.to_string())
            .or_insert_with(|| TenantRAG::new(tenant_id))
    }

    /// Get an existing tenant.
    fn get_tenant(&self, tenant_id: &str) -> Option<&TenantRAG> {
        self.tenants.get(tenant_id)
    }

    /// List all tenant IDs.
    fn list_tenants(&self) -> Vec<&str> {
        self.tenants.keys().map(|s| s.as_str()).collect()
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== EdgeQuake Multi-Tenant Example ===\n");

    // Create multi-tenant manager
    let mut manager = MultiTenantRAG::new();

    // === Tenant A: Tech Company ===
    println!("--- Setting up Tenant A (TechCorp) ---");
    let tenant_a = manager.get_or_create_tenant("techcorp");
    tenant_a.initialize().await?;

    tenant_a
        .ingest_document(
            "tech-doc-1",
            "TechCorp is a leading software company founded in 2010. \
             The company specializes in cloud computing and AI solutions. \
             Their flagship product, CloudAI, serves over 10,000 customers worldwide.",
        )
        .await?;

    tenant_a
        .ingest_document(
            "tech-doc-2",
            "TechCorp recently announced a partnership with GlobalData Inc. \
             The partnership will focus on developing next-generation data analytics tools.",
        )
        .await?;

    // === Tenant B: Healthcare Company ===
    println!("\n--- Setting up Tenant B (HealthPlus) ---");
    let tenant_b = manager.get_or_create_tenant("healthplus");
    tenant_b.initialize().await?;

    tenant_b
        .ingest_document(
            "health-doc-1",
            "HealthPlus is a healthcare technology company established in 2015. \
             They provide telemedicine solutions and electronic health records systems. \
             Their platform is used by over 500 hospitals and clinics.",
        )
        .await?;

    tenant_b
        .ingest_document(
            "health-doc-2",
            "HealthPlus recently received FDA approval for their AI diagnostic tool. \
             The tool can analyze medical images to detect early signs of disease.",
        )
        .await?;

    // === Query each tenant ===
    println!("\n--- Querying Tenant A ---");
    let tenant_a = manager.get_tenant("techcorp").unwrap();
    let results_a = tenant_a.query("What does TechCorp do?").await?;
    println!("TechCorp results:");
    for (i, result) in results_a.iter().enumerate() {
        println!("  {}. {}", i + 1, &result[..result.len().min(100)]);
    }

    println!("\n--- Querying Tenant B ---");
    let tenant_b = manager.get_tenant("healthplus").unwrap();
    let results_b = tenant_b
        .query("What did HealthPlus get approved for?")
        .await?;
    println!("HealthPlus results:");
    for (i, result) in results_b.iter().enumerate() {
        println!("  {}. {}", i + 1, &result[..result.len().min(100)]);
    }

    // === Verify data isolation ===
    println!("\n--- Verifying Data Isolation ---");

    // Query TechCorp for HealthPlus data (should return nothing relevant)
    let tenant_a = manager.get_tenant("techcorp").unwrap();
    let cross_results = tenant_a.query("HealthPlus telemedicine FDA").await?;
    println!(
        "TechCorp queried for HealthPlus data: {} results (should be 0 or irrelevant)",
        cross_results.len()
    );

    // === Print tenant statistics ===
    println!("\n--- Tenant Statistics ---");
    for tenant_id in manager.list_tenants() {
        if let Some(tenant) = manager.get_tenant(tenant_id) {
            let stats = tenant.get_stats().await?;
            println!(
                "{}: {} docs, {} chunks, {} nodes, {} edges",
                stats.tenant_id,
                stats.document_count,
                stats.chunk_count,
                stats.node_count,
                stats.edge_count
            );
        }
    }

    println!("\n=== Multi-Tenant Example Complete ===");

    Ok(())
}
