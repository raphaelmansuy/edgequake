import { ArrowRight, Book, Code2, ExternalLink, GitCompare, Layers, Rocket, Server } from "lucide-react";
import type { Metadata } from "next";
import Link from "next/link";

export const metadata: Metadata = {
  title: "Documentation — EdgeQuake",
  description: "Learn how to build knowledge graphs with EdgeQuake. Guides, API reference, architecture docs, and deployment tutorials.",
};

const categories = [
  {
    icon: Rocket,
    title: "Getting Started",
    description: "Install EdgeQuake, run your first ingestion, and query your knowledge graph in minutes.",
    links: [
      { label: "Installation", href: "https://github.com/raphaelmansuy/edgequake/blob/main/docs/getting-started/installation.md" },
      { label: "Quick Start", href: "https://github.com/raphaelmansuy/edgequake/blob/main/docs/getting-started/quick-start.md" },
      { label: "Docker Quick Start", href: "https://github.com/raphaelmansuy/edgequake/blob/main/DOCKER_QUICK_START.md" },
    ],
  },
  {
    icon: Book,
    title: "Core Concepts",
    description: "Understand Graph-RAG, entity extraction, knowledge graphs, and the 6 query modes.",
    links: [
      { label: "Graph-RAG Explained", href: "https://github.com/raphaelmansuy/edgequake/blob/main/docs/concepts/graph-rag.md" },
      { label: "Entity Extraction", href: "https://github.com/raphaelmansuy/edgequake/blob/main/docs/concepts/entity-extraction.md" },
      { label: "Query Modes", href: "https://github.com/raphaelmansuy/edgequake/blob/main/docs/concepts/query-modes.md" },
    ],
  },
  {
    icon: Layers,
    title: "Architecture",
    description: "Deep dive into the crate structure, data flow, storage adapters, and pipeline architecture.",
    links: [
      { label: "System Overview", href: "https://github.com/raphaelmansuy/edgequake/blob/main/docs/architecture/overview.md" },
      { label: "Data Flow", href: "https://github.com/raphaelmansuy/edgequake/blob/main/docs/architecture/data-flow.md" },
      { label: "Crate Map", href: "https://github.com/raphaelmansuy/edgequake/blob/main/docs/architecture/crates.md" },
    ],
  },
  {
    icon: Code2,
    title: "API Reference",
    description: "REST API endpoints for documents, queries, workspaces, and graph operations.",
    links: [
      { label: "Documents API", href: "https://github.com/raphaelmansuy/edgequake/blob/main/docs/api-reference/documents.md" },
      { label: "Query API", href: "https://github.com/raphaelmansuy/edgequake/blob/main/docs/api-reference/query.md" },
      { label: "Graph API", href: "https://github.com/raphaelmansuy/edgequake/blob/main/docs/api-reference/graph.md" },
    ],
  },
  {
    icon: Server,
    title: "Deployment",
    description: "Run EdgeQuake in production with Docker, Kubernetes, or bare metal. Configuration and monitoring guides.",
    links: [
      { label: "Docker Compose", href: "https://github.com/raphaelmansuy/edgequake/blob/main/docs/operations/docker.md" },
      { label: "Configuration", href: "https://github.com/raphaelmansuy/edgequake/blob/main/docs/operations/configuration.md" },
      { label: "Production Guide", href: "https://github.com/raphaelmansuy/edgequake/blob/main/docs/operations/production.md" },
    ],
  },
  {
    icon: GitCompare,
    title: "Comparisons",
    description: "See how EdgeQuake compares to traditional RAG, Microsoft GraphRAG, and LightRAG.",
    links: [
      { label: "vs Traditional RAG", href: "https://github.com/raphaelmansuy/edgequake/blob/main/docs/comparisons/vs-traditional-rag.md" },
      { label: "vs GraphRAG", href: "https://github.com/raphaelmansuy/edgequake/blob/main/docs/comparisons/vs-graphrag.md" },
      { label: "vs LightRAG", href: "https://github.com/raphaelmansuy/edgequake/blob/main/docs/comparisons/vs-lightrag.md" },
    ],
  },
];

export default function DocsPage() {
  return (
    <div className="min-h-screen">
      <section className="py-24 sm:py-32">
        <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
          <div className="max-w-2xl mx-auto text-center mb-16">
            <p className="text-sm font-medium text-accent uppercase tracking-widest mb-3">
              Documentation
            </p>
            <h1 className="text-4xl sm:text-5xl font-bold tracking-tight">
              Learn EdgeQuake
            </h1>
            <p className="mt-4 text-lg text-muted-foreground leading-relaxed">
              Everything you need to build, deploy, and scale knowledge graphs with EdgeQuake.
            </p>
          </div>

          <div className="grid sm:grid-cols-2 lg:grid-cols-3 gap-6">
            {categories.map((category) => (
              <div
                key={category.title}
                className="rounded-xl border border-border bg-card p-6"
              >
                <div className="flex items-center gap-3 mb-4">
                  <div className="p-2 rounded-lg bg-secondary">
                    <category.icon className="h-5 w-5 text-foreground" />
                  </div>
                  <h2 className="text-lg font-semibold">{category.title}</h2>
                </div>
                <p className="text-sm text-muted-foreground mb-4 leading-relaxed">
                  {category.description}
                </p>
                <ul className="space-y-2">
                  {category.links.map((link) => (
                    <li key={link.label}>
                      <Link
                        href={link.href}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="text-sm text-accent hover:text-accent/80 inline-flex items-center gap-1.5 transition-colors"
                      >
                        {link.label}
                        <ExternalLink className="h-3 w-3" />
                      </Link>
                    </li>
                  ))}
                </ul>
              </div>
            ))}
          </div>

          <div className="text-center mt-16">
            <Link
              href="https://github.com/raphaelmansuy/edgequake/tree/main/docs"
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex items-center gap-2 text-sm font-medium text-muted-foreground hover:text-foreground transition-colors"
            >
              Browse all docs on GitHub <ArrowRight className="h-4 w-4" />
            </Link>
          </div>
        </div>
      </section>
    </div>
  );
}
