import { describe, expect, it } from "vitest";
import { resolveWorkspaceStatCounts } from "../stats-display";

describe("resolveWorkspaceStatCounts (UI-P3-001)", () => {
  const workspace = {
    id: "w1",
    tenant_id: "t1",
    name: "Test",
    created_at: "",
    document_count: 5,
    entity_count: 10,
  };

  it("prefers stats API counts when present", () => {
    expect(
      resolveWorkspaceStatCounts(
        {
          workspace_id: "w1",
          document_count: 12,
          entity_count: 40,
          relationship_count: 7,
          entity_type_count: 3,
          chunk_count: 99,
          embedding_count: 99,
          storage_bytes: 0,
        },
        workspace,
      ),
    ).toEqual({
      documents: 12,
      entities: 40,
      relationships: 7,
      chunks: 99,
    });
  });

  it("falls back to workspace inline counts", () => {
    expect(resolveWorkspaceStatCounts(undefined, workspace)).toEqual({
      documents: 5,
      entities: 10,
      relationships: 0,
      chunks: 0,
    });
  });
});
