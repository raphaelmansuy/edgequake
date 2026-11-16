import { notFound } from "next/navigation";
import { VirtualizedHighlightFixtureClient } from "./fixture-client";

/**
 * Deterministic GH-556 follow-up fixture for Playwright (no auth / API).
 *
 * Server gate: non-production only, same as the other `e2e-fixtures` routes.
 */
export default function VirtualizedHighlightFixturePage() {
  if (process.env.NODE_ENV === "production") {
    notFound();
  }

  return <VirtualizedHighlightFixtureClient />;
}
