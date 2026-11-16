import { notFound } from "next/navigation";
import { InlineHtmlMergeFixtureClient } from "./fixture-client";

/**
 * Deterministic GH-556 fixture for Playwright (no auth / API).
 *
 * Exposes the production `mergeInlineHtmlTokens` to a real browser so the
 * complexity claim in the issue can be re-measured where it actually hurts —
 * the chat UI's main thread — rather than only in jsdom.
 *
 * Server gate: non-production only, same as the markdown+LaTeX fixture.
 */
export default function InlineHtmlMergeFixturePage() {
  if (process.env.NODE_ENV === "production") {
    notFound();
  }

  return <InlineHtmlMergeFixtureClient />;
}
