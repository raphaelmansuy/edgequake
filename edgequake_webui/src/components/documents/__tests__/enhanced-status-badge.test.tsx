/**
 * @vitest-environment jsdom
 */
import { EnhancedStatusBadge } from "@/components/documents/enhanced-status-badge";
import type { Document } from "@/types";
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

const readyDoc = {
  id: "doc-ready",
  status: "completed",
  current_stage: "completed",
  query_ready: true,
} as Document;

afterEach(cleanup);

describe("EnhancedStatusBadge compact fence", () => {
  it("shows a single visible Ready (not Ready Ready)", () => {
    render(<EnhancedStatusBadge document={readyDoc} compact />);
    const cell = screen.getByTestId("status-cell");
    expect(cell.textContent?.replace(/\s+/g, " ").trim()).toBe("Ready");
    expect(screen.getByTestId("spec091-serving-fence-badge")).toHaveAttribute(
      "data-query-ready",
      "true",
    );
  });
});
