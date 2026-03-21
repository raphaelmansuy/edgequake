"use client";

import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Building2, FileText, Globe, Lightbulb, MapPin, Search, Users, Zap } from "lucide-react";
import { useState } from "react";

const queryModes = [
  { id: "naive", label: "Naive", description: "Simple vector similarity search against all chunks." },
  { id: "local", label: "Local", description: "Entity-centered search — finds direct relationships around matched entities." },
  { id: "global", label: "Global", description: "Map-reduce summarization across the entire knowledge graph." },
  { id: "hybrid", label: "Hybrid", description: "Combines local entity context with vector similarity for best results." },
  { id: "mix", label: "Mix", description: "Weighted blend of multiple strategies with automatic mode selection." },
  { id: "bypass", label: "Bypass", description: "Direct LLM call without retrieval — useful for meta-queries." },
];

const sampleEntities = [
  { name: "SARAH CHEN", type: "Person", icon: Users, connections: 5 },
  { name: "ACME CORP", type: "Organization", icon: Building2, connections: 8 },
  { name: "SAN FRANCISCO", type: "Location", icon: MapPin, connections: 3 },
  { name: "GRAPH-RAG", type: "Concept", icon: Lightbulb, connections: 12 },
  { name: "RESEARCH PAPER", type: "Document", icon: FileText, connections: 4 },
  { name: "KNOWLEDGE GRAPH", type: "Concept", icon: Globe, connections: 9 },
];

const sampleQueries = [
  {
    question: "Who are the key researchers and what organizations are they affiliated with?",
    mode: "hybrid",
    answer: `Based on the knowledge graph, **Sarah Chen** is the primary researcher identified in the corpus. She is affiliated with **Acme Corp**, a technology organization headquartered in **San Francisco**.

Her research focuses on **Graph-RAG** techniques and their application to **knowledge graph** construction. She has authored multiple papers on entity extraction and relationship mapping.

**Key relationships:**
- Sarah Chen → works_at → Acme Corp
- Sarah Chen → authored → "Advanced Graph-RAG Techniques" (2025)
- Acme Corp → headquartered_in → San Francisco
- Sarah Chen → researches → Knowledge Graph Construction`,
  },
  {
    question: "What are the main concepts discussed across all documents?",
    mode: "global",
    answer: `The corpus covers several interconnected concepts:

1. **Graph-RAG** — Retrieval-Augmented Generation using knowledge graphs instead of pure vector similarity
2. **Entity Extraction** — LLM-powered identification of persons, organizations, locations, and concepts
3. **Knowledge Graphs** — Structured representation of entities and their relationships
4. **Hybrid Retrieval** — Combining vector search with graph traversal for better context
5. **Multi-Tenancy** — Workspace-level isolation for enterprise deployments

The central theme is the superiority of graph-based retrieval over traditional vector-only approaches.`,
  },
];

export default function DemoPage() {
  const [selectedMode, setSelectedMode] = useState("hybrid");
  const [selectedQuery, setSelectedQuery] = useState(0);

  return (
    <div className="min-h-screen">
      <section className="py-24 sm:py-32">
        <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
          <div className="max-w-2xl mx-auto text-center mb-16">
            <p className="text-sm font-medium text-accent uppercase tracking-widest mb-3">
              Interactive Demo
            </p>
            <h1 className="text-4xl sm:text-5xl font-bold tracking-tight">
              See EdgeQuake in Action
            </h1>
            <p className="mt-4 text-lg text-muted-foreground leading-relaxed">
              Explore pre-computed query results from a sample dataset. See how different
              query modes produce different answers.
            </p>
          </div>

          {/* Query Mode Selector */}
          <div className="mb-8">
            <h2 className="text-sm font-medium text-muted-foreground uppercase tracking-widest mb-4 text-center">
              Query Modes
            </h2>
            <div className="flex flex-wrap justify-center gap-2">
              {queryModes.map((mode) => (
                <button
                  key={mode.id}
                  onClick={() => setSelectedMode(mode.id)}
                  className={`px-4 py-2 rounded-lg text-sm font-medium transition-colors border ${
                    selectedMode === mode.id
                      ? "bg-accent text-accent-foreground border-accent"
                      : "border-border text-muted-foreground hover:text-foreground hover:border-foreground/20"
                  }`}
                >
                  {mode.label}
                </button>
              ))}
            </div>
            <p className="text-center text-sm text-muted-foreground mt-3">
              {queryModes.find((m) => m.id === selectedMode)?.description}
            </p>
          </div>

          {/* Query + Results */}
          <div className="grid lg:grid-cols-2 gap-6">
            {/* Left: Question + Entities */}
            <div className="space-y-6">
              {/* Sample Question */}
              <div className="rounded-xl border border-border p-6">
                <h3 className="text-sm font-semibold mb-4 flex items-center gap-2">
                  <Search className="h-4 w-4 text-accent" /> Sample Query
                </h3>
                <Tabs defaultValue="0" onValueChange={(v) => setSelectedQuery(Number(v))}>
                  <TabsList className="w-full">
                    <TabsTrigger value="0" className="flex-1 text-xs">Query 1</TabsTrigger>
                    <TabsTrigger value="1" className="flex-1 text-xs">Query 2</TabsTrigger>
                  </TabsList>
                  {sampleQueries.map((q, i) => (
                    <TabsContent key={i} value={String(i)}>
                      <p className="text-sm leading-relaxed mt-3">{q.question}</p>
                      <p className="text-xs text-muted-foreground mt-2">
                        Mode: <span className="text-accent font-medium">{q.mode}</span>
                      </p>
                    </TabsContent>
                  ))}
                </Tabs>
              </div>

              {/* Entity List */}
              <div className="rounded-xl border border-border p-6">
                <h3 className="text-sm font-semibold mb-4 flex items-center gap-2">
                  <Zap className="h-4 w-4 text-accent" /> Extracted Entities
                </h3>
                <div className="space-y-3">
                  {sampleEntities.map((entity) => (
                    <div key={entity.name} className="flex items-center justify-between">
                      <div className="flex items-center gap-3">
                        <div className="p-1.5 rounded bg-secondary">
                          <entity.icon className="h-3.5 w-3.5 text-muted-foreground" />
                        </div>
                        <div>
                          <p className="text-sm font-mono font-medium">{entity.name}</p>
                          <p className="text-xs text-muted-foreground">{entity.type}</p>
                        </div>
                      </div>
                      <span className="text-xs text-muted-foreground">{entity.connections} edges</span>
                    </div>
                  ))}
                </div>
              </div>
            </div>

            {/* Right: Answer */}
            <div className="rounded-xl border border-border p-6">
              <h3 className="text-sm font-semibold mb-4">Query Result</h3>
              <div className="prose prose-sm dark:prose-invert max-w-none">
                {sampleQueries[selectedQuery].answer.split("\n").map((line, i) => {
                  if (line.startsWith("**") && line.endsWith("**")) {
                    return <p key={i} className="font-semibold text-sm">{line.replace(/\*\*/g, "")}</p>;
                  }
                  if (line.startsWith("- ")) {
                    return (
                      <p key={i} className="text-sm text-muted-foreground pl-4 font-mono">
                        {line}
                      </p>
                    );
                  }
                  if (line.match(/^\d+\./)) {
                    return <p key={i} className="text-sm leading-relaxed">{line.replace(/\*\*/g, "")}</p>;
                  }
                  if (line.trim() === "") return <br key={i} />;
                  return <p key={i} className="text-sm text-muted-foreground leading-relaxed">{line.replace(/\*\*/g, "")}</p>;
                })}
              </div>
            </div>
          </div>

          <p className="text-center text-sm text-muted-foreground mt-10">
            This demo uses pre-computed results from a sample dataset.{" "}
            <a href="https://github.com/raphaelmansuy/edgequake/blob/main/DOCKER_QUICK_START.md"
               target="_blank" rel="noopener noreferrer"
               className="text-accent hover:text-accent/80">
              Try with your own data →
            </a>
          </p>
        </div>
      </section>
    </div>
  );
}
