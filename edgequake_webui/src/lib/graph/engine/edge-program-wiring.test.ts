/**
 * Source-level guard for edge program configuration.
 *
 * WHY a source assertion instead of a behavioural test: instantiating a Sigma
 * edge program requires a live WebGL context, which the unit runner does not
 * provide. The regression this guards is cheap to reintroduce (swap the custom
 * program back for the stock export) and expensive to notice (labels silently
 * change style), so the invariant is asserted on the source directly.
 */
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

const engineSource = readFileSync(
  join(__dirname, "create-engine.ts"),
  "utf8",
);

describe("edge program label wiring", () => {
  it("does not use the stock curved program (it would override our labels)", () => {
    // sigma prefers `edgeProgram.drawLabel` over `settings.defaultDrawEdgeLabel`,
    // so the stock export silently replaces our themed renderer.
    // Strip comments first: the rationale above is allowed to name it.
    const code = engineSource
      .replace(/\/\*[\s\S]*?\*\//g, "")
      .replace(/\/\/[^\n]*/g, "");
    expect(code).not.toContain("EdgeCurvedArrowProgram");
  });

  it("builds the curved program with our label renderer", () => {
    expect(engineSource).toMatch(
      /curvedArrow:\s*createEdgeCurveProgram\(\{[\s\S]*?drawLabel:\s*drawEdgeLabelWithBackground/,
    );
  });

  it("keeps the straight arrow program for lone edges", () => {
    expect(engineSource).toContain("arrow: EdgeArrowProgram");
  });
});

describe("edge curvature SSOT", () => {
  it("delegates parallel curvature to the shared module", () => {
    const applyDelta = readFileSync(join(__dirname, "apply-delta.ts"), "utf8");
    expect(applyDelta).toContain("assignPairCurvatures");
    expect(applyDelta).toContain("undirectedPairKey");
  });

  it("no longer hardcodes an unbounded curvature ramp", () => {
    const applyDelta = readFileSync(join(__dirname, "apply-delta.ts"), "utf8");
    // The old rule was `BASE + idx * STEP`, which grew without bound.
    expect(applyDelta).not.toMatch(/CURVATURE_BASE/);
    expect(applyDelta).not.toMatch(/CURVATURE_STEP/);
  });
});

describe("focus overlay curve parity", () => {
  it("reuses the shared curve geometry (matches the WebGL shader)", () => {
    const overlay = readFileSync(join(__dirname, "focus-edge-overlay.ts"), "utf8");
    expect(overlay).toContain("edgeCurveGeometry");
    // The buggy inline math scaled the control offset by an extra 0.5.
    expect(overlay).not.toMatch(/curvature\s*\*\s*chord\s*\*\s*0\.5/);
  });
});
