/**
 * Graph Exploration — EdgeQuake TypeScript SDK
 *
 * WHY: EdgeQuake's knowledge graph stores entities and relationships
 * extracted from documents. This example shows how to traverse it.
 *
 * Usage:
 *   npx tsx examples/graph_exploration.ts
 */
import { EdgeQuake } from "@edgequake/sdk";

async function main() {
  const client = new EdgeQuake({
    baseUrl: process.env.EDGEQUAKE_URL ?? "http://localhost:8080",
    apiKey: process.env.EDGEQUAKE_API_KEY ?? "demo-key",
  });

  // ── 1. Get graph overview ─────────────────────────────────

  const graph = await client.graph.get();
  console.log("Graph overview:", graph);

  // ── 2. Search entities by keyword ─────────────────────────

  // WHY: Node search uses fuzzy matching on entity names and descriptions.
  const nodes = await client.graph.searchNodes({ query: "machine learning" });
  console.log("\nSearch results:");
  for (const node of nodes ?? []) {
    console.log(`  ${node.name} (${node.entity_type})`);
  }

  // ── 3. List entities ──────────────────────────────────────

  const entities = await client.graph.entities.list();
  console.log(`\nTotal entities: ${entities.length}`);
  for (const entity of entities.slice(0, 5)) {
    console.log(
      `  ${entity.name} — ${entity.description ?? "(no description)"}`,
    );
  }

  // ── 4. Get entity neighborhood ────────────────────────────

  // WHY: Neighborhood returns the entity plus all directly connected
  // entities (1-hop), useful for context expansion.
  if (entities.length > 0) {
    const firstEntity = entities[0];
    const neighborhood = await client.graph.entities.neighborhood(
      firstEntity.name,
    );
    console.log(`\nNeighborhood of "${firstEntity.name}":`, neighborhood);
  }

  // ── 5. List relationships ─────────────────────────────────

  const relationships = await client.graph.relationships.list();
  console.log(`\nTotal relationships: ${relationships.length}`);
  for (const rel of relationships.slice(0, 5)) {
    console.log(
      `  ${rel.source_name} --[${rel.relationship_type}]--> ${rel.target_name}`,
    );
  }

  // ── 6. Search labels ──────────────────────────────────────

  const labels = await client.graph.searchLabels({ query: "PER" });
  console.log("\nLabel search:", labels);

  // ── 7. Popular labels ─────────────────────────────────────

  const popular = await client.graph.getPopularLabels();
  console.log("Popular labels:", popular);
}

main().catch(console.error);
