/**
 * SPEC-168 realistic demo data (deterministic) for visual inspection.
 * Pure data + builders — no Playwright imports.
 */
import type {
  FixtureEdge,
  FixtureGraph,
  FixtureNode,
} from "../../../src/lib/fixtures/graph";

const WS = "spec155-ws-001";

type NodeSpec = [name: string, type: string, community: number, blurb: string];

const NODES: NodeSpec[] = [
  ["LightRAG", "METHOD", 0, "Graph-based retrieval-augmented generation framework with dual-level retrieval."],
  ["GraphRAG", "METHOD", 0, "Microsoft approach that builds community summaries over an entity graph."],
  ["Naive RAG", "METHOD", 0, "Vector-only retrieval baseline without graph structure."],
  ["Dual-Level Retrieval", "CONCEPT", 0, "Combines low-level entity lookups with high-level thematic keywords."],
  ["Knowledge Graph", "CONCEPT", 0, "Typed entities and relationships extracted from source documents."],
  ["Entity Extraction", "CONCEPT", 0, "LLM-driven identification of entities and relations in text chunks."],
  ["Community Detection", "METHOD", 0, "Leiden or Louvain clustering to group related entities."],
  ["Incremental Update", "CONCEPT", 0, "Merging new documents into an existing graph without a rebuild."],
  ["Zirui Guo", "PERSON", 1, "First author of the LightRAG paper."],
  ["Chao Huang", "PERSON", 1, "Senior author, University of Hong Kong."],
  ["Lianghao Xia", "PERSON", 1, "Co-author, retrieval and recommendation researcher."],
  ["University of Hong Kong", "ORGANIZATION", 1, "Research institution behind LightRAG."],
  ["Beijing University of Posts and Telecommunications", "ORGANIZATION", 1, "Co-affiliation of the authors."],
  ["Microsoft Research", "ORGANIZATION", 1, "Authors of the GraphRAG paper."],
  ["OpenAI", "ORGANIZATION", 2, "Provider of GPT and embedding models."],
  ["GPT-4o-mini", "TECHNOLOGY", 2, "Default extraction model used in the LightRAG evaluation."],
  ["text-embedding-3-small", "TECHNOLOGY", 2, "1536-dimension embedding model."],
  ["Ollama", "TECHNOLOGY", 2, "Local model runner used for private deployments."],
  ["PostgreSQL", "TECHNOLOGY", 2, "Primary store with pgvector and Apache AGE extensions."],
  ["pgvector", "TECHNOLOGY", 2, "Vector similarity extension for PostgreSQL."],
  ["Apache AGE", "TECHNOLOGY", 2, "Graph extension that adds openCypher to PostgreSQL."],
  ["Neo4j", "TECHNOLOGY", 2, "Native graph database used by other GraphRAG stacks."],
  ["Agriculture", "DOMAIN", 3, "Evaluation corpus: farming practice and crop science."],
  ["Computer Science", "DOMAIN", 3, "Evaluation corpus: programming and systems."],
  ["Legal", "DOMAIN", 3, "Evaluation corpus: contracts and case law."],
  ["Mix", "DOMAIN", 3, "Evaluation corpus: mixed topics for cross-domain tests."],
  ["UltraDomain", "DATASET", 3, "Benchmark suite spanning four specialist domains."],
  ["Comprehensiveness", "METRIC", 4, "Does the answer cover all aspects of the question?"],
  ["Diversity", "METRIC", 4, "How varied are the perspectives in the answer?"],
  ["Empowerment", "METRIC", 4, "Does the answer help the reader judge and decide?"],
  ["Overall Win Rate", "METRIC", 4, "Aggregate head-to-head preference against a baseline."],
  ["Token Cost", "METRIC", 4, "Tokens spent at index time and at query time."],
  ["Chunking", "CONCEPT", 5, "Splitting source text into overlapping token windows."],
  ["Vector Index", "CONCEPT", 5, "Approximate nearest-neighbour index over chunk embeddings."],
  ["Reranking", "METHOD", 5, "Second-stage scoring of retrieved candidates."],
  ["Citation", "CONCEPT", 5, "Pointer from an answer span to a source passage."],
  ["Hybrid Search", "METHOD", 5, "Blends lexical and semantic signals."],
  ["Query Mode", "CONCEPT", 5, "Local, global, hybrid, naive or mix retrieval strategies."],
  ["Keyword Extraction", "METHOD", 0, "High and low level keywords derived from the question."],
  ["Description Merge", "METHOD", 0, "LLM summarisation of duplicate entity descriptions."],
  ["Sparse Retrieval", "METHOD", 5, "BM25 style term matching."],
];

const EDGES: Array<[string, string, string]> = [
  ["LightRAG", "Dual-Level Retrieval", "IMPLEMENTS"],
  ["LightRAG", "Knowledge Graph", "BUILDS"],
  ["LightRAG", "Incremental Update", "SUPPORTS"],
  ["LightRAG", "GraphRAG", "OUTPERFORMS"],
  ["LightRAG", "Naive RAG", "OUTPERFORMS"],
  ["LightRAG", "Entity Extraction", "USES"],
  ["LightRAG", "Keyword Extraction", "USES"],
  ["LightRAG", "Description Merge", "USES"],
  ["LightRAG", "UltraDomain", "EVALUATED_ON"],
  ["LightRAG", "Zirui Guo", "AUTHORED_BY"],
  ["LightRAG", "Chao Huang", "AUTHORED_BY"],
  ["LightRAG", "Lianghao Xia", "AUTHORED_BY"],
  ["LightRAG", "Query Mode", "OFFERS"],
  ["GraphRAG", "Community Detection", "USES"],
  ["GraphRAG", "Knowledge Graph", "BUILDS"],
  ["GraphRAG", "Microsoft Research", "CREATED_BY"],
  ["GraphRAG", "Token Cost", "HIGH"],
  ["Naive RAG", "Vector Index", "USES"],
  ["Naive RAG", "Chunking", "USES"],
  ["Dual-Level Retrieval", "Keyword Extraction", "REQUIRES"],
  ["Dual-Level Retrieval", "Vector Index", "QUERIES"],
  ["Dual-Level Retrieval", "Knowledge Graph", "TRAVERSES"],
  ["Knowledge Graph", "Entity Extraction", "DERIVED_FROM"],
  ["Knowledge Graph", "Apache AGE", "STORED_IN"],
  ["Knowledge Graph", "Neo4j", "STORED_IN"],
  ["Entity Extraction", "GPT-4o-mini", "PERFORMED_BY"],
  ["Entity Extraction", "Chunking", "OPERATES_ON"],
  ["Description Merge", "GPT-4o-mini", "PERFORMED_BY"],
  ["Zirui Guo", "University of Hong Kong", "AFFILIATED_WITH"],
  ["Chao Huang", "University of Hong Kong", "AFFILIATED_WITH"],
  ["Lianghao Xia", "University of Hong Kong", "AFFILIATED_WITH"],
  ["Zirui Guo", "Beijing University of Posts and Telecommunications", "AFFILIATED_WITH"],
  ["Zirui Guo", "Chao Huang", "COLLABORATES_WITH"],
  ["Chao Huang", "Lianghao Xia", "COLLABORATES_WITH"],
  ["OpenAI", "GPT-4o-mini", "PROVIDES"],
  ["OpenAI", "text-embedding-3-small", "PROVIDES"],
  ["Ollama", "GPT-4o-mini", "ALTERNATIVE_TO"],
  ["text-embedding-3-small", "Vector Index", "POPULATES"],
  ["PostgreSQL", "pgvector", "EXTENDED_BY"],
  ["PostgreSQL", "Apache AGE", "EXTENDED_BY"],
  ["pgvector", "Vector Index", "PROVIDES"],
  ["UltraDomain", "Agriculture", "CONTAINS"],
  ["UltraDomain", "Computer Science", "CONTAINS"],
  ["UltraDomain", "Legal", "CONTAINS"],
  ["UltraDomain", "Mix", "CONTAINS"],
  ["UltraDomain", "Comprehensiveness", "SCORED_ON"],
  ["UltraDomain", "Diversity", "SCORED_ON"],
  ["UltraDomain", "Empowerment", "SCORED_ON"],
  ["UltraDomain", "Overall Win Rate", "SCORED_ON"],
  ["Comprehensiveness", "Overall Win Rate", "CONTRIBUTES_TO"],
  ["Diversity", "Overall Win Rate", "CONTRIBUTES_TO"],
  ["Empowerment", "Overall Win Rate", "CONTRIBUTES_TO"],
  ["Hybrid Search", "Reranking", "FOLLOWED_BY"],
  ["Hybrid Search", "Sparse Retrieval", "COMBINES"],
  ["Hybrid Search", "Vector Index", "COMBINES"],
  ["Query Mode", "Hybrid Search", "INCLUDES"],
  ["Citation", "Chunking", "REFERENCES"],
  ["Reranking", "Citation", "IMPROVES"],
  ["Community Detection", "Knowledge Graph", "PARTITIONS"],
  ["Incremental Update", "Knowledge Graph", "EXTENDS"],
  ["Incremental Update", "Description Merge", "TRIGGERS"],
  ["Microsoft Research", "Naive RAG", "COMPARED_AGAINST"],
];

const slug = (s: string) => s.toUpperCase().replace(/[^A-Z0-9]+/g, "_");
const nodeId = (name: string) => `${WS}::${slug(name)}`;

/** Realistic ~40-node GraphRAG knowledge graph. */
export function buildDemoGraph(): FixtureGraph {
  const nodes: FixtureNode[] = NODES.map(([name, type, c, blurb], i) => ({
    id: nodeId(name),
    label: name,
    node_type: type,
    description: blurb,
    degree: 0,
    community_id: `c${c}`,
    created_at: `2026-10-${String((i % 18) + 1).padStart(2, "0")}T09:00:00Z`,
    properties: { entity_type: type },
  }));
  const ids = new Set(nodes.map((n) => n.id));
  const edges: FixtureEdge[] = EDGES.map(([a, b, rel], i) => {
    if (!ids.has(nodeId(a)) || !ids.has(nodeId(b))) {
      throw new Error(`demo graph: unknown node in edge ${a} -> ${b}`);
    }
    return {
      id: `${nodeId(a)}|${rel}|${nodeId(b)}|${i}`,
      source: nodeId(a),
      target: nodeId(b),
      relationship_type: rel,
      weight: 1 + (i % 4),
      description: `${a} ${rel.toLowerCase().replace(/_/g, " ")} ${b}.`,
      keywords: [rel.toLowerCase()],
    };
  });
  const degree = new Map<string, number>();
  for (const e of edges) {
    degree.set(e.source, (degree.get(e.source) ?? 0) + 1);
    degree.set(e.target, (degree.get(e.target) ?? 0) + 1);
  }
  for (const n of nodes) n.degree = degree.get(n.id) ?? 0;
  return {
    nodes,
    edges,
    is_truncated: false,
    total_nodes: nodes.length,
    total_edges: edges.length,
    max_nodes: 500,
  };
}

const HOUR = 3_600_000;
const iso = (msAgo: number) => new Date(Date.now() - msAgo).toISOString();

type DocSpec = [
  title: string,
  file: string,
  status: string,
  entities: number,
  rels: number,
  chunks: number,
  pages: number,
  hoursAgo: number,
];

const DOCS: DocSpec[] = [
  ["LightRAG: Simple and Fast Retrieval-Augmented Generation", "lightrag_2410.05779.pdf", "completed", 214, 388, 96, 18, 2],
  ["From Local to Global: A Graph RAG Approach to Query-Focused Summarization", "graphrag_2404.16130.pdf", "completed", 187, 301, 84, 15, 5],
  ["Retrieval-Augmented Generation for Knowledge-Intensive NLP Tasks", "rag_2005.11401.pdf", "completed", 142, 219, 71, 19, 9],
  ["Dense Passage Retrieval for Open-Domain Question Answering", "dpr_2004.04906.pdf", "completed", 98, 150, 52, 13, 13],
  ["Self-RAG: Learning to Retrieve, Generate, and Critique", "selfrag_2310.11511.pdf", "processing", 0, 0, 0, 30, 0.2],
  ["Q3 Customer Interview Synthesis", "interviews_q3.md", "completed", 63, 94, 28, 0, 26],
  ["Platform Architecture Decision Records", "adr_platform.md", "completed", 77, 120, 33, 0, 30],
  ["Security Whitepaper 2026", "security_whitepaper.pdf", "pending", 0, 0, 0, 24, 0.05],
  ["Annual Report 2025 (scanned)", "annual_report_2025.pdf", "failed", 0, 0, 0, 112, 40],
  ["Clinical Study Protocol v4", "clinical_protocol_v4.pdf", "partial_failure", 119, 164, 59, 41, 52],
  ["Vendor Contract Master Agreement", "msa_vendor.docx", "completed", 54, 71, 22, 0, 70],
  ["Onboarding Handbook", "onboarding.md", "completed", 41, 58, 17, 0, 96],
];

export function buildDemoDocuments() {
  return DOCS.map(([title, file, status, e, r, c, pages, h], i) => ({
    id: `d0c00000-0000-4000-8000-${String(i + 1).padStart(12, "0")}`,
    title,
    file_name: file,
    status,
    source_type: file.endsWith(".pdf") ? "pdf" : "markdown",
    mime_type: file.endsWith(".pdf") ? "application/pdf" : "text/markdown",
    file_size: 180_000 + i * 97_000,
    page_count: pages || undefined,
    entity_count: e,
    relationship_count: r,
    chunk_count: c,
    tenant_id: "spec155-tenant-001",
    workspace_id: "spec155-ws-001",
    created_at: iso(h * HOUR),
    updated_at: iso(Math.max(0.02, h - 0.5) * HOUR),
    ...(status === "failed"
      ? { error_message: "Vision extraction failed on page 37: provider timeout after 120s" }
      : {}),
    ...(status === "processing"
      ? { current_stage: "extracting", stage_progress: 0.42, stage_message: "Extracting entities 18/43 chunks" }
      : {}),
  }));
}

export function buildCostHistory() {
  const out = [];
  for (let d = 29; d >= 0; d--) {
    const wave = 0.6 + 0.4 * Math.sin(d / 3);
    out.push({
      timestamp: new Date(Date.now() - d * 24 * HOUR).toISOString().slice(0, 10),
      total_cost: Number((wave * 0.42 + (d % 5 === 0 ? 0.35 : 0)).toFixed(4)),
      document_count: Math.round(wave * 4),
      total_tokens: Math.round(wave * 180_000),
    });
  }
  return out;
}

export const DEMO_CONVERSATIONS = [
  ["How does LightRAG differ from GraphRAG?", 0.3],
  ["Summarise the evaluation metrics used in UltraDomain", 3],
  ["Which databases can store the knowledge graph?", 26],
  ["Who are the authors and where do they work?", 50],
] as const;
