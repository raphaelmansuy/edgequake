import { StaggerContainer, StaggerItem } from "@/components/animations";
import { Card, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Building2, DollarSign, FileText, Layers, Link2Off, Timer } from "lucide-react";

const problems = [
  {
    icon: Link2Off,
    title: "Lost Relationships",
    description: "Traditional vector search destroys entity connections. You get fragments, not answers.",
  },
  {
    icon: Timer,
    title: "Slow at Scale",
    description: "Python-based RAG pipelines choke on thousands of documents. Processing becomes the bottleneck.",
  },
  {
    icon: Layers,
    title: "Single Query Mode",
    description: "One retrieval strategy can't serve every question. Simple lookups need different logic than synthesis.",
  },
  {
    icon: Building2,
    title: "No Multi-Tenancy",
    description: "Most RAG frameworks assume one user, one dataset. Enterprise isolation is an afterthought.",
  },
  {
    icon: FileText,
    title: "PDF Pain",
    description: "PDFs are the most common enterprise format, yet most RAG tools can't parse tables or layouts.",
  },
  {
    icon: DollarSign,
    title: "Cloud-Only Cost",
    description: "Vendor lock-in to expensive embedding APIs. No option to run locally or control costs.",
  },
];

export function ProblemSection() {
  return (
    <section id="problem" className="py-24 sm:py-32 border-t border-border">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div className="max-w-2xl mx-auto text-center mb-16">
          <p className="text-sm font-medium text-muted-foreground uppercase tracking-widest mb-3">
            The Problem
          </p>
          <h2 className="text-3xl sm:text-4xl font-bold tracking-tight">
            Traditional RAG Loses Knowledge
          </h2>
          <p className="mt-4 text-lg text-muted-foreground leading-relaxed">
            Vector-only retrieval breaks the connections between entities.
            You need structure, not just similarity.
          </p>
        </div>

        <StaggerContainer className="grid sm:grid-cols-2 lg:grid-cols-3 gap-6">
          {problems.map((problem) => (
            <StaggerItem key={problem.title}>
              <Card className="h-full border-border bg-card">
                <CardHeader>
                  <div className="flex items-center gap-3 mb-2">
                    <div className="p-2 rounded-lg bg-secondary">
                      <problem.icon className="h-5 w-5 text-muted-foreground" />
                    </div>
                    <CardTitle className="text-base">{problem.title}</CardTitle>
                  </div>
                  <CardDescription className="text-sm leading-relaxed">{problem.description}</CardDescription>
                </CardHeader>
              </Card>
            </StaggerItem>
          ))}
        </StaggerContainer>
      </div>
    </section>
  );
}
