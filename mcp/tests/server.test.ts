/**
 * Unit tests for the MCP server.
 * Uses in-memory transport to verify tool/resource/prompt registration
 * and the MCP protocol handshake works correctly.
 *
 * When the EdgeQuake backend is running, tools return real data.
 * When it isn't, tools return isError:true with a message.
 */
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { InMemoryTransport } from "@modelcontextprotocol/sdk/inMemory.js";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { createServer } from "../src/server.js";

describe("MCP server unit tests", () => {
  let client: Client;
  let cleanup: () => Promise<void>;

  beforeAll(async () => {
    const server = createServer();
    client = new Client({ name: "test-client", version: "0.1.0" });

    const [clientTransport, serverTransport] =
      InMemoryTransport.createLinkedPair();

    await Promise.all([
      server.connect(serverTransport),
      client.connect(clientTransport),
    ]);

    cleanup = async () => {
      await client.close();
      await server.close();
    };
  });

  afterAll(async () => {
    if (cleanup) await cleanup();
  });

  it("should list all 16 registered tools", async () => {
    const tools = await client.listTools();
    const toolNames = tools.tools.map((t) => t.name).sort();

    const expected = [
      "document_delete",
      "document_get",
      "document_list",
      "document_status",
      "document_upload",
      "graph_entity_neighborhood",
      "graph_get_entity",
      "graph_search_entities",
      "graph_search_relationships",
      "health",
      "query",
      "workspace_create",
      "workspace_delete",
      "workspace_get",
      "workspace_list",
      "workspace_stats",
    ];

    expect(toolNames).toEqual(expected);
  });

  it("should have correct input schemas for key tools", async () => {
    const tools = await client.listTools();
    const toolMap = new Map(tools.tools.map((t) => [t.name, t]));

    // health has no required params
    const health = toolMap.get("health");
    expect(health).toBeDefined();

    // query requires 'query' string
    const query = toolMap.get("query");
    expect(query).toBeDefined();
    const queryProps = query!.inputSchema.properties as Record<
      string,
      unknown
    >;
    expect(queryProps).toHaveProperty("query");
    expect(queryProps).toHaveProperty("mode");
    expect(queryProps).toHaveProperty("conversation_history");

    // document_upload requires 'content' string
    const upload = toolMap.get("document_upload");
    expect(upload).toBeDefined();
    const uploadProps = upload!.inputSchema.properties as Record<
      string,
      unknown
    >;
    expect(uploadProps).toHaveProperty("content");
    expect(uploadProps).toHaveProperty("title");
    expect(uploadProps).toHaveProperty("enable_gleaning");

    // workspace_create requires 'name' string
    const create = toolMap.get("workspace_create");
    expect(create).toBeDefined();
    const createProps = create!.inputSchema.properties as Record<
      string,
      unknown
    >;
    expect(createProps).toHaveProperty("name");
    expect(createProps).toHaveProperty("llm_provider");
    expect(createProps).toHaveProperty("embedding_model");
  });

  it("should list registered prompts", async () => {
    const prompts = await client.listPrompts();
    const promptNames = prompts.prompts.map((p) => p.name).sort();
    expect(promptNames).toEqual(["document_summary", "rag_query"]);
  });

  it("should resolve rag_query prompt with topic interpolation", async () => {
    const result = await client.getPrompt({
      name: "rag_query",
      arguments: { topic: "What is EdgeQuake?" },
    });

    expect(result.messages).toHaveLength(1);
    expect(result.messages[0].role).toBe("user");
    const content = result.messages[0].content as {
      type: string;
      text: string;
    };
    expect(content.text).toContain("EdgeQuake");
    expect(content.text).toContain("hybrid");
  });

  it("should resolve document_summary prompt with document_id", async () => {
    const result = await client.getPrompt({
      name: "document_summary",
      arguments: { document_id: "test-uuid-123" },
    });

    expect(result.messages).toHaveLength(1);
    expect(result.messages[0].role).toBe("user");
    const content = result.messages[0].content as {
      type: string;
      text: string;
    };
    expect(content.text).toContain("test-uuid-123");
  });

  it("should call health tool and get a response (not crash)", async () => {
    const result = await client.callTool({ name: "health", arguments: {} });
    const content = result.content as Array<{ type: string; text: string }>;
    expect(content).toHaveLength(1);
    // Either healthy or error — both are valid, the key is it doesn't crash
    if (result.isError) {
      expect(content[0].text).toContain("Error");
    } else {
      const parsed = JSON.parse(content[0].text);
      expect(parsed).toHaveProperty("status");
    }
  });

  it("should call workspace_list and get a response (not crash)", async () => {
    const result = await client.callTool({
      name: "workspace_list",
      arguments: {},
    });
    const content = result.content as Array<{ type: string; text: string }>;
    expect(content).toHaveLength(1);
    if (result.isError) {
      expect(content[0].text).toContain("Error");
    } else {
      const parsed = JSON.parse(content[0].text);
      expect(Array.isArray(parsed)).toBe(true);
    }
  });

  it("should call document_list and get a response (not crash)", async () => {
    const result = await client.callTool({
      name: "document_list",
      arguments: {},
    });
    const content = result.content as Array<{ type: string; text: string }>;
    expect(content).toHaveLength(1);
    if (result.isError) {
      expect(content[0].text).toContain("Error");
    } else {
      const parsed = JSON.parse(content[0].text);
      expect(parsed).toHaveProperty("documents");
      expect(parsed).toHaveProperty("total");
    }
  });

  it("should call graph_search_entities and get a response (not crash)", async () => {
    const result = await client.callTool({
      name: "graph_search_entities",
      arguments: {},
    });
    const content = result.content as Array<{ type: string; text: string }>;
    expect(content).toHaveLength(1);
    if (result.isError) {
      expect(content[0].text).toContain("Error");
    } else {
      const parsed = JSON.parse(content[0].text);
      expect(Array.isArray(parsed)).toBe(true);
    }
  });
});
