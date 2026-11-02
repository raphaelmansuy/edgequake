import { describe, expect, it } from "vitest";
import {
  blockedModelSentence,
  emptyModelDraft,
  hasAnyPick,
  inheritedRoles,
  isCompletePick,
  pickedCount,
  refusedPickedRoles,
  refusedRoles,
  roleSummary,
  toModelPayload,
  verdictSentence,
  type WorkspaceModelDraft,
} from "../workspace-model-draft";

const ladder = inheritedRoles({
  defaultLlmProvider: "openai",
  defaultLlmModel: "gpt-5-nano",
  defaultEmbeddingProvider: "openai",
  defaultEmbeddingModel: "text-embedding-3-small",
  defaultEmbeddingDimension: 1536,
  defaultVisionProvider: "openai",
  defaultVisionModel: "gpt-5-nano",
});

const choose = (patch: Partial<WorkspaceModelDraft> = {}): WorkspaceModelDraft => ({
  source: "choose",
  ...patch,
});

describe("toModelPayload", () => {
  it("sends nothing while the workspace inherits", () => {
    const draft = emptyModelDraft();
    // A pick can survive behind the inherit radio; it must still not be sent.
    expect(toModelPayload(draft)).toEqual({});
    expect(
      toModelPayload({ ...draft, llm: { provider: "openai", model: "gpt-5-mini" } }),
    ).toEqual({});
  });

  it("carries only the roles the operator pinned", () => {
    const body = toModelPayload(
      choose({ llm: { provider: "openai", model: "gpt-5-mini" } }),
    );
    expect(body).toEqual({ llm_provider: "openai", llm_model: "gpt-5-mini" });
    expect(body.embedding_model).toBeUndefined();
    expect(body.vision_llm_model).toBeUndefined();
  });

  it("never guesses an embedding dimension", () => {
    const body = toModelPayload(
      choose({
        embedding: { provider: "ollama", model: "embeddinggemma:latest", dimension: 768 },
      }),
    );
    expect(body).toEqual({
      embedding_provider: "ollama",
      embedding_model: "embeddinggemma:latest",
    });
    expect("embedding_dimension" in body).toBe(false);
  });

  it("ignores a half-made pick", () => {
    expect(
      toModelPayload(choose({ llm: { provider: "openai", model: "  " } })),
    ).toEqual({});
  });

  it("carries all three roles when each is complete", () => {
    expect(
      toModelPayload(
        choose({
          llm: { provider: "openai", model: "gpt-5-nano" },
          embedding: { provider: "ollama", model: "embeddinggemma:latest" },
          vision: { provider: "openai", model: "gpt-5.2" },
        }),
      ),
    ).toEqual({
      llm_provider: "openai",
      llm_model: "gpt-5-nano",
      embedding_provider: "ollama",
      embedding_model: "embeddinggemma:latest",
      vision_llm_provider: "openai",
      vision_llm_model: "gpt-5.2",
    });
  });
});

describe("pickedCount / hasAnyPick", () => {
  it("counts complete pairs only", () => {
    expect(pickedCount(emptyModelDraft())).toBe(0);
    expect(hasAnyPick(emptyModelDraft())).toBe(false);
    expect(pickedCount(choose({ llm: { provider: "openai" } }))).toBe(0);
    expect(hasAnyPick(choose({ llm: { provider: "openai", model: "x" } }))).toBe(true);
    expect(
      pickedCount(
        choose({
          llm: { provider: "openai", model: "x" },
          vision: { provider: "openai", model: "y" },
        }),
      ),
    ).toBe(2);
  });

  it("isCompletePick rejects blanks and missing halves", () => {
    expect(isCompletePick(undefined)).toBe(false);
    expect(isCompletePick({ model: "gpt-5" })).toBe(false);
    expect(isCompletePick({ provider: "openai", model: "   " })).toBe(false);
    expect(isCompletePick({ provider: "openai", model: "gpt-5" })).toBe(true);
  });
});

describe("roleSummary", () => {
  it("prints the inherited pair with its dimension", () => {
    const embed = roleSummary("embedding", ladder, undefined);
    expect(embed.chip).toBe("inherited");
    expect(embed.title).toBe("text-embedding-3-small");
    expect(embed.detail).toBe("1536 dims");
    expect(embed.id).toBe("openai/text-embedding-3-small");
    expect(embed.unset).toBe(false);
  });

  it("prints the SPEC-181 §09 one-line form, provider included", () => {
    // "Extract  gpt-5-nano · openai", "Embed  text-embedding-3-small · openai · 1536".
    expect(roleSummary("llm", ladder, undefined).label).toBe("gpt-5-nano · openai");
    expect(roleSummary("embedding", ladder, undefined).label).toBe(
      "text-embedding-3-small · openai · 1536",
    );
  });

  it("prints the unset and follows forms the spec names", () => {
    // "Vision  not set"
    const empty = inheritedRoles({});
    expect(roleSummary("vision", empty, undefined).label).toBe("not set");
    // A vision pair that mirrors Extract says so rather than repeating the id.
    expect(roleSummary("vision", ladder, undefined).label).toBe("follows Extract");
  });

  it("says vision follows Extract instead of repeating the id", () => {
    const vision = roleSummary("vision", ladder, undefined);
    expect(vision.chip).toBe("follows");
    expect(vision.title).toBe("Follows Extract");
    expect(vision.id).toBeNull();
  });

  it("prints a distinct inherited vision pair as its own id", () => {
    const distinct = { ...ladder, vision: { provider: "openai", model: "gpt-5.2" } };
    const vision = roleSummary("vision", distinct, undefined);
    expect(vision.chip).toBe("inherited");
    expect(vision.id).toBe("openai/gpt-5.2");
  });

  it("marks a pick as this workspace's", () => {
    const llm = roleSummary("llm", ladder, {
      provider: "ollama",
      model: "gemma3:12b",
      displayName: "Gemma 3 12B",
    });
    expect(llm.chip).toBe("workspace");
    expect(llm.title).toBe("Gemma 3 12B");
    expect(llm.id).toBe("ollama/gemma3:12b");
  });

  it("reports an empty ladder slot as not set", () => {
    const empty = inheritedRoles({});
    const llm = roleSummary("llm", empty, undefined);
    expect(llm.unset).toBe(true);
    expect(llm.title).toBe("Not set");
    expect(llm.id).toBeNull();
  });
});

describe("refusals", () => {
  const verdicts = {
    llm: "not_eligible",
    embedding: "dimension_missing",
    vision: "ok",
  } as const;

  it("names only refusals, in role order", () => {
    expect(refusedRoles(verdicts)).toEqual(["llm", "embedding"]);
    expect(refusedRoles({ llm: "ok", embedding: "ok", vision: "ok" })).toEqual([]);
    expect(refusedRoles(undefined)).toEqual([]);
  });

  it("gates only a role the operator pinned", () => {
    // Nothing pinned: the body names no pair, so the server judges nothing.
    expect(refusedPickedRoles(verdicts, emptyModelDraft())).toEqual([]);
    // Chooser open, still nothing pinned: same answer.
    expect(refusedPickedRoles(verdicts, { source: "choose" })).toEqual([]);
    // A pinned, refused role blocks.
    expect(
      refusedPickedRoles(verdicts, {
        source: "choose",
        llm: { provider: "ollama", model: "gemma3:12b" },
      }),
    ).toEqual(["llm"]);
    // A refusal on an unpinned role does not block a pinned, healthy one.
    expect(
      refusedPickedRoles(verdicts, {
        source: "choose",
        vision: { provider: "openai", model: "gpt-5.2" },
      }),
    ).toEqual([]);
  });

  it("words each verdict once", () => {
    expect(verdictSentence("not_eligible")).toContain("Not offered");
    expect(verdictSentence("dimension_missing")).toContain("vector size");
    expect(verdictSentence("ok")).toBeNull();
    expect(verdictSentence(undefined)).toBeNull();
  });

  it("names the roles in the footer sentence", () => {
    expect(blockedModelSentence([])).toBeNull();
    expect(blockedModelSentence(["embedding"])).toBe(
      "Choose another model for Embed to continue.",
    );
    expect(blockedModelSentence(["llm", "embedding"])).toBe(
      "Choose another model for Extract, Embed to continue.",
    );
  });
});
