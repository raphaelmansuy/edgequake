import { describe, expect, it } from "vitest";
import {
  createInitialStreamingState,
  reduceStreamingEvent,
  type SourceReference,
} from "../chat";
import type { SubgraphBundle } from "@/lib/utils/subgraph-types";

describe("reduceStreamingEvent", () => {
  it("builds queryContext from context event subgraph", () => {
    const sources: SourceReference[] = [
      {
        source_type: "chunk",
        id: "doc-1-chunk-0",
        score: 0.9,
        snippet: "text",
        document_id: "doc-1",
      },
    ];
    const subgraph: SubgraphBundle = {
      entities: [
        {
          id: "ent:EDGEQUAKE",
          name: "EDGEQUAKE",
          entity_type: "TECHNOLOGY",
          description: "",
          score: 0.9,
          degree: 3,
        },
      ],
      relationships: [
        {
          id: "rel:1",
          source: "EDGEQUAKE",
          target: "RAG",
          relation_type: "IMPLEMENTS",
          description: "",
          score: 0.8,
        },
      ],
    };

    const next = reduceStreamingEvent(
      {
        type: "context",
        sources,
        subgraph,
        query_mode: "hybrid",
        retrieval_time_ms: 42,
      },
      createInitialStreamingState(),
    );

    expect(next.subgraph).toEqual(subgraph);
    expect(next.queryContext?.entities[0].label).toBe("EDGEQUAKE");
    expect(next.queryContext?.entities[0].entity_type).toBe("TECHNOLOGY");
    expect(next.queryContext?.relationships[0].type).toBe("IMPLEMENTS");
    expect(next.queryMode).toBe("hybrid");
    expect(next.retrievalTimeMs).toBe(42);
  });
});
