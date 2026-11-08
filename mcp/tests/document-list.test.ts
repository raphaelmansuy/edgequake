/**
 * document_list must forward its `search` argument to the REST API.
 *
 * The SDK's ListDocumentsQuery has no `search` field; the server-side
 * title filter is `document_pattern`. Passing `search` straight through was
 * silently dropped, so every document_list call returned unfiltered results.
 */
import { beforeEach, describe, expect, it, vi } from "vitest";

const list = vi.fn();

vi.mock("../src/client.js", () => ({
  getClient: async () => ({ documents: { list } }),
}));

import { registerDocumentTools } from "../src/tools/document.js";

type ToolHandler = (params: Record<string, unknown>) => Promise<{
  content: Array<{ type: string; text: string }>;
  isError?: boolean;
}>;

function captureHandlers(): Map<string, ToolHandler> {
  const handlers = new Map<string, ToolHandler>();
  const fakeServer = {
    tool: (name: string, ...rest: unknown[]) => {
      handlers.set(name, rest[rest.length - 1] as ToolHandler);
    },
  };
  registerDocumentTools(fakeServer as never);
  return handlers;
}

describe("document_list", () => {
  beforeEach(() => {
    list.mockReset();
    list.mockResolvedValue({
      documents: [],
      total: 0,
      page: 1,
      page_size: 20,
      has_more: false,
    });
  });

  it("forwards search as the server's document_pattern filter", async () => {
    const handler = captureHandlers().get("document_list")!;
    const result = await handler({ search: "quarterly report", status: "completed" });

    expect(result.isError).toBeUndefined();
    expect(list).toHaveBeenCalledTimes(1);
    const query = list.mock.calls[0][0];
    expect(query.document_pattern).toBe("quarterly report");
    expect(query.status).toBe("completed");
    expect(query).not.toHaveProperty("search");
  });

  it("omits document_pattern when no search term is given", async () => {
    const handler = captureHandlers().get("document_list")!;
    await handler({ page: 2, page_size: 5 });

    const query = list.mock.calls[0][0];
    expect(query.document_pattern).toBeUndefined();
    expect(query.page).toBe(2);
    expect(query.page_size).toBe(5);
  });
});
