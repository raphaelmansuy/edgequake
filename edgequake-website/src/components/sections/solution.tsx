import { StaggerContainer, StaggerItem } from "@/components/animations";
import { Card, CardDescription, CardFooter, CardHeader, CardTitle } from "@/components/ui/card";
import { ArrowRight, Bot, Brain, Building2, FileText, GitFork, Zap } from "lucide-react";
import Link from "next/link";

const features = [
  {
    icon: Brain,
    title: "Knowledge Graph Engine",
    description: "Extract entities and relationships from documents automatically. Build structured knowledge from unstructured text.",
    link: "/docs/concepts/",
    linkLabel: "Learn More",
  },
  {
    icon: Zap,
    title: "10x Faster",
    description: "Rust core delivers 1000+ docs/min throughput. Sub-second query latency at scale. 300MB memory per core.",
    link: "/docs/concepts/",
    linkLabel: "See Benchmarks",
  },
  {
    icon: GitFork,
    title: "6 Query Modes",
    description: "Naive, Local, Global, Hybrid, Mix, and Bypass. Choose the right strategy for every question type.",
    link: "/docs/concepts/",
    linkLabel: "Explore Modes",
  },
  {
    icon: Building2,
    title: "Multi-Tenant",
    description: "Workspace-level isolation with per-tenant storage, LLM config, and access control. Enterprise-ready from day one.",
    link: "/enterprise/",
    linkLabel: "Enterprise",
  },
  {
    icon: FileText,
    title: "PDF Vision Pipeline",
    description: "Embedded pdfium + LLM vision for accurate table and layout extraction. No external dependencies needed.",
    link: "/docs/concepts/",
    linkLabel: "PDF Guide",
  },
  {
    icon: Bot,
    title: "MCP Integration",
    description: "18 Model Context Protocol tools. Let AI agents query your knowledge graph directly. Claude, GPT, and more.",
    link: "/docs/concepts/",
    linkLabel: "MCP Docs",
  },
];

export function SolutionSection() {
  return (
    <section id="solution" className="py-24 sm:py-32 border-t border-border">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div className="max-w-2xl mx-auto text-center mb-16">
          <p className="text-sm font-medium text-accent uppercase tracking-widest mb-3">
            The Solution
          </p>
          <h2 className="text-3xl sm:text-4xl font-bold tracking-tight">
            Everything you need.{" "}
            <span className="text-muted-foreground">Nothing you don&apos;t.</span>
          </h2>
          <p className="mt-4 text-lg text-muted-foreground leading-relaxed">
            EdgeQuake combines knowledge graphs, vector search, and multi-modal query
            modes into a single Rust-based framework.
          </p>
        </div>

        <StaggerContainer className="grid sm:grid-cols-2 lg:grid-cols-3 gap-6">
          {features.map((feature) => (
            <StaggerItem key={feature.title}>
              <Card className="h-full flex flex-col border-border bg-card transition-colors hover:border-accent/30">
                <CardHeader className="flex-1">
                  <div className="flex items-center gap-3 mb-2">
                    <div className="p-2 rounded-lg bg-secondary">
                      <feature.icon className="h-5 w-5 text-foreground" />
                    </div>
                    <CardTitle className="text-base">{feature.title}</CardTitle>
                  </div>
                  <CardDescription className="text-sm leading-relaxed">
                    {feature.description}
                  </CardDescription>
                </CardHeader>
                <CardFooter>
                  <Link
                    href={feature.link}
                    className="text-sm font-medium text-accent hover:text-accent/80 inline-flex items-center gap-1 transition-colors"
                  >
                    {feature.linkLabel} <ArrowRight className="h-3 w-3" />
                  </Link>
                </CardFooter>
              </Card>
            </StaggerItem>
          ))}
        </StaggerContainer>
      </div>
    </section>
  );
}
