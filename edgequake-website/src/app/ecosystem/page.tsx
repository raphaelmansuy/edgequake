import { Badge } from "@/components/ui/badge";
import { Bot, ExternalLink, GitBranch, Package } from "lucide-react";
import type { Metadata } from "next";
import Link from "next/link";

export const metadata: Metadata = {
  title: "Ecosystem — EdgeQuake",
  description: "Explore EdgeQuake's modular Rust crates, MCP tools, and integrations. Use the full framework or cherry-pick individual components.",
};

const crates = [
  {
    name: "edgequake-core",
    version: "0.7.0",
    description: "Orchestration layer with pipeline management, workspace context, and EdgeQuake API. The main entry point for the framework.",
    deps: ["edgequake-llm", "edgequake-storage", "edgequake-pipeline", "edgequake-query"],
    category: "Core",
  },
  {
    name: "edgequake-api",
    version: "0.7.0",
    description: "Axum-based REST API with SSE streaming, multi-workspace support, document management, and query endpoints.",
    deps: ["edgequake-core", "axum", "tokio"],
    category: "API",
  },
  {
    name: "edgequake-storage",
    version: "0.7.0",
    description: "Storage adapters for PostgreSQL with pgvector and Apache AGE. KV, vector, and graph storage unified.",
    deps: ["sqlx", "pgvector"],
    category: "Storage",
  },
  {
    name: "edgequake-pipeline",
    version: "0.7.0",
    description: "Document processing pipeline with entity extraction, relationship mapping, and graph construction.",
    deps: ["edgequake-llm", "edgequake-storage"],
    category: "Pipeline",
  },
  {
    name: "edgequake-query",
    version: "0.7.0",
    description: "6-mode query engine: Naive, Local, Global, Hybrid, Mix, and Bypass. Automatic mode selection available.",
    deps: ["edgequake-storage", "edgequake-llm"],
    category: "Query",
  },
  {
    name: "edgequake-llm",
    version: "0.3.0",
    description: "Multi-provider LLM abstraction supporting OpenAI, Ollama, LM Studio, and mock providers. Hot-swappable at runtime.",
    deps: ["reqwest", "async-trait"],
    category: "LLM",
  },
  {
    name: "edgequake-pdf2md",
    version: "0.7.0",
    description: "PDF to Markdown converter with embedded pdfium and LLM vision for accurate table and layout extraction.",
    deps: ["pdfium-auto", "edgequake-llm"],
    category: "Pipeline",
  },
  {
    name: "edgequake-graph",
    version: "0.7.0",
    description: "Graph data structures, traversal algorithms, and community detection. Used by storage and query crates.",
    deps: [],
    category: "Core",
  },
];

const mcpTools = [
  "query — Query the knowledge graph",
  "document_upload — Upload documents for processing",
  "document_list — List all documents in workspace",
  "document_get — Get document details and content",
  "document_status — Check processing status",
  "document_delete — Remove a document",
  "graph_search_entities — Search entities by name or type",
  "graph_get_entity — Get entity with relationships",
  "graph_entity_neighborhood — Explore entity neighborhood",
  "graph_search_relationships — Search relationships",
  "workspace_create — Create a new workspace",
  "workspace_get — Get workspace details",
  "workspace_list — List all workspaces",
  "workspace_stats — Get workspace statistics",
  "workspace_delete — Delete a workspace",
  "health — Check service health",
];

export default function EcosystemPage() {
  return (
    <div className="min-h-screen">
      <section className="py-24 sm:py-32">
        <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
          <div className="max-w-2xl mx-auto text-center mb-16">
            <p className="text-sm font-medium text-accent uppercase tracking-widest mb-3">
              Ecosystem
            </p>
            <h1 className="text-4xl sm:text-5xl font-bold tracking-tight">
              Modular by Design
            </h1>
            <p className="mt-4 text-lg text-muted-foreground leading-relaxed">
              Use the full framework or cherry-pick individual crates. Each component
              is independently versioned and documented.
            </p>
          </div>

          {/* Crates Grid */}
          <div className="mb-20">
            <h2 className="text-sm font-medium text-muted-foreground uppercase tracking-widest mb-6">
              Rust Crates
            </h2>
            <div className="grid sm:grid-cols-2 gap-4">
              {crates.map((crate) => (
                <div
                  key={crate.name}
                  className="rounded-xl border border-border p-5 hover:border-accent/30 transition-colors"
                >
                  <div className="flex items-start justify-between mb-3">
                    <div className="flex items-center gap-2.5">
                      <Package className="h-4 w-4 text-accent shrink-0" />
                      <h3 className="font-mono font-semibold text-sm">{crate.name}</h3>
                    </div>
                    <Badge variant="outline" className="text-xs font-mono shrink-0">v{crate.version}</Badge>
                  </div>
                  <p className="text-sm text-muted-foreground leading-relaxed mb-3">
                    {crate.description}
                  </p>
                  {crate.deps.length > 0 && (
                    <div className="flex flex-wrap gap-1.5">
                      {crate.deps.map((dep) => (
                        <span key={dep} className="text-xs text-muted-foreground bg-secondary px-2 py-0.5 rounded font-mono">
                          {dep}
                        </span>
                      ))}
                    </div>
                  )}
                </div>
              ))}
            </div>
          </div>

          {/* MCP Tools */}
          <div className="mb-20">
            <div className="flex items-center gap-3 mb-6">
              <Bot className="h-5 w-5 text-accent" />
              <h2 className="text-sm font-medium text-muted-foreground uppercase tracking-widest">
                MCP Tools ({mcpTools.length})
              </h2>
            </div>
            <div className="rounded-xl border border-border p-6">
              <p className="text-sm text-muted-foreground mb-4 leading-relaxed">
                EdgeQuake exposes {mcpTools.length} Model Context Protocol tools, enabling AI agents
                (Claude, GPT, etc.) to interact with your knowledge graph directly.
              </p>
              <div className="grid sm:grid-cols-2 gap-2">
                {mcpTools.map((tool) => (
                  <div key={tool} className="text-sm font-mono">
                    <span className="text-accent">{tool.split(" — ")[0]}</span>
                    <span className="text-muted-foreground"> — {tool.split(" — ")[1]}</span>
                  </div>
                ))}
              </div>
            </div>
          </div>

          {/* Dependency Graph */}
          <div className="mb-16">
            <div className="flex items-center gap-3 mb-6">
              <GitBranch className="h-5 w-5 text-accent" />
              <h2 className="text-sm font-medium text-muted-foreground uppercase tracking-widest">
                Dependency Graph
              </h2>
            </div>
            <div className="rounded-xl border border-border bg-card p-6 overflow-x-auto">
              <pre className="text-sm font-mono text-muted-foreground leading-loose whitespace-pre">
{`edgequake-api
├── edgequake-core
│   ├── edgequake-pipeline
│   │   ├── edgequake-llm
│   │   └── edgequake-storage
│   ├── edgequake-query
│   │   ├── edgequake-llm
│   │   └── edgequake-storage
│   └── edgequake-graph
├── edgequake-pdf2md
│   └── edgequake-llm
└── axum + tokio (runtime)`}
              </pre>
            </div>
          </div>

          <div className="text-center">
            <Link
              href="https://github.com/raphaelmansuy/edgequake/tree/main/edgequake/crates"
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex items-center gap-2 text-sm font-medium text-muted-foreground hover:text-foreground transition-colors"
            >
              Browse source on GitHub <ExternalLink className="h-4 w-4" />
            </Link>
          </div>
        </div>
      </section>
    </div>
  );
}
