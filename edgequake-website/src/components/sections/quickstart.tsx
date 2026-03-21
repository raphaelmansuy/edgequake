"use client";

import { FadeIn } from "@/components/animations";
import { buttonVariants } from "@/components/ui/button";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { ArrowRight, Check, Copy } from "lucide-react";
import Link from "next/link";
import { useState } from "react";

const rustCode = `use edgequake_core::EdgeQuake;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let eq = EdgeQuake::builder()
        .database_url("postgres://localhost/edgequake")
        .llm_provider("ollama")
        .build()
        .await?;

    // Ingest a document
    eq.ingest("research-paper.pdf").await?;

    // Query with hybrid mode
    let result = eq.query("What are the key findings?")
        .mode("hybrid")
        .execute()
        .await?;

    println!("{}", result.answer);
    Ok(())
}`;

const dockerCode = `# Start EdgeQuake with Docker Compose
docker compose up -d

# Upload a document
curl -X POST http://localhost:8080/api/v1/documents \\
  -F "file=@research-paper.pdf"

# Query your knowledge graph
curl http://localhost:8080/api/v1/query \\
  -H "Content-Type: application/json" \\
  -d '{"query": "What are the key findings?", "mode": "hybrid"}'`;

const apiCode = `# Upload document via REST API
curl -X POST https://your-server:8080/api/v1/documents \\
  -H "Authorization: Bearer YOUR_TOKEN" \\
  -F "file=@research-paper.pdf" \\
  -F "workspace=my-workspace"

# Query with streaming
curl -N https://your-server:8080/api/v1/query/stream \\
  -H "Content-Type: application/json" \\
  -d '{
    "query": "Summarize the relationship between X and Y",
    "mode": "global",
    "workspace": "my-workspace"
  }'`;

function CodeBlock({ code, lang }: { code: string; lang: string }) {
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    await navigator.clipboard.writeText(code);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="relative group">
      <button
        onClick={handleCopy}
        className="absolute top-3 right-3 p-1.5 rounded-md bg-muted/50 hover:bg-muted text-muted-foreground hover:text-foreground transition-colors opacity-0 group-hover:opacity-100"
        aria-label="Copy code"
      >
        {copied ? <Check className="h-4 w-4" /> : <Copy className="h-4 w-4" />}
      </button>
      <pre className="bg-[#0A0A0F] border border-border rounded-lg p-4 overflow-x-auto text-sm leading-relaxed">
        <code className="font-mono text-[#E8E8E8]">{code}</code>
      </pre>
      <div className="absolute top-3 left-3 text-xs text-muted-foreground font-mono">{lang}</div>
    </div>
  );
}

export function QuickStartSection() {
  return (
    <section id="quickstart" className="py-24 sm:py-32 border-t border-border">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <div className="max-w-2xl mx-auto text-center mb-16">
          <p className="text-sm font-medium text-muted-foreground uppercase tracking-widest mb-3">
            Quick Start
          </p>
          <h2 className="text-3xl sm:text-4xl font-bold tracking-tight">
            Three steps. That&apos;s it.
          </h2>
          <p className="mt-4 text-lg text-muted-foreground leading-relaxed">
            From zero to knowledge graph in three steps. Choose your preferred approach.
          </p>
        </div>

        <FadeIn>
          <div className="max-w-3xl mx-auto">
            <Tabs defaultValue="rust" className="w-full">
              <TabsList className="grid w-full grid-cols-3 mb-4">
                <TabsTrigger value="rust">Rust</TabsTrigger>
                <TabsTrigger value="docker">Docker</TabsTrigger>
                <TabsTrigger value="api">REST API</TabsTrigger>
              </TabsList>
              <TabsContent value="rust">
                <CodeBlock code={rustCode} lang="rust" />
              </TabsContent>
              <TabsContent value="docker">
                <CodeBlock code={dockerCode} lang="bash" />
              </TabsContent>
              <TabsContent value="api">
                <CodeBlock code={apiCode} lang="bash" />
              </TabsContent>
            </Tabs>

            <div className="mt-6 text-center">
              <Link href="/docs/" className={buttonVariants({ variant: "outline", className: "gap-2" })}>
                  Read the Full Guide <ArrowRight className="h-4 w-4" />
              </Link>
            </div>
          </div>
        </FadeIn>
      </div>
    </section>
  );
}
