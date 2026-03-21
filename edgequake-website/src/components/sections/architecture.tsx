import { FadeIn } from "@/components/animations";

export function ArchitectureSection() {
  return (
    <section id="architecture" className="py-24 sm:py-32 border-t border-border">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div className="max-w-2xl mx-auto text-center mb-16">
          <p className="text-sm font-medium text-muted-foreground uppercase tracking-widest mb-3">
            Architecture
          </p>
          <h2 className="text-3xl sm:text-4xl font-bold tracking-tight">
            How It All Fits Together
          </h2>
          <p className="mt-4 text-lg text-muted-foreground leading-relaxed">
            A modular pipeline from document ingestion to intelligent query response.
          </p>
        </div>

        <FadeIn>
          <div className="bg-background border border-border rounded-xl p-6 sm:p-10 max-w-4xl mx-auto overflow-x-auto">
            <svg viewBox="0 0 800 340" className="w-full h-auto" role="img" aria-label="EdgeQuake Architecture Diagram">
              {/* Definitions */}
              <defs>
                <marker id="arrowhead" markerWidth="10" markerHeight="7" refX="10" refY="3.5" orient="auto">
                  <polygon points="0 0, 10 3.5, 0 7" className="fill-primary" />
                </marker>
                <filter id="glow">
                  <feGaussianBlur stdDeviation="2" result="blur" />
                  <feMerge>
                    <feMergeNode in="blur" />
                    <feMergeNode in="SourceGraphic" />
                  </feMerge>
                </filter>
              </defs>

              {/* Row 1: Documents → Pipeline → Storage */}
              {/* Documents */}
              <rect x="20" y="40" width="140" height="70" rx="8" className="fill-card stroke-border" strokeWidth="1.5" />
              <text x="90" y="68" textAnchor="middle" className="fill-foreground text-xs font-semibold">Documents</text>
              <text x="90" y="88" textAnchor="middle" className="fill-muted-foreground" fontSize="10">PDF, TXT, MD, HTML</text>

              {/* Arrow 1 */}
              <line x1="160" y1="75" x2="210" y2="75" className="stroke-primary" strokeWidth="2" markerEnd="url(#arrowhead)" />

              {/* Pipeline */}
              <rect x="220" y="30" width="160" height="90" rx="8" className="fill-primary/10 stroke-primary/40" strokeWidth="1.5" />
              <text x="300" y="55" textAnchor="middle" className="fill-foreground text-xs font-semibold">Ingestion Pipeline</text>
              <text x="300" y="72" textAnchor="middle" className="fill-muted-foreground" fontSize="10">PDF Vision + Chunking</text>
              <text x="300" y="87" textAnchor="middle" className="fill-muted-foreground" fontSize="10">Entity Extraction</text>
              <text x="300" y="102" textAnchor="middle" className="fill-muted-foreground" fontSize="10">Embedding + Dedup</text>

              {/* Arrow 2 */}
              <line x1="380" y1="75" x2="430" y2="75" className="stroke-primary" strokeWidth="2" markerEnd="url(#arrowhead)" />

              {/* Storage */}
              <rect x="440" y="20" width="180" height="110" rx="8" className="fill-accent/10 stroke-accent/40" strokeWidth="1.5" />
              <text x="530" y="48" textAnchor="middle" className="fill-foreground text-xs font-semibold">PostgreSQL Storage</text>
              <text x="530" y="68" textAnchor="middle" className="fill-muted-foreground" fontSize="10">pgvector (embeddings)</text>
              <text x="530" y="85" textAnchor="middle" className="fill-muted-foreground" fontSize="10">Apache AGE (graph)</text>
              <text x="530" y="102" textAnchor="middle" className="fill-muted-foreground" fontSize="10">KV Store (documents)</text>
              <text x="530" y="119" textAnchor="middle" className="fill-muted-foreground" fontSize="10">Multi-tenant workspaces</text>

              {/* Row 2: Clients ← API ← Query Engine ↔ Storage */}
              {/* Arrow: Storage down to Query Engine */}
              <line x1="530" y1="130" x2="530" y2="190" className="stroke-accent" strokeWidth="2" markerEnd="url(#arrowhead)" />

              {/* Query Engine */}
              <rect x="440" y="200" width="180" height="80" rx="8" className="fill-primary/10 stroke-primary/40" strokeWidth="1.5" />
              <text x="530" y="228" textAnchor="middle" className="fill-foreground text-xs font-semibold">Query Engine</text>
              <text x="530" y="248" textAnchor="middle" className="fill-muted-foreground" fontSize="10">6 Modes: Naive · Local · Global</text>
              <text x="530" y="263" textAnchor="middle" className="fill-muted-foreground" fontSize="10">Hybrid · Mix · Bypass</text>

              {/* Arrow: Query Engine → API */}
              <line x1="440" y1="240" x2="390" y2="240" className="stroke-primary" strokeWidth="2" markerEnd="url(#arrowhead)" />

              {/* REST API */}
              <rect x="220" y="200" width="160" height="80" rx="8" className="fill-card stroke-border" strokeWidth="1.5" />
              <text x="300" y="228" textAnchor="middle" className="fill-foreground text-xs font-semibold">REST API (Axum)</text>
              <text x="300" y="248" textAnchor="middle" className="fill-muted-foreground" fontSize="10">SSE Streaming</text>
              <text x="300" y="263" textAnchor="middle" className="fill-muted-foreground" fontSize="10">18 MCP Tools</text>

              {/* Arrow: API → Clients */}
              <line x1="220" y1="240" x2="170" y2="240" className="stroke-primary" strokeWidth="2" markerEnd="url(#arrowhead)" />

              {/* Clients */}
              <rect x="20" y="200" width="140" height="80" rx="8" className="fill-card stroke-border" strokeWidth="1.5" />
              <text x="90" y="228" textAnchor="middle" className="fill-foreground text-xs font-semibold">Clients</text>
              <text x="90" y="248" textAnchor="middle" className="fill-muted-foreground" fontSize="10">Web UI · CLI · SDK</text>
              <text x="90" y="263" textAnchor="middle" className="fill-muted-foreground" fontSize="10">AI Agents (MCP)</text>

              {/* LLM Provider on the right */}
              <rect x="660" y="100" width="120" height="100" rx="8" className="fill-card stroke-border" strokeWidth="1.5" strokeDasharray="4 2" />
              <text x="720" y="128" textAnchor="middle" className="fill-foreground text-xs font-semibold">LLM Provider</text>
              <text x="720" y="148" textAnchor="middle" className="fill-muted-foreground" fontSize="10">OpenAI</text>
              <text x="720" y="163" textAnchor="middle" className="fill-muted-foreground" fontSize="10">Ollama</text>
              <text x="720" y="178" textAnchor="middle" className="fill-muted-foreground" fontSize="10">LM Studio</text>
              <text x="720" y="193" textAnchor="middle" className="fill-muted-foreground" fontSize="10">Any OpenAI-compat</text>

              {/* Arrow from Pipeline to LLM */}
              <line x1="380" y1="60" x2="660" y2="130" className="stroke-border" strokeWidth="1" strokeDasharray="4 2" />
              {/* Arrow from Query Engine to LLM */}
              <line x1="620" y1="230" x2="660" y2="180" className="stroke-border" strokeWidth="1" strokeDasharray="4 2" />
            </svg>
          </div>
        </FadeIn>
      </div>
    </section>
  );
}
