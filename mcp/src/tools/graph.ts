/**
 * Knowledge graph exploration tools.
 */
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { getClient } from "../client.js";
import { formatError } from "../errors.js";

export function registerGraphTools(server: McpServer): void {
  // graph_search_entities
  server.tool(
    "graph_search_entities",
    "Search for entities in the knowledge graph. Entities are people, organizations, technologies, concepts, etc. extracted from documents.",
    {
      search: z
        .string()
        .optional()
        .describe("Search term to filter entities by name"),
      label: z
        .string()
        .optional()
        .describe(
          "Filter by entity type: PERSON, ORGANIZATION, TECHNOLOGY, CONCEPT, EVENT, LOCATION, PRODUCT",
        ),
      limit: z.number().optional().describe("Max results (default: 20)"),
    },
    async (params) => {
      try {
        const client = await getClient();
        const entities = await client.graph.entities.list({
          search: params.search,
          label: params.label,
          per_page: params.limit ?? 20,
        });

        return {
          content: [
            {
              type: "text" as const,
              text: JSON.stringify(
                entities.map((e) => ({
                  name: e.name,
                  label: e.label,
                  description: e.description,
                })),
                null,
                2,
              ),
            },
          ],
        };
      } catch (error) {
        return formatError(error);
      }
    },
  );

  // graph_get_entity
  server.tool(
    "graph_get_entity",
    "Get detailed information about a specific entity including its properties and source documents",
    {
      entity_name: z
        .string()
        .describe("Entity name (e.g. RUST, OPENAI, MACHINE_LEARNING)"),
    },
    async (params) => {
      try {
        const client = await getClient();
        const entity = await client.graph.entities.get(params.entity_name);

        return {
          content: [
            {
              type: "text" as const,
              text: JSON.stringify(entity, null, 2),
            },
          ],
        };
      } catch (error) {
        return formatError(error);
      }
    },
  );

  // graph_entity_neighborhood
  server.tool(
    "graph_entity_neighborhood",
    "Get an entity's neighborhood — all directly connected entities and their relationships. Useful for exploring how concepts relate to each other.",
    {
      entity_name: z.string().describe("Entity name"),
    },
    async (params) => {
      try {
        const client = await getClient();
        const neighborhood = await client.graph.entities.neighborhood(
          params.entity_name,
        );

        return {
          content: [
            {
              type: "text" as const,
              text: JSON.stringify(
                {
                  center: {
                    name: neighborhood.center.name,
                    label: neighborhood.center.label,
                    description: neighborhood.center.description,
                  },
                  neighbors: neighborhood.neighbors.map((n) => ({
                    entity: {
                      name: n.entity.name,
                      label: n.entity.label,
                      description: n.entity.description,
                    },
                    relationship: n.relationship,
                    direction: n.direction,
                  })),
                },
                null,
                2,
              ),
            },
          ],
        };
      } catch (error) {
        return formatError(error);
      }
    },
  );

  // graph_search_relationships
  server.tool(
    "graph_search_relationships",
    "Search relationships between entities in the knowledge graph",
    {
      source: z.string().optional().describe("Source entity name"),
      target: z.string().optional().describe("Target entity name"),
      label: z.string().optional().describe("Relationship type/label"),
      limit: z.number().optional().describe("Max results (default: 20)"),
    },
    async (params) => {
      try {
        const client = await getClient();
        const relationships = await client.graph.relationships.list({
          source: params.source,
          target: params.target,
          label: params.label,
          per_page: params.limit ?? 20,
        });

        return {
          content: [
            {
              type: "text" as const,
              text: JSON.stringify(
                relationships.map((r) => ({
                  source: r.source,
                  target: r.target,
                  label: r.label,
                  description: r.description,
                  weight: r.weight,
                })),
                null,
                2,
              ),
            },
          ],
        };
      } catch (error) {
        return formatError(error);
      }
    },
  );
}
