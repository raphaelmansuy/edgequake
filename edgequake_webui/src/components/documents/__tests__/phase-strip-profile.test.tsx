/**
 * SPEC-180 — the ingest strip must describe the work each profile really does.
 *
 * The bug this pins: a Normal (classical RAG) workspace was painted with the GraphRAG
 * four-phase strip, "Extract" included, so a chunk-and-embed ingest appeared to be building a
 * knowledge graph it will never have.
 */

import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { PhaseStrip } from "@/components/documents/phase-strip";
import type { IngestionRunView } from "@/lib/pipeline/ingestion-run-view";

function run(overrides: Partial<IngestionRunView> = {}): IngestionRunView {
  return {
    documentId: "doc-1",
    trackId: "insert-1",
    filename: "handbook.pdf",
    sourceType: "pdf",
    stage: "chunking",
    stageStatus: "active",
    message: "Chunking",
    ...overrides,
  };
}

const phaseIds = (container: HTMLElement) =>
  // Scope to the phase strip: WireMarkers also emits `spec048-stage-*` markers.
  [...container.querySelectorAll("[data-testid='spec091-phase-strip'] [data-testid^='spec091-phase-']")]
    .filter((el) => el.getAttribute("data-testid") !== "spec091-phase-strip")
    .map((el) => el.getAttribute("data-testid"));

describe("PhaseStrip by profile", () => {
  it("classical RAG shows Admit, Prepare, Index — never Extract", () => {
    const { container } = render(<PhaseStrip run={run()} profile="normal" />);
    expect(phaseIds(container)).toEqual([
      "spec091-phase-admit",
      "spec091-phase-prepare",
      "spec091-phase-index",
    ]);
    expect(screen.queryByTestId("spec091-phase-extract")).toBeNull();
    expect(screen.queryByText(/extract/i)).toBeNull();
  });

  it("GraphRAG keeps the four phases, extraction included", () => {
    const { container } = render(<PhaseStrip run={run()} profile="graph" />);
    expect(phaseIds(container)).toEqual([
      "spec091-phase-admit",
      "spec091-phase-prepare",
      "spec091-phase-extract",
      "spec091-phase-index",
    ]);
  });

  it("a chunking run is Prepare and the earlier phase is done", () => {
    render(<PhaseStrip run={run({ stage: "chunking" })} profile="normal" />);
    expect(screen.getByTestId("spec091-phase-admit").dataset.state).toBe("done");
    expect(screen.getByTestId("spec091-phase-prepare").dataset.state).toBe(
      "active",
    );
    expect(screen.getByTestId("spec091-phase-index").dataset.state).toBe(
      "pending",
    );
  });

  it("a normal ingest at embedding never paints an extraction phase", () => {
    const { container } = render(
      <PhaseStrip run={run({ stage: "embedding" })} profile="normal" />,
    );
    expect(phaseIds(container)).toHaveLength(3);
    expect(screen.getByTestId("spec091-phase-index").dataset.state).toBe(
      "active",
    );
    expect(screen.getByTestId("spec091-phase-prepare").dataset.state).toBe(
      "done",
    );
  });

  it("warns when a normal workspace reports a graph-only stage", () => {
    const { container } = render(
      <PhaseStrip run={run({ stage: "extracting" })} profile="normal" />,
    );
    const strip = container.querySelector("[data-testid='spec091-phase-strip']");
    expect(strip?.getAttribute("data-outside-profile")).toBe("true");
    // The last real phase still shows activity rather than resetting to Admit.
    expect(screen.getByTestId("spec091-phase-index").dataset.state).toBe(
      "active",
    );
  });

  it("each phase explains what it produces, for screen readers", () => {
    render(<PhaseStrip run={run()} profile="normal" />);
    expect(
      screen.getByTestId("spec091-phase-index").getAttribute("aria-label"),
    ).toMatch(/embed|store/i);
  });

  it("a failed run marks the active phase failed, not a phase it does not have", () => {
    render(
      <PhaseStrip
        run={run({ stage: "embedding", stageStatus: "failed" })}
        profile="normal"
      />,
    );
    expect(screen.getByTestId("spec091-phase-index").dataset.state).toBe(
      "failed",
    );
  });

  it("defaults to GraphRAG for call sites that do not pass a profile", () => {
    const { container } = render(<PhaseStrip run={run()} />);
    expect(phaseIds(container)).toHaveLength(4);
  });
});
