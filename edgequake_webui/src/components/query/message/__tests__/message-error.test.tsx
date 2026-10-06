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
});
