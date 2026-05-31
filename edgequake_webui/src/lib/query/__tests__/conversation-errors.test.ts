import { ApiRequestError } from "@/lib/api/client";
import { describe, expect, it } from "vitest";
import {
  isConversationNotFoundError,
  isServerPersistedMessageId,
} from "../conversation-errors";

describe("conversation-errors (UI-P3-005)", () => {
  it("detects ApiRequestError 404 as conversation not found", () => {
    expect(
      isConversationNotFoundError(new ApiRequestError("Not found", 404)),
    ).toBe(true);
  });

  it("detects message-based conversation not found", () => {
    expect(
      isConversationNotFoundError(new Error("Conversation not found: abc")),
    ).toBe(true);
  });

  it("rejects unrelated errors", () => {
    expect(isConversationNotFoundError(new Error("Network timeout"))).toBe(
      false,
    );
  });

  it("distinguishes optimistic from server message ids", () => {
    expect(isServerPersistedMessageId("optimistic-user-123")).toBe(false);
    expect(isServerPersistedMessageId("f6fa9cad-bbff-4892-a855-3bd7d70da044")).toBe(
      true,
    );
  });
});
