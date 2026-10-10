import { describe, expect, it } from "vitest";
import {
  createReasoningPanelState,
  isReasoningPanelExpanded,
  isReasoningPanelLive,
  reduceReasoningPanel,
  reasoningEventFromStream,
} from "../reasoning-panel-machine";

describe("reasoning-panel-machine", () => {
  it("stays open through the answer and collapses when the stream ends", () => {
    let s = createReasoningPanelState();
    s = reduceReasoningPanel(s, { type: "stream_live" });
    expect(s.phase).toBe("live");
    expect(isReasoningPanelExpanded(s.phase)).toBe(true);
    expect(isReasoningPanelLive(s.phase)).toBe(true);

    s = reduceReasoningPanel(s, { type: "stream_answer" });
    expect(s.phase).toBe("live");

    s = reduceReasoningPanel(s, { type: "stream_idle" });
    expect(s.phase).toBe("collapsed");
    expect(isReasoningPanelLive(s.phase)).toBe(false);
  });

  it("user expand locks against auto-collapse", () => {
    let s = createReasoningPanelState();
    s = reduceReasoningPanel(s, { type: "user_toggle" });
    expect(s.phase).toBe("expanded");
    s = reduceReasoningPanel(s, { type: "stream_answer" });
    expect(s.phase).toBe("expanded");
  });

  it("user collapse locks against auto-expand", () => {
    let s = createReasoningPanelState();
    s = reduceReasoningPanel(s, { type: "stream_live" });
    s = reduceReasoningPanel(s, { type: "user_toggle" }); // → expanded
    s = reduceReasoningPanel(s, { type: "user_toggle" }); // → collapsed_locked
    expect(s.phase).toBe("collapsed_locked");
    s = reduceReasoningPanel(s, { type: "stream_live" });
    expect(s.phase).toBe("collapsed_locked");
  });

  it("derives events from stream facts", () => {
    expect(
      reasoningEventFromStream({
        isStreaming: true,
        cotOpen: true,
        hasResponseText: false,
      }),
    ).toEqual({ type: "stream_live" });
    expect(
      reasoningEventFromStream({
        isStreaming: true,
        cotOpen: false,
        hasResponseText: true,
      }),
    ).toEqual({ type: "stream_answer" });
    expect(
      reasoningEventFromStream({
        isStreaming: false,
        cotOpen: false,
        hasResponseText: true,
      }),
    ).toEqual({ type: "stream_idle" });
    expect(
      reasoningEventFromStream({
        isStreaming: false,
        cotOpen: false,
        hasResponseText: false,
      }),
    ).toEqual({ type: "stream_idle" });
  });
});
