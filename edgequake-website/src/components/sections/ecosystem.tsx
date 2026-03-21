"use client";

import { StaggerContainer, StaggerItem } from "@/components/animations";
import { Badge } from "@/components/ui/badge";
import { buttonVariants } from "@/components/ui/button";
import { ArrowRight, Package } from "lucide-react";
import Link from "next/link";

const crates = [
  { name: "edgequake-core", description: "Orchestration & pipeline API", version: "0.7.0" },
  { name: "edgequake-api", description: "Axum REST API + SSE streaming", version: "0.7.0" },
  { name: "edgequake-storage", description: "pgvector + AGE adapters", version: "0.7.0" },
  { name: "edgequake-pipeline", description: "Document processing pipeline", version: "0.7.0" },
  { name: "edgequake-query", description: "6-mode query engine", version: "0.7.0" },
  { name: "edgequake-llm", description: "Multi-provider LLM abstraction", version: "0.3.0" },
  { name: "edgequake-pdf2md", description: "PDF to Markdown with vision", version: "0.7.0" },
  { name: "edgequake-graph", description: "Graph data structures", version: "0.7.0" },
];

export function EcosystemSection() {
  return (
    <section id="ecosystem" className="py-24 sm:py-32 border-t border-border">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div className="max-w-2xl mx-auto text-center mb-16">
          <p className="text-sm font-medium text-muted-foreground uppercase tracking-widest mb-3">
            Ecosystem
          </p>
          <h2 className="text-3xl sm:text-4xl font-bold tracking-tight">
            Modular Crates, Infinite Possibilities
          </h2>
          <p className="mt-4 text-lg text-muted-foreground leading-relaxed">
            Use the full framework or cherry-pick individual crates for your specific needs.
          </p>
        </div>

        <StaggerContainer className="grid sm:grid-cols-2 lg:grid-cols-4 gap-4 mb-10">
          {crates.map((crate) => (
            <StaggerItem key={crate.name}>
              <div className="h-full border border-border rounded-lg p-4 hover:border-accent/40 transition-colors">
                <div className="flex items-start gap-3">
                  <Package className="h-4 w-4 text-muted-foreground mt-1 shrink-0" />
                  <div className="min-w-0">
                    <h3 className="text-sm font-mono font-semibold truncate">{crate.name}</h3>
                    <p className="text-xs text-muted-foreground mt-1">{crate.description}</p>
                    <Badge variant="outline" className="mt-2 text-xs font-mono">
                      v{crate.version}
                    </Badge>
                  </div>
                </div>
              </div>
            </StaggerItem>
          ))}
        </StaggerContainer>

        <div className="text-center">
          <Link href="/ecosystem/" className={buttonVariants({ variant: "outline", className: "gap-2" })}>
              Explore All Crates <ArrowRight className="h-4 w-4" />
          </Link>
        </div>
      </div>
    </section>
  );
}
