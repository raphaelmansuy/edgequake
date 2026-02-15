/**
 * E2E: Document tools test.
 */
import type { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { createTestClient, isServerRunning } from "./helpers.js";

describe("document tools (e2e)", () => {
  let client: Client;
  let cleanup: () => Promise<void>;
  let serverUp: boolean;

  beforeAll(async () => {
    serverUp = await isServerRunning();
    if (!serverUp) return;
    const ctx = await createTestClient();
    client = ctx.client;
    cleanup = ctx.cleanup;
  });

  afterAll(async () => {
    if (cleanup) await cleanup();
  });

  /** Call a tool and return the raw MCP result (content + isError). */
  async function rawCallTool(
    name: string,
    args: Record<string, unknown> = {},
  ) {
    const result = await client.callTool({ name, arguments: args });
    const textContent = result.content as Array<{
      type: string;
      text: string;
    }>;
    const text = textContent?.[0]?.text ?? "";
    let parsed: unknown;
    try {
      parsed = JSON.parse(text);
    } catch {
      parsed = text;
    }
    return { parsed, isError: result.isError, raw: text };
  }

  /** Poll document status until it's no longer 'pending'. */
  async function waitForProcessing(
    docId: string,
    timeoutMs = 30000,
  ): Promise<string> {
    const start = Date.now();
    while (Date.now() - start < timeoutMs) {
      const status = await rawCallTool("document_status", {
        document_id: docId,
      });
      if (!status.isError) {
        const data = status.parsed as { status: string };
        if (data.status !== "pending") {
          return data.status;
        }
      }
      await new Promise((r) => setTimeout(r, 1000));
    }
    return "timeout";
  }

  it("should upload, get status, list, get, and delete a document", async () => {
    if (!serverUp) {
      console.log("SKIP: EdgeQuake server not running");
      return;
    }

    // Upload
    const upload = await rawCallTool("document_upload", {
      content:
        "EdgeQuake is a Graph-RAG framework written in Rust. It transforms documents into knowledge graphs using LLM-powered entity extraction.",
      title: "MCP E2E Test Document",
      enable_gleaning: false,
    });
    expect(upload.isError, `Upload failed: ${upload.raw}`).toBeFalsy();
    const uploaded = upload.parsed as {
      document_id: string;
      status: string;
    };
    expect(uploaded).toHaveProperty("document_id");
    const docId = uploaded.document_id;

    // Status
    const status = await rawCallTool("document_status", {
      document_id: docId,
    });
    expect(status.isError, `Status failed: ${status.raw}`).toBeFalsy();
    const statusData = status.parsed as { id: string; status: string };
    expect(statusData.id).toBe(docId);

    // List (may not include the new doc yet due to async processing)
    const list = await rawCallTool("document_list", {
      page: 1,
      page_size: 5,
    });
    expect(list.isError, `List failed: ${list.raw}`).toBeFalsy();
    const listData = list.parsed as {
      documents: unknown[];
      total: number;
    };
    expect(listData).toHaveProperty("documents");
    expect(Array.isArray(listData.documents)).toBe(true);

    // Get by ID
    const get = await rawCallTool("document_get", {
      document_id: docId,
    });
    expect(get.isError, `Get failed: ${get.raw}`).toBeFalsy();
    const doc = get.parsed as {
      id: string;
      title: string;
    };
    expect(doc.id).toBe(docId);
    expect(doc.title).toBe("MCP E2E Test Document");

    // Wait for the document to leave 'pending' status before deleting
    await waitForProcessing(docId);

    // Delete
    const del = await rawCallTool("document_delete", {
      document_id: docId,
    });
    expect(del.isError, `Delete failed: ${del.raw}`).toBeFalsy();
    const deleted = del.parsed as { success: boolean };
    expect(deleted.success).toBe(true);
  });
});
