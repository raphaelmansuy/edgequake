import { describe, expect, it } from "vitest";
import {
  applyPreset,
  collectZones,
  defaultWorkspaceDocument,
  deserializeWorkspace,
  dockZone,
  hasAllZones,
  migrateLegacyLayoutMode,
  minSizeForSubtree,
  minSizeForZone,
  presetClassic,
  presetLibraryCenter,
  presetLibraryLeft,
  removeZone,
  sanitizeWorkspaceDocument,
  swapZones,
  WORKSPACE_PRESET_ORDER,
  WORKSPACE_PRESETS,
  WORKSPACE_ZONES,
  ZONE_MIN_PX,
  ZONE_RAIL_PX,
  type LayoutNode,
} from "../workspace-layout";

function assertAllZonesOnce(tree: LayoutNode) {
  const zones = collectZones(tree);
  expect(zones.sort()).toEqual([...WORKSPACE_ZONES].sort());
  expect(new Set(zones).size).toBe(3);
}

describe("workspace presets", () => {
  for (const id of WORKSPACE_PRESET_ORDER) {
    it(`${id} contains each zone exactly once`, () => {
      assertAllZonesOnce(WORKSPACE_PRESETS[id].build());
    });
  }
});

describe("dockZone", () => {
  it("docks library to the left of intake → tall library column", () => {
    const tree = presetClassic();
    const next = dockZone(tree, "library", "intake", "left");
    assertAllZonesOnce(next);
    expect(next.type).toBe("split");
    if (next.type !== "split") return;
    // After remove+replace, library should be sibling of intake somewhere.
    expect(collectZones(next)).toContain("library");
    // Left of intake means a horizontal split with library then intake.
    const flat = JSON.stringify(next);
    expect(flat).toContain('"zone":"library"');
    expect(flat).toContain('"zone":"intake"');
  });

  it("swap via center edge", () => {
    const tree = presetClassic();
    const next = dockZone(tree, "intake", "runs", "center");
    assertAllZonesOnce(next);
    // Intake and runs swapped inside the tools row.
    expect(JSON.stringify(next)).toContain('"zone":"runs"');
  });

  it("no-op when docking a zone onto itself", () => {
    const tree = presetLibraryLeft();
    const next = dockZone(tree, "library", "library", "left");
    expect(JSON.stringify(next)).toBe(JSON.stringify(tree));
  });

  it("docks runs below library", () => {
    const tree = presetLibraryCenter();
    const next = dockZone(tree, "runs", "library", "bottom");
    assertAllZonesOnce(next);
    expect(hasAllZones(next)).toBe(true);
  });
});

describe("swapZones / removeZone", () => {
  it("swaps two leaves", () => {
    const tree = presetClassic();
    const next = swapZones(tree, "intake", "library");
    assertAllZonesOnce(next);
    const zones = collectZones(next);
    expect(zones).toContain("intake");
    expect(zones).toContain("library");
  });

  it("removeZone promotes sibling", () => {
    const tree = presetClassic();
    const without = removeZone(tree, "runs");
    expect(without).not.toBeNull();
    expect(collectZones(without!).sort()).toEqual(["intake", "library"].sort());
  });
});

describe("sanitize / serialize", () => {
  it("rejects corrupt JSON / missing zones", () => {
    expect(sanitizeWorkspaceDocument(null).presetId).toBe("classic");
    expect(
      sanitizeWorkspaceDocument({ version: 2, tree: { type: "leaf", zone: "intake" } })
        .presetId,
    ).toBe("classic");
    expect(
      sanitizeWorkspaceDocument({
        version: 1,
        tree: presetClassic(),
      }).presetId,
    ).toBe("classic");
  });

  it("round-trips a valid document", () => {
    const doc = applyPreset("library-left");
    const again = deserializeWorkspace(JSON.stringify(doc));
    expect(again?.presetId).toBe("library-left");
    assertAllZonesOnce(again!.tree);
  });

  it("default document is classic", () => {
    expect(defaultWorkspaceDocument().presetId).toBe("classic");
    assertAllZonesOnce(defaultWorkspaceDocument().tree);
  });

  it("default document and presets rail idle Runs", () => {
    expect(defaultWorkspaceDocument().collapsed.runs).toBe(true);
    expect(applyPreset("classic").collapsed.runs).toBe(true);
    expect(applyPreset("library-left").collapsed.runs).toBe(true);
    expect(applyPreset("classic", { runsIdle: false }).collapsed.runs).toBeFalsy();
  });
});

describe("legacy migration", () => {
  it("maps focus → library-center", () => {
    expect(migrateLegacyLayoutMode("focus").presetId).toBe("library-center");
  });
  it("maps split → classic", () => {
    expect(migrateLegacyLayoutMode("split").presetId).toBe("classic");
  });
  it("maps stack → stacked tools", () => {
    const doc = migrateLegacyLayoutMode("stack");
    assertAllZonesOnce(doc.tree);
    expect(doc.tree.type).toBe("split");
  });
  it("unknown → classic", () => {
    expect(migrateLegacyLayoutMode("nope").presetId).toBe("classic");
  });
});

describe("zone minimum sizes", () => {
  it("exposes per-zone mins", () => {
    expect(minSizeForZone("intake", "horizontal")).toBe(ZONE_MIN_PX.intake.width);
    expect(minSizeForZone("library", "vertical")).toBe(ZONE_MIN_PX.library.height);
  });

  it("sums side-by-side children and maxes stacked children", () => {
    const classic = presetClassic();
    // Root is vertical: tools | library → max(toolsHeight, libraryHeight)
    const rootMin = minSizeForSubtree(classic, "vertical");
    expect(rootMin).toBeGreaterThanOrEqual(ZONE_MIN_PX.library.height);

    // Tools row horizontal min ≥ intake + runs + splitter
    if (classic.type !== "split") throw new Error("expected split");
    const tools = classic.children[0];
    const toolsMin = minSizeForSubtree(tools, "horizontal");
    expect(toolsMin).toBeGreaterThanOrEqual(
      ZONE_MIN_PX.intake.width + ZONE_MIN_PX.runs.width,
    );
  });

  it("collapsed zone uses rail thickness", () => {
    expect(
      minSizeForSubtree(
        { type: "leaf", zone: "runs" },
        "horizontal",
        { runs: true },
      ),
    ).toBe(ZONE_RAIL_PX);
  });
});
