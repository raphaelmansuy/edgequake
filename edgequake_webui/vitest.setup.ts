import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterEach } from "vitest";

// WHY: Testing Library does not auto-clean when `globals` is used without its own
// auto-cleanup entry point. Without this, every `render` in a file stacked another copy of
// the same `data-testid` in the document, and `getByTestId` failed with "Found multiple
// elements" — a test-infrastructure bug that looks like a component bug.
afterEach(() => {
  cleanup();
});
