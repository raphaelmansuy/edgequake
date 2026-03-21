"use client";

import { buttonVariants } from "@/components/ui/button";
import { ArrowRight, BarChart3, Cloud, Headphones, Lock, Settings, Users } from "lucide-react";
import Link from "next/link";

const title = "Enterprise — EdgeQuake";
const description = "Enterprise-grade Graph-RAG with dedicated support, custom integrations, and on-premise deployment. Built by Elitizon.";

const features = [
  {
    icon: Lock,
    title: "Enterprise Security",
    description: "SOC 2 ready architecture. GDPR-compliant data handling. On-premise deployment options with full data sovereignty.",
  },
  {
    icon: Users,
    title: "Multi-Tenant Isolation",
    description: "Workspace-level isolation with per-tenant storage, LLM configuration, and access control. No data leakage between tenants.",
  },
  {
    icon: Headphones,
    title: "Priority Support",
    description: "Dedicated engineering support with SLA guarantees. Direct access to the core team for architecture consulting.",
  },
  {
    icon: Settings,
    title: "Custom Integrations",
    description: "Tailored document pipelines, custom entity types, specialized query modes, and integration with your existing infrastructure.",
  },
  {
    icon: Cloud,
    title: "Deployment Flexibility",
    description: "Run on your cloud, on-premise, or air-gapped. Docker, Kubernetes, and bare-metal deployment guides included.",
  },
  {
    icon: BarChart3,
    title: "Monitoring & Observability",
    description: "OpenTelemetry integration, Prometheus metrics, and structured logging. Full visibility into pipeline performance.",
  },
];

const tiers = [
  {
    name: "Community",
    price: "Free",
    description: "For individual developers and small teams evaluating Graph-RAG.",
    features: [
      "Apache 2.0 license",
      "All core features",
      "Community Discord support",
      "GitHub Issues",
      "Self-hosted only",
    ],
    cta: "Get Started",
    ctaHref: "/docs/",
    variant: "outline" as const,
  },
  {
    name: "Enterprise",
    price: "Custom",
    description: "For organizations deploying Graph-RAG at scale in production.",
    features: [
      "Everything in Community",
      "Priority support (SLA-backed)",
      "Custom integrations",
      "Architecture consulting",
      "Security review & compliance",
      "On-premise deployment support",
      "Dedicated engineering contact",
    ],
    cta: "Contact Us",
    ctaHref: "/contact/",
    variant: "default" as const,
    highlighted: true,
  },
];

export default function EnterprisePage() {
  return (
    <div className="min-h-screen">
      {/* Hero */}
      <section className="py-24 sm:py-32">
        <div className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8">
          <div className="max-w-3xl mx-auto text-center mb-20">
            <p className="text-sm font-medium text-accent uppercase tracking-widest mb-3">
              Enterprise
            </p>
            <h1 className="text-4xl sm:text-5xl font-bold tracking-tight">
              Graph-RAG for{" "}
              <span className="text-muted-foreground">Your Organization</span>
            </h1>
            <p className="mt-4 text-lg text-muted-foreground leading-relaxed max-w-2xl mx-auto">
              Deploy EdgeQuake at scale with dedicated support, custom integrations,
              and architecture consulting from the team at Elitizon.
            </p>
          </div>

          {/* Features Grid */}
          <div className="grid sm:grid-cols-2 lg:grid-cols-3 gap-6 mb-24">
            {features.map((feature) => (
              <div key={feature.title} className="rounded-xl border border-border p-6">
                <div className="flex items-center gap-3 mb-3">
                  <feature.icon className="h-5 w-5 text-accent shrink-0" />
                  <h3 className="font-semibold">{feature.title}</h3>
                </div>
                <p className="text-sm text-muted-foreground leading-relaxed">
                  {feature.description}
                </p>
              </div>
            ))}
          </div>

          {/* Pricing Tiers */}
          <div className="max-w-3xl mx-auto">
            <h2 className="text-2xl font-bold tracking-tight text-center mb-10">
              Choose Your Plan
            </h2>
            <div className="grid sm:grid-cols-2 gap-6">
              {tiers.map((tier) => (
                <div
                  key={tier.name}
                  className={`rounded-xl border p-6 ${
                    tier.highlighted
                      ? "border-accent bg-card"
                      : "border-border"
                  }`}
                >
                  <h3 className="text-lg font-semibold">{tier.name}</h3>
                  <p className="text-3xl font-bold tracking-tight mt-2">{tier.price}</p>
                  <p className="text-sm text-muted-foreground mt-2 mb-6">{tier.description}</p>
                  <ul className="space-y-2 mb-8">
                    {tier.features.map((f) => (
                      <li key={f} className="text-sm flex items-start gap-2">
                        <span className="text-accent mt-0.5">&#10003;</span>
                        {f}
                      </li>
                    ))}
                  </ul>
                  <Link
                    href={tier.ctaHref}
                    className={buttonVariants({ variant: tier.variant, className: "w-full gap-2" })}
                  >
                    {tier.cta} <ArrowRight className="h-4 w-4" />
                  </Link>
                </div>
              ))}
            </div>
          </div>
        </div>
      </section>
    </div>
  );
}
