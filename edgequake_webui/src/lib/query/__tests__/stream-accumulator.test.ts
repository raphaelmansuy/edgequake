import { describe, expect, it } from "vitest";
import {
  applyStreamContext,
  applyStreamConversationId,
  applyStreamToken,
  createStreamAccumulator,
  shouldTransitionToGenerating,
} from "../stream-accumulator";

describe("stream-accumulator (UI-P3-006)", () => {
  it("creates accumulator with initial conversation id", () => {
    expect(createStreamAccumulator("conv-1")).toEqual({
      fullContent: "",
      context: undefined,
      thinkingTimeMs: undefined,
      newConversationId: "conv-1",
      streamingPhase: "thinking",
    });
  });

  it("detects thinking to generating transition", () => {
    expect(shouldTransitionToGenerating(true, undefined)).toBe(true);
    expect(shouldTransitionToGenerating(true, 100)).toBe(false);
    expect(shouldTransitionToGenerating(false, undefined)).toBe(false);
  });

  it("accumulates tokens and transitions phase once response text appears", () => {
    let acc = createStreamAccumulator(null);
    const start = 1_000;

    ({ accumulator: acc } = applyStreamToken(acc, "reasoning", false, start + 100, start));
    expect(acc.streamingPhase).toBe("thinking");
    expect(acc.fullContent).toContain("reasoning");

    const result = applyStreamToken(
      acc,
      "Hello world",
      true,
      start + 500,
      start,
    );
    expect(result.accumulator.streamingPhase).toBe("generating");
    expect(result.update.thinkingTimeMs).toBe(500);
    expect(result.update.content).toContain("Hello world");
  });

  it("applies context and conversation id updates", () => {
    let acc = createStreamAccumulator("old");
    acc = applyStreamContext(acc, { chunks: [], entities: [], relationships: [] });
    acc = applyStreamConversationId(acc, "new");
    expect(acc.newConversationId).toBe("new");
    expect(acc.context).toEqual({ chunks: [], entities: [], relationships: [] });
  });
});
