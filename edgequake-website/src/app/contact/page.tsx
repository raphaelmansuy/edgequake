"use client";

import { buttonVariants } from "@/components/ui/button";
import { CheckCircle, Send } from "lucide-react";
import { useState } from "react";

const useCases = [
  "Document Intelligence",
  "Knowledge Management",
  "Research & Analysis",
  "Customer Support",
  "Compliance & Audit",
  "Other",
];

export default function ContactPage() {
  const [submitted, setSubmitted] = useState(false);

  if (submitted) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <div className="max-w-md text-center px-4">
          <CheckCircle className="h-12 w-12 text-accent mx-auto mb-4" />
          <h1 className="text-2xl font-bold mb-2">Message Received</h1>
          <p className="text-muted-foreground">
            Thank you for your interest in EdgeQuake. Our team will get back to you
            within 1-2 business days.
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen">
      <section className="py-24 sm:py-32">
        <div className="mx-auto max-w-2xl px-4 sm:px-6 lg:px-8">
          <div className="text-center mb-12">
            <p className="text-sm font-medium text-accent uppercase tracking-widest mb-3">
              Contact
            </p>
            <h1 className="text-4xl sm:text-5xl font-bold tracking-tight">
              Get in Touch
            </h1>
            <p className="mt-4 text-lg text-muted-foreground leading-relaxed">
              Interested in EdgeQuake for your organization? Tell us about your use case.
            </p>
          </div>

          <form
            onSubmit={(e) => {
              e.preventDefault();
              setSubmitted(true);
            }}
            className="space-y-6"
          >
            <div className="grid sm:grid-cols-2 gap-4">
              <div>
                <label htmlFor="name" className="block text-sm font-medium mb-1.5">
                  Name
                </label>
                <input
                  id="name"
                  name="name"
                  type="text"
                  required
                  className="w-full rounded-lg border border-border bg-card px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-accent"
                  placeholder="Your name"
                />
              </div>
              <div>
                <label htmlFor="email" className="block text-sm font-medium mb-1.5">
                  Email
                </label>
                <input
                  id="email"
                  name="email"
                  type="email"
                  required
                  className="w-full rounded-lg border border-border bg-card px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-accent"
                  placeholder="you@company.com"
                />
              </div>
            </div>

            <div>
              <label htmlFor="company" className="block text-sm font-medium mb-1.5">
                Company
              </label>
              <input
                id="company"
                name="company"
                type="text"
                className="w-full rounded-lg border border-border bg-card px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-accent"
                placeholder="Company name"
              />
            </div>

            <div>
              <label className="block text-sm font-medium mb-2">
                Use Case
              </label>
              <div className="flex flex-wrap gap-2">
                {useCases.map((uc) => (
                  <label
                    key={uc}
                    className="text-sm border border-border rounded-lg px-3 py-1.5 cursor-pointer hover:border-accent/40 transition-colors has-[:checked]:bg-accent has-[:checked]:text-accent-foreground has-[:checked]:border-accent"
                  >
                    <input type="radio" name="useCase" value={uc} className="sr-only" />
                    {uc}
                  </label>
                ))}
              </div>
            </div>

            <div>
              <label htmlFor="message" className="block text-sm font-medium mb-1.5">
                Message
              </label>
              <textarea
                id="message"
                name="message"
                rows={4}
                required
                className="w-full rounded-lg border border-border bg-card px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-accent resize-none"
                placeholder="Tell us about your project and what you're looking to achieve..."
              />
            </div>

            <button
              type="submit"
              className={buttonVariants({ size: "lg", className: "w-full gap-2" })}
            >
              Send Message <Send className="h-4 w-4" />
            </button>

            <p className="text-xs text-muted-foreground text-center">
              By submitting, you agree to our privacy policy. We&apos;ll respond within 1-2 business days.
            </p>
          </form>
        </div>
      </section>
    </div>
  );
}
