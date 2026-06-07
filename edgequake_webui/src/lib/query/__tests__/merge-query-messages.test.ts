import { describe, expect, it } from "vitest";
import { mergeQueryMessages } from "../merge-query-messages";
import type { QueryMessage } from "../query-interface-types";

const user = (content: string, id = "u1"): QueryMessage => ({
  id,
  role: "user",
  content,
});

const assistant = (content: string, id = "a1"): QueryMessage => ({
  id,
  role: "assistant",
  content,
});

describe("mergeQueryMessages (UI-P3-005)", () => {
  it("returns server messages when no optimistic or pending", () => {
    const server = [user("hi"), assistant("hello")];
    expect(mergeQueryMessages(server, null, null)).toEqual(server);
  });

  it("appends optimistic user message when not yet on server", () => {
    const server = [assistant("hello")];
    const optimistic = user("new question", "optimistic-user-1");
    expect(mergeQueryMessages(server, optimistic, null)).toEqual([
      ...server,
      optimistic,
    ]);
  });

  it("skips optimistic user when server already has same content", () => {
    const server = [user("same"), assistant("hello")];
    const optimistic = user("same", "optimistic-user-1");
    expect(mergeQueryMessages(server, optimistic, null)).toEqual(server);
  });

  it("appends pending assistant when content differs from last server message", () => {
    const server = [user("q"), assistant("partial")];
    const pending = assistant("partial stream...", "pending-1");
    pending.isStreaming = true;
    expect(mergeQueryMessages(server, null, pending)).toEqual([
      ...server,
      pending,
    ]);
  });

  it("skips pending when last server assistant matches content", () => {
    const server = [user("q"), assistant("done")];
    const pending = assistant("done", "pending-1");
    expect(mergeQueryMessages(server, null, pending)).toEqual(server);
  });
});
