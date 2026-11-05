import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { MessageError } from "../message-error";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (_key: string, fallback: string) => fallback }),
}));

afterEach(cleanup);

describe("query error attribution", () => {
  it.each([
    "Storage error: Storage deadline exceeded: canceling statement due to statement timeout",
    "Storage operation timed out.",
    "STORAGE_DEADLINE_EXCEEDED",
    "Database timeout",
  ])("attributes %s to retrieval", (message) => {
    render(<MessageError message={message} />);
    expect(screen.getByRole("alert").textContent).toContain(
      "Searching the knowledge base took too long.",
    );
    expect(screen.getByRole("alert").textContent).not.toContain("model");
  });

  it.each(["LLM error: Request timeout", "Query timed out after 600000ms"])(
    "keeps model timeout guidance for %s",
    (message) => {
      render(<MessageError message={message} />);
      expect(screen.getByRole("alert").textContent).toContain(
        "The model took too long to respond.",
      );
    },
  );

  /*
   * SPEC-185 — the reported failure. A 403 PERMISSION_DENIED body is long, so the length guard
   * used to replace it with "Something went wrong", which hid a fixable credential problem.
   */
  it("names a provider permission failure instead of hiding it", () => {
    const raw =
      'LLM error: API error: Stream error: {"error":{"code":403,"message":"Permission ' +
      "&#39;aiplatform.endpoints.predict&#39; denied on resource " +
      '\'//aiplatform.googleapis.com/projects/mobioos-506015/locations/global/publishers/google/models/gemini-3.8-flash\'","status":"PERMISSION_DENIED"}}';
    render(<MessageError message={raw} />);

    const text = screen.getByRole("alert").textContent ?? "";
    expect(text).toContain("refused the request");
    expect(text).not.toContain("Something went wrong");
  });

  it("names an expired credential instead of hiding it", () => {
    render(<MessageError message="LLM error: 401 UNAUTHENTICATED: token expired" />);

    const text = screen.getByRole("alert").textContent ?? "";
    expect(text).toContain("rejected the credentials");
    expect(text).not.toContain("Something went wrong");
  });

  it("still refuses to dump a stack trace", () => {
    const stack = `LLM error: boom\n    at Object.<anonymous> (${"x".repeat(300)})`;
    render(<MessageError message={stack} />);

    expect(screen.getByRole("alert").textContent).toContain("Something went wrong");
  });
});
