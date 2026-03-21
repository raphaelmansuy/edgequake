"use client";

import { FadeIn, StaggerContainer, StaggerItem } from "@/components/animations";

const throughputData = [
  { label: "EdgeQuake", value: 1000, max: 1000, color: "bg-accent" },
  { label: "LightRAG (Python)", value: 100, max: 1000, color: "bg-muted-foreground/30" },
  { label: "GraphRAG", value: 50, max: 1000, color: "bg-muted-foreground/30" },
  { label: "Traditional RAG", value: 200, max: 1000, color: "bg-muted-foreground/30" },
];

const memoryData = [
  { label: "EdgeQuake", value: 300, unit: "MB", max: 3000, color: "bg-accent" },
  { label: "LightRAG", value: 3000, unit: "MB", max: 3000, color: "bg-muted-foreground/30" },
  { label: "GraphRAG", value: 1500, unit: "MB", max: 3000, color: "bg-muted-foreground/30" },
];

const stats = [
  { value: "10x", label: "Faster ingestion" },
  { value: "300MB", label: "Memory per core" },
  { value: "<100ms", label: "Query latency (p95)" },
  { value: "6", label: "Query modes" },
];

export function BenchmarksSection() {
  return (
    <section id="benchmarks" className="py-24 sm:py-32 border-t border-border">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div className="max-w-2xl mx-auto text-center mb-16">
          <p className="text-sm font-medium text-muted-foreground uppercase tracking-widest mb-3">
            Performance
          </p>
          <h2 className="text-3xl sm:text-4xl font-bold tracking-tight">
            Built for speed.{" "}
            <span className="text-muted-foreground">Obsessively optimized.</span>
          </h2>
          <p className="mt-4 text-lg text-muted-foreground leading-relaxed">
            Compiled Rust, zero-copy parsing, and PostgreSQL-native storage for production-grade throughput.
          </p>
        </div>

        {/* Stats Row */}
        <StaggerContainer className="grid grid-cols-2 sm:grid-cols-4 gap-8 mb-20">
          {stats.map((stat) => (
            <StaggerItem key={stat.label}>
              <div className="text-center">
                <div className="text-4xl sm:text-5xl font-bold tracking-tight">{stat.value}</div>
                <div className="text-sm text-muted-foreground mt-2">{stat.label}</div>
              </div>
            </StaggerItem>
          ))}
        </StaggerContainer>

        {/* Charts */}
        <div className="grid md:grid-cols-2 gap-8">
          {/* Throughput */}
          <FadeIn>
            <div className="border border-border rounded-xl p-6">
              <h3 className="text-lg font-semibold mb-6">Throughput (docs/min)</h3>
              <div className="space-y-5">
                {throughputData.map((item) => (
                  <div key={item.label}>
                    <div className="flex justify-between text-sm mb-1.5">
                      <span className="text-muted-foreground">{item.label}</span>
                      <span className="font-mono font-medium">{item.value}</span>
                    </div>
                    <div className="h-2 bg-secondary rounded-full overflow-hidden">
                      <div
                        className={`h-full rounded-full ${item.color} transition-all duration-1000`}
                        style={{ width: `${(item.value / item.max) * 100}%` }}
                      />
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </FadeIn>

          {/* Memory */}
          <FadeIn>
            <div className="border border-border rounded-xl p-6">
              <h3 className="text-lg font-semibold mb-6">Memory per Core</h3>
              <div className="space-y-5">
                {memoryData.map((item) => (
                  <div key={item.label}>
                    <div className="flex justify-between text-sm mb-1.5">
                      <span className="text-muted-foreground">{item.label}</span>
                      <span className="font-mono font-medium">
                        {item.value >= 1000 ? `${item.value / 1000}GB` : `${item.value}${item.unit}`}
                      </span>
                    </div>
                    <div className="h-2 bg-secondary rounded-full overflow-hidden">
                      <div
                        className={`h-full rounded-full ${item.color} transition-all duration-1000`}
                        style={{ width: `${(item.value / item.max) * 100}%` }}
                      />
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </FadeIn>
        </div>
      </div>
    </section>
  );
}
