/**
 * E2E Tests: Graph operations
 *
 * Tests entity and relationship CRUD operations against a live backend.
 *
 * Run: EDGEQUAKE_E2E_URL=http://localhost:8080 npm test -- tests/e2e/
 */

import { describe, it, expect, beforeAll, afterAll } from "vitest";
import { EdgeQuake } from "../../src/index.js";
import { E2E_ENABLED, createE2EClient, testId } from "./helpers.js";

const describeE2E = E2E_ENABLED ? describe : describe.skip;

describeE2E("E2E: Graph Entities", () => {
  let client: EdgeQuake;
  const createdEntities: string[] = [];

  beforeAll(() => {
    client = createE2EClient()!;
  });

  afterAll(async () => {
    // WHY: Clean up test entities to avoid graph pollution
    for (const name of createdEntities) {
      try {
        await client.graph.entities.delete(name);
      } catch {
        // Ignore cleanup errors
      }
    }
  });

  it("should merge an entity", async () => {
    const name = testId("ENTITY").toUpperCase();
    const result = await client.graph.entities.merge({
      name,
      type: "TEST_ENTITY",
      description: "E2E test entity",
    });
    expect(result).toBeDefined();
    createdEntities.push(name);
  });

  it("should list entities", async () => {
    const entities = await client.graph.entities.list();
    expect(entities).toBeDefined();
  });

  it("should search entities by name", async () => {
    const name = testId("SEARCH_ENTITY").toUpperCase();
    await client.graph.entities.merge({
      name,
      type: "TEST_ENTITY",
      description: "Entity for search test",
    });
    createdEntities.push(name);

    const results = await client.graph.entities.search({ query: name });
    expect(results).toBeDefined();
  });

  it("should check entity existence", async () => {
    const name = testId("EXISTS_ENTITY").toUpperCase();
    await client.graph.entities.merge({
      name,
      type: "TEST_ENTITY",
      description: "Entity for existence check",
    });
    createdEntities.push(name);

    const exists = await client.graph.entities.exists(name);
    expect(exists).toBeDefined();
  });

  it("should get entity neighborhood", async () => {
    const name = testId("NEIGHBOR_ENTITY").toUpperCase();
    await client.graph.entities.merge({
      name,
      type: "TEST_ENTITY",
      description: "Entity for neighborhood test",
    });
    createdEntities.push(name);

    const neighborhood = await client.graph.entities.getNeighborhood(name);
    expect(neighborhood).toBeDefined();
  });
});

describeE2E("E2E: Graph Relationships", () => {
  let client: EdgeQuake;
  const entitiesToCleanup: string[] = [];

  beforeAll(async () => {
    client = createE2EClient()!;

    // Create two entities for relationship tests
    const src = testId("REL_SRC").toUpperCase();
    const tgt = testId("REL_TGT").toUpperCase();
    await client.graph.entities.merge({
      name: src,
      type: "TEST_ENTITY",
      description: "Source entity",
    });
    await client.graph.entities.merge({
      name: tgt,
      type: "TEST_ENTITY",
      description: "Target entity",
    });
    entitiesToCleanup.push(src, tgt);
  });

  afterAll(async () => {
    for (const name of entitiesToCleanup) {
      try {
        await client.graph.entities.delete(name);
      } catch {
        // Ignore
      }
    }
  });

  it("should list relationships", async () => {
    const rels = await client.graph.relationships.list();
    expect(rels).toBeDefined();
  });
});

describeE2E("E2E: Graph Stats", () => {
  let client: EdgeQuake;

  beforeAll(() => {
    client = createE2EClient()!;
  });

  it("should return graph statistics", async () => {
    const stats = await client.graph.getStats();
    expect(stats).toBeDefined();
  });
});
