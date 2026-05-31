import { describe, expect, it } from "vitest";
import { convertServerMessage } from "../convert-server-message";
import type { ServerMessage } from "@/types";

describe("convertServerMessage (UI-P3-005)", () => {
  it("maps basic fields from server message", () => {
    const msg: ServerMessage = {
      id: "msg-1",
      conversation_id: "conv-1",
      role: "assistant",
      content: "Hello",
      is_error: false,
      created_at: "2026-01-01T00:00:00Z",
      updated_at: "2026-01-01T00:00:00Z",
      mode: "hybrid",
      tokens_used: 42,
      llm_provider: "openai",
      llm_model: "gpt-5-nano",
    };

    const result = convertServerMessage(msg);
    expect(result.id).toBe("msg-1");
    expect(result.role).toBe("assistant");
    expect(result.content).toBe("Hello");
    expect(result.mode).toBe("hybrid");
    expect(result.tokensUsed).toBe(42);
    expect(result.llmProvider).toBe("openai");
    expect(result.isStreaming).toBe(false);
  });

  it("maps chunk sources and extracts document id from chunk id", () => {
    const msg: ServerMessage = {
      id: "msg-2",
      conversation_id: "conv-1",
      role: "assistant",
      content: "Answer",
      is_error: false,
      created_at: "2026-01-01T00:00:00Z",
      updated_at: "2026-01-01T00:00:00Z",
      context: {
        sources: [
          {
            id: "doc-uuid-chunk-3",
            content: "chunk text",
            score: 0.9,
            source_type: "chunk",
            title: "report.pdf",
          },
        ],
        entities: [{ name: "ENTITY_A", entity_type: "person", score: 1 }],
        relationships: [
          {
            source: "A",
            target: "B",
            relation_type: "related",
            score: 1,
          },
        ],
      },
    };

    const result = convertServerMessage(msg);
    expect(result.context?.chunks[0].document_id).toBe("doc-uuid");
    expect(result.context?.chunks[0].file_path).toBe("report.pdf");
    expect(result.context?.entities[0].label).toBe("ENTITY_A");
    expect(result.context?.relationships[0].type).toBe("related");
  });
});
