import { describe, expect, it } from "vitest";
import { classifyCreateFailure } from "../use-create-scope";

/**
 * `classifyCreateFailure` decides which sentence the dialog shows and which control it points at,
 * so it has to read the server's reason code, not guess from prose.
 *
 * The bug this pins: the classifier searched the message for `models_not_eligible` (plural), while
 * every create route returns `code: "model_not_eligible"` (singular) in the body and a sentence
 * that never contains the code at all. A refused model therefore fell through to the generic
 * "could not create" copy.
 */

/** The shape `ApiRequestError` has at runtime: a message plus the body's `code`. */
class FakeApiError extends Error {
  constructor(
    message: string,
    readonly status: number,
    readonly code?: string,
  ) {
    super(message);
    this.name = "ApiRequestError";
  }
}

describe("classifyCreateFailure", () => {
  it("reads the structured reason code before the prose", () => {
    // The real body: a human sentence plus a code. The sentence contains no code at all.
    expect(
      classifyCreateFailure(
        new FakeApiError("model is not selectable for this workspace", 422, "model_not_eligible"),
      ).kind,
    ).toBe("models");
  });

  it.each([
    ["model_not_eligible", "models"],
    ["capability_mismatch", "models"],
    ["model_pair_incomplete", "models"],
    ["embedding_dimension_mismatch", "models"],
    ["embedding_not_multimodal", "models"],
    ["usage_locked", "models"],
    ["workspace_quota_reached", "quota"],
    ["tenant_slug_taken", "slug"],
    ["workspace_slug_taken", "slug"],
    ["capability_required", "capability"],
    ["scope_inactive", "inactive"],
  ] as const)("maps code %s to %s", (code, kind) => {
    expect(classifyCreateFailure(new FakeApiError("boom", 400, code)).kind).toBe(kind);
  });

  it("still classifies when only the message survives", () => {
    // A transport that folds the code into the message, and the legacy plural spelling.
    expect(classifyCreateFailure(new Error("models_not_eligible: nope")).kind).toBe("models");
    expect(classifyCreateFailure(new Error("model_not_eligible: nope")).kind).toBe("models");
    expect(classifyCreateFailure(new Error("usage_locked")).kind).toBe("models");
    expect(classifyCreateFailure(new Error("workspace_quota_reached")).kind).toBe("quota");
  });

  it("does not mistake an unrelated word for a model refusal", () => {
    // The old regex matched any message containing "embedding", which is everywhere.
    expect(classifyCreateFailure(new Error("embedding rebuild in progress")).kind).toBe("server");
    expect(classifyCreateFailure(new Error("boom")).kind).toBe("server");
  });

  it("keeps the message for the caller", () => {
    const result = classifyCreateFailure(new FakeApiError("model is not selectable", 422, "model_not_eligible"));
    expect(result.message).toBe("model is not selectable");
  });
});
