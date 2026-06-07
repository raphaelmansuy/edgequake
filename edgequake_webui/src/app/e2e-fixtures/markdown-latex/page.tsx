import { notFound } from "next/navigation";
import { MarkdownLatexFixtureClient } from "./fixture-client";

/**
 * Deterministic markdown+LaTeX fixture for Playwright (no auth / API).
 * Server gate: non-production only.
 */
export default function MarkdownLatexFixturePage() {
  if (process.env.NODE_ENV === "production") {
    notFound();
  }

  return <MarkdownLatexFixtureClient />;
}
