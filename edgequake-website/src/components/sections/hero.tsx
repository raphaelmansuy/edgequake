"use client";

import { FadeIn } from "@/components/animations";
import { GraphAnimation } from "@/components/graph-animation";
import { Badge } from "@/components/ui/badge";
import { buttonVariants } from "@/components/ui/button";
import { ArrowRight, Github } from "lucide-react";
import Link from "next/link";

export function Hero() {
  return (
    <section id="hero" className="relative">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8 pt-24 pb-20 sm:pt-32 sm:pb-28">
        <div className="grid lg:grid-cols-2 gap-16 items-center">
          {/* Left: Copy */}
          <FadeIn>
            <div className="space-y-8">
              <h1 className="text-4xl sm:text-5xl lg:text-6xl font-bold tracking-tight leading-[1.08]">
                Graph-RAG.{" "}
                <span className="text-accent">Built for Speed.</span>
              </h1>
              <p className="text-lg text-muted-foreground max-w-xl leading-relaxed">
                Turn documents into knowledge graphs. Query with 6 modes. 10x
                faster than Python RAG. Built in Rust, powered by PostgreSQL.
              </p>

              {/* CTAs */}
              <div className="flex flex-wrap gap-3">
                <Link href="/docs/" className={buttonVariants({ size: "lg", className: "gap-2" })}>
                  Get Started <ArrowRight className="h-4 w-4" />
                </Link>
                <a
                  href="https://github.com/raphaelmansuy/edgequake"
                  target="_blank"
                  rel="noopener noreferrer"
                  className={buttonVariants({ variant: "outline", size: "lg", className: "gap-2" })}
                >
                  <Github className="h-4 w-4" /> GitHub
                </a>
              </div>

              {/* Badge Row */}
              <div className="flex flex-wrap gap-2">
                <Badge variant="secondary" className="font-mono text-xs">
                  Apache 2.0
                </Badge>
                <Badge variant="secondary" className="font-mono text-xs">
                  1000+ docs/min
                </Badge>
                <Badge variant="secondary" className="font-mono text-xs">
                  6 Query Modes
                </Badge>
                <Badge variant="secondary" className="font-mono text-xs">
                  Built with Rust
                </Badge>
              </div>
            </div>
          </FadeIn>

          {/* Right: Animated Graph */}
          <FadeIn className="hidden lg:flex items-center justify-center">
            <div className="relative w-full max-w-md aspect-square">
              <GraphAnimation />
            </div>
          </FadeIn>
        </div>
      </div>
    </section>
  );
}
