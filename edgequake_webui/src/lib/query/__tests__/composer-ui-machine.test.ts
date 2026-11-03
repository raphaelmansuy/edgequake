import { describe, expect, it } from "vitest";
import {
  composerPhaseFromSession,
  composerShowsSend,
  composerShowsStop,
  composerSubmitIntent,
  createComposerUiState,
  reduceComposerUi,
} from "../composer-ui-machine";

describe("composer-ui-machine", () => {
  it("maps session facts to phase", () => {
    expect(
      composerPhaseFromSession({
        isStreaming: true,
        queuedMessage: null,
        isError: false,
      }),
    ).toBe("streaming");
    expect(
      composerPhaseFromSession({
        isStreaming: false,
        queuedMessage: "hi",
        isError: false,
      }),
    ).toBe("queued");
    expect(
      composerPhaseFromSession({
        isStreaming: false,
        queuedMessage: null,
        isError: true,
      }),
    ).toBe("error");
  });

  it("stop only while streaming", () => {
    expect(composerShowsStop("streaming")).toBe(true);
    expect(composerShowsSend("streaming")).toBe(false);
    expect(composerShowsSend("idle")).toBe(true);
  });

  it("queue survives stream_end", () => {
    let s = createComposerUiState();
    s = reduceComposerUi(s, { type: "stream_start" });
    s = reduceComposerUi(s, { type: "queue", text: "next" });
    s = reduceComposerUi(s, { type: "stream_end" });
    expect(s.phase).toBe("queued");
    expect(s.queuedText).toBe("next");
  });

  it("a picture alone is enough to send", () => {
    // The regression: `if (!text) return;` made Send a dead button for an image-only
    // search, even though the button was enabled and the API accepts it.
    expect(composerSubmitIntent({ text: "", hasSearchImage: true, phase: "idle" })).toBe("send");
    expect(composerSubmitIntent({ text: "   ", hasSearchImage: true, phase: "idle" })).toBe("send");
  });

  it("nothing to send means nothing happens", () => {
    expect(composerSubmitIntent({ text: "", hasSearchImage: false, phase: "idle" })).toBe("none");
    expect(composerSubmitIntent({ text: "  \n ", hasSearchImage: false, phase: "idle" })).toBe("none");
  });

  it("text sends on its own, with or without a picture", () => {
    expect(composerSubmitIntent({ text: "pizza", hasSearchImage: false, phase: "idle" })).toBe("send");
    expect(composerSubmitIntent({ text: "pizza", hasSearchImage: true, phase: "idle" })).toBe("send");
  });

  it("while streaming a turn is queued and only if it has text", () => {
    // WHY: the request in flight already consumed the picture; a queued image-only turn
    // would be an empty message.
    expect(composerSubmitIntent({ text: "next", hasSearchImage: true, phase: "streaming" })).toBe(
      "queue",
    );
    expect(composerSubmitIntent({ text: "", hasSearchImage: true, phase: "streaming" })).toBe("none");
  });
});
