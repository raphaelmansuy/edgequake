/**
 * E2E Tests: Graph operations
 *
 * Tests entity and relationship CRUD operations against a live backend.
 *
 * Run: EDGEQUAKE_E2E_URL=http://localhost:8080 npm test -- tests/e2e/
 */

import { afterAll, beforeAll, describe, expect, it } from "vitest";
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

  it("should create an entity", async () => {
    const name = testId("ENTITY").toUpperCase().replace(/-/g, "_");
    // WHY: API requires entity_name, entity_type, description, source_id
    const result = await client.graph.entities.create({
      entity_name: name,
      entity_type: "TEST_ENTITY",
      description: "E2E test entity",
      source_id: "manual_entry",
    });
    expect(result).toBeDefined();
    createdEntities.push(name);
  }, 15_000);

  it("should list entities", async () => {
    const entities = await client.graph.entities.list();
    expect(entities).toBeDefined();
    expect(Array.isArray(entities)).toBe(true);
  }, 15_000);

  it("should search entities by name", async () => {
    const name = testId("SEARCH").toUpperCase().replace(/-/g, "_");
    await client.graph.entities.create({
      entity_name: name,
      entity_type: "TEST_ENTITY",
      description: "Entity for search test",
      source_id: "manual_entry",
    });
    createdEntities.push(name);

    // WHY: No dedicated search() — use list() with search param
    const results = await client.graph.entities.list({ search: name });
    expect(results).toBeDefined();
    expect(Array.isArray(results)).toBe(true);
  }, 15_000);

  it("should check entity existence", async () => {
    const name = testId("EXISTS").toUpperCase().replace(/-/g, "_");
    await client.graph.entities.create({
      entity_name: name,
      entity_type: "TEST_ENTITY",
      description: "Entity for existence check",
      source_id: "manual_entry",
    });
    createdEntities.push(name);

    const exists = await client.graph.entities.exists(name);
    expect(exists).toBeDefined();
  }, 15_000);

  it("should get entity neighborhood", async () => {
    const name = testId("NEIGHBOR").toUpperCase().replace(/-/g, "_");
    await client.graph.entities.create({
      entity_name: name,
      entity_type: "TEST_ENTITY",
      description: "Entity for neighborhood test",
      source_id: "manual_entry",
    });
    createdEntities.push(name);

    // WHY: SDK method is neighborhood(), not getNeighborhood()
    const neighborhood = await client.graph.entities.neighborhood(name);
    expect(neighborhood).toBeDefined();
  }, 15_000);
});

describeE2E("E2E: Graph Relationships", () => {
  let client: EdgeQuake;

  beforeAll(async () => {
    client = createE2EClient()!;
  });

  it("should list relationships", async () => {
    const rels = await client.graph.relationships.list();
    expect(rels).toBeDefined();
  }, 15_000);
});

describeE2E("E2E: Graph Query", () => {
  let client: EdgeQuake;

  beforeAll(() => {
    client = createE2EClient()!;
  });

  it("should query the graph", async () => {
    // WHY: graph.get() returns graph data, not graph.getStats()
    const graph = await client.graph.get();
    expect(graph).toBeDefined();
  }, 15_000);
});
