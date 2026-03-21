"use client";

import { FadeIn } from "@/components/animations";
import { buttonVariants } from "@/components/ui/button";
import { ArrowRight, Headphones, Settings, Shield } from "lucide-react";
import Link from "next/link";

export function EnterpriseCTA() {
  return (
    <section id="enterprise-cta" className="py-24 sm:py-32 border-t border-border">
      <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
        <FadeIn>
          <div className="rounded-2xl border border-border bg-card p-8 sm:p-12 lg:p-16">
            <div>
              <h2 className="text-2xl sm:text-3xl lg:text-4xl font-bold tracking-tight mb-4">
                Need EdgeQuake for{" "}
                <span className="text-muted-foreground">Your Organization?</span>
              </h2>
              <p className="text-lg text-muted-foreground max-w-2xl mb-10 leading-relaxed">
                Get dedicated support, custom integrations, and architecture consulting
                from the EdgeQuake team at Elitizon.
              </p>

              <div className="grid sm:grid-cols-3 gap-6 mb-10">
                <div className="flex items-start gap-3">
                  <Shield className="h-5 w-5 text-accent mt-0.5 shrink-0" />
                  <div>
                    <h3 className="text-sm font-semibold">Enterprise Security</h3>
                    <p className="text-sm text-muted-foreground">SOC 2, GDPR, on-premise deployment</p>
                  </div>
                </div>
                <div className="flex items-start gap-3">
                  <Headphones className="h-5 w-5 text-accent mt-0.5 shrink-0" />
                  <div>
                    <h3 className="text-sm font-semibold">Priority Support</h3>
                    <p className="text-sm text-muted-foreground">Dedicated engineering, SLA-backed</p>
                  </div>
                </div>
                <div className="flex items-start gap-3">
                  <Settings className="h-5 w-5 text-accent mt-0.5 shrink-0" />
                  <div>
                    <h3 className="text-sm font-semibold">Custom Integrations</h3>
                    <p className="text-sm text-muted-foreground">Tailored pipelines and connectors</p>
                  </div>
                </div>
              </div>

              <Link href="/contact/" className={buttonVariants({ size: "lg", className: "gap-2" })}>
                  Contact Us <ArrowRight className="h-4 w-4" />
              </Link>
            </div>
          </div>
        </FadeIn>
      </div>
    </section>
  );
}
